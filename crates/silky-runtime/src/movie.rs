//! Movies (`.vsd`: an MPEG-1 program stream with MP2 audio), decoded with
//! pl_mpeg (MIT, vendored in `vendor/pl_mpeg`). ASF files (VC-1 and WMA
//! Pro) go to FFmpeg instead when the `ffmpeg` feature is on
//! (`movie_ff.rs`).

use std::ffi::c_void;
use std::sync::Arc;

use crate::audio::Pcm;
use crate::{Error, Result};

#[allow(non_camel_case_types)]
type plm_t = c_void;

extern "C" {
    fn silky_plm_open(bytes: *mut u8, length: usize, audio: i32) -> *mut plm_t;
    fn silky_plm_close(p: *mut plm_t);
    fn silky_plm_width(p: *mut plm_t) -> i32;
    fn silky_plm_height(p: *mut plm_t) -> i32;
    fn silky_plm_framerate(p: *mut plm_t) -> f64;
    fn silky_plm_samplerate(p: *mut plm_t) -> i32;
    fn silky_plm_next_frame(p: *mut plm_t, rgba: *mut u8, time: *mut f64) -> i32;
    fn silky_plm_next_audio(p: *mut plm_t, out: *mut f32) -> i32;
}

/// One pl_mpeg decoder over a stream it does not own.
struct Decoder {
    plm: *mut plm_t,
    /// Keeps the stream alive as long as the decoder.
    _data: Arc<Vec<u8>>,
}

// The decoder is used from one thread at a time.
unsafe impl Send for Decoder {}

impl Decoder {
    fn open(data: Arc<Vec<u8>>, audio: bool) -> Result<Self> {
        // pl_mpeg only reads the buffer, it takes a mutable pointer anyway.
        let plm = unsafe { silky_plm_open(data.as_ptr() as *mut u8, data.len(), audio as i32) };
        if plm.is_null() {
            return Err(Error::Invalid("movie: not an MPEG-1 stream".into()));
        }
        Ok(Self { plm, _data: data })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { silky_plm_close(self.plm) };
    }
}

/// Whether the bytes start with the ASF header GUID.
pub fn is_asf(data: &[u8]) -> bool {
    data.starts_with(&[0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11])
}

#[cfg(not(feature = "ffmpeg"))]
fn no_ffmpeg() -> Error {
    Error::Invalid("movie: ASF files need the `ffmpeg` feature (tools/ffmpeg/build.sh)".into())
}

/// The stream of a movie file as the decoders read it: an ASF file as it
/// is, else the MPEG-1 stream behind the `VSD1` header.
pub fn movie_stream(file: Vec<u8>) -> Result<Vec<u8>> {
    if is_asf(&file) {
        Ok(file)
    } else {
        mpeg_stream(&file)
    }
}

/// Strips the `VSD1` header of a movie file.
pub fn mpeg_stream(file: &[u8]) -> Result<Vec<u8>> {
    let range = silky_format::vsd::stream_range(file, file.len() as u64)?;
    Ok(file[range.start as usize..].to_vec())
}

/// Decodes the whole sound track of a stream.
pub fn decode_audio(data: Arc<Vec<u8>>) -> Result<Pcm> {
    if is_asf(&data) {
        #[cfg(feature = "ffmpeg")]
        return crate::movie_ff::decode_audio(data);
        #[cfg(not(feature = "ffmpeg"))]
        return Err(no_ffmpeg());
    }
    let d = Decoder::open(data, true)?;
    let rate = unsafe { silky_plm_samplerate(d.plm) }.max(1) as u32;
    let mut buf = vec![0f32; 1152 * 2];
    let mut frames = Vec::new();
    loop {
        let n = unsafe { silky_plm_next_audio(d.plm, buf.as_mut_ptr()) };
        if n <= 0 {
            break;
        }
        frames.extend(buf[..n as usize * 2].chunks_exact(2).map(|s| [s[0], s[1]]));
    }
    Ok(Pcm {
        rate,
        frames,
        loop_frames: None,
    })
}

/// The picture side of a movie, decoded as time goes on.
pub struct Movie {
    video: Source,
    pub width: u32,
    pub height: u32,
    frame_ms: f64,
    /// Current picture, RGBA.
    pub rgba: Vec<u8>,
    /// Time of the current picture.
    shown_ms: f64,
    pub ended: bool,
}

/// Where the pictures come from.
enum Source {
    Mpeg(Decoder),
    #[cfg(feature = "ffmpeg")]
    Asf(crate::movie_ff::Video),
}

impl Movie {
    pub fn open(data: Arc<Vec<u8>>) -> Result<Self> {
        let (video, w, h, fps) = if is_asf(&data) {
            #[cfg(feature = "ffmpeg")]
            {
                let v = crate::movie_ff::Video::open(data)?;
                let (w, h, fps) = (v.width, v.height, v.fps);
                (Source::Asf(v), w, h, fps)
            }
            #[cfg(not(feature = "ffmpeg"))]
            return Err(no_ffmpeg());
        } else {
            let d = Decoder::open(data, false)?;
            let (w, h, fps) = unsafe {
                (
                    silky_plm_width(d.plm),
                    silky_plm_height(d.plm),
                    silky_plm_framerate(d.plm),
                )
            };
            (Source::Mpeg(d), w, h, fps)
        };
        if w <= 0 || h <= 0 {
            return Err(Error::Invalid("movie without pictures".into()));
        }
        let mut m = Self {
            video,
            width: w as u32,
            height: h as u32,
            frame_ms: 1000.0 / if fps > 0.0 { fps } else { 30.0 },
            rgba: vec![0; (w * h * 4) as usize],
            shown_ms: -1.0,
            ended: false,
        };
        m.next_frame();
        Ok(m)
    }

    fn next_frame(&mut self) -> bool {
        let mut t = 0.0;
        let ok = match &mut self.video {
            Source::Mpeg(d) => unsafe {
                silky_plm_next_frame(d.plm, self.rgba.as_mut_ptr(), &mut t) != 0
            },
            #[cfg(feature = "ffmpeg")]
            Source::Asf(v) => match v.next_frame(&mut self.rgba) {
                Some(time) => {
                    t = time;
                    true
                }
                None => false,
            },
        };
        if !ok {
            self.ended = true;
            return false;
        }
        // pl_mpeg leaves alpha alone.
        for px in self.rgba.chunks_exact_mut(4) {
            px[3] = 255;
        }
        self.shown_ms = t * 1000.0;
        true
    }

    /// Moves to the picture for `ms` into the movie. Returns whether the
    /// picture changed.
    pub fn seek_forward(&mut self, ms: f64) -> bool {
        let mut changed = false;
        while !self.ended && self.shown_ms + self.frame_ms <= ms {
            changed |= self.next_frame();
        }
        changed
    }
}
