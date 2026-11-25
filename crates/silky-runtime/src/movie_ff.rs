//! ASF movies (VC-1 pictures, WMA Pro sound) decoded with a trimmed FFmpeg
//! (`ffmpeg` feature; `tools/ffmpeg/build.sh`). The shim in
//! `ffmpeg/shim.c` reads the file from memory; one handle decodes one stream.

use std::ffi::c_void;
use std::sync::Arc;

use crate::audio::Pcm;
use crate::{Error, Result};

type Ff = c_void;

extern "C" {
    fn silky_ff_open(bytes: *const u8, length: usize, video: i32) -> *mut Ff;
    fn silky_ff_close(f: *mut Ff);
    fn silky_ff_width(f: *mut Ff) -> i32;
    fn silky_ff_height(f: *mut Ff) -> i32;
    fn silky_ff_framerate(f: *mut Ff) -> f64;
    fn silky_ff_samplerate(f: *mut Ff) -> i32;
    fn silky_ff_next_frame(f: *mut Ff, rgba: *mut u8, time: *mut f64) -> i32;
    fn silky_ff_next_audio(f: *mut Ff, out: *mut *const f32) -> i32;
    #[cfg(test)]
    fn silky_ff_yuv420_to_rgba(
        y: *const u8,
        ys: i32,
        cb: *const u8,
        cbs: i32,
        cr: *const u8,
        crs: i32,
        width: i32,
        height: i32,
        rgba: *mut u8,
    );
}

/// One stream of an ASF file.
pub(crate) struct Stream {
    ff: *mut Ff,
    /// Keeps the file alive as long as the decoder reads it.
    _data: Arc<Vec<u8>>,
}

// The decoder is used from one thread at a time.
unsafe impl Send for Stream {}

impl Stream {
    fn open(data: Arc<Vec<u8>>, video: bool) -> Result<Self> {
        let ff = unsafe { silky_ff_open(data.as_ptr(), data.len(), video as i32) };
        if ff.is_null() {
            return Err(Error::Invalid(format!(
                "movie: no {} stream FFmpeg can decode",
                if video { "picture" } else { "sound" }
            )));
        }
        Ok(Self { ff, _data: data })
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe { silky_ff_close(self.ff) };
    }
}

/// The pictures of an ASF movie.
pub(crate) struct Video {
    s: Stream,
    pub width: i32,
    pub height: i32,
    pub fps: f64,
}

impl Video {
    pub fn open(data: Arc<Vec<u8>>) -> Result<Self> {
        let s = Stream::open(data, true)?;
        let (width, height, fps) = unsafe {
            (
                silky_ff_width(s.ff),
                silky_ff_height(s.ff),
                silky_ff_framerate(s.ff),
            )
        };
        Ok(Self {
            s,
            width,
            height,
            fps,
        })
    }

    /// Decodes the next picture into `rgba` (opaque); gives its time in
    /// seconds. `None` at the end.
    pub fn next_frame(&mut self, rgba: &mut [u8]) -> Option<f64> {
        assert!(rgba.len() >= (self.width * self.height * 4) as usize);
        let mut t = 0.0;
        let ok = unsafe { silky_ff_next_frame(self.s.ff, rgba.as_mut_ptr(), &mut t) };
        (ok != 0).then_some(t)
    }
}

/// Decodes the whole sound track.
pub(crate) fn decode_audio(data: Arc<Vec<u8>>) -> Result<Pcm> {
    let s = Stream::open(data, false)?;
    let rate = unsafe { silky_ff_samplerate(s.ff) }.max(1) as u32;
    let mut frames = Vec::new();
    loop {
        let mut p: *const f32 = std::ptr::null();
        let n = unsafe { silky_ff_next_audio(s.ff, &mut p) };
        if n <= 0 || p.is_null() {
            break;
        }
        let block = unsafe { std::slice::from_raw_parts(p, n as usize * 2) };
        frames.extend(block.chunks_exact(2).map(|c| [c[0], c[1]]));
    }
    Ok(Pcm {
        rate,
        frames,
        loop_frames: None,
    })
}

/// The shim's YUV 4:2:0 to opaque RGBA conversion (BT.601 limited range,
/// the integers of the MPEG-1 decoder), for the test.
#[cfg(test)]
fn yuv420_to_rgba(
    y: &[u8],
    y_stride: usize,
    cb: &[u8],
    cb_stride: usize,
    cr: &[u8],
    cr_stride: usize,
    width: usize,
    height: usize,
) -> Vec<u8> {
    assert!(y.len() >= y_stride * (height - 1) + width);
    let ch = height.div_ceil(2);
    let cw = width.div_ceil(2);
    assert!(cb.len() >= cb_stride * (ch - 1) + cw && cr.len() >= cr_stride * (ch - 1) + cw);
    let mut rgba = vec![0u8; width * height * 4];
    unsafe {
        silky_ff_yuv420_to_rgba(
            y.as_ptr(),
            y_stride as i32,
            cb.as_ptr(),
            cb_stride as i32,
            cr.as_ptr(),
            cr_stride as i32,
            width as i32,
            height as i32,
            rgba.as_mut_ptr(),
        );
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yuv_to_rgba_is_bt601_limited() {
        // Black, white, and a mid grey; chroma neutral (128).
        let y = [16u8, 235, 126, 126];
        let c = [128u8];
        let rgba = yuv420_to_rgba(&y, 2, &c, 1, &c, 1, 2, 2);
        assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
        // (235 - 16) * 76309 >> 16 = 254, as pl_mpeg rounds it.
        assert_eq!(&rgba[4..8], &[254, 254, 254, 255]);
        // (126 - 16) * 76309 >> 16 = 128
        assert_eq!(&rgba[8..12], &[128, 128, 128, 255]);
        // Pure red in limited range: Y 81, Cb 90, Cr 240.
        let rgba = yuv420_to_rgba(&[81; 4], 2, &[90], 1, &[240], 1, 2, 2);
        assert!(
            rgba[0] >= 250 && rgba[1] <= 6 && rgba[2] <= 6,
            "{:?}",
            &rgba[..4]
        );
    }

    /// Decodes the first pictures and some sound of a game's movie.
    /// Ignored by default: it needs the game's file.
    #[test]
    #[ignore]
    fn decodes_the_first_frames_of_a_game_movie() {
        let path = std::env::var("SILKY_TEST_MOVIE")
            .unwrap_or_else(|_| "/home/user/senpai/wine/do-jin4.dat".into());
        let data = Arc::new(std::fs::read(path).expect("movie file"));
        assert!(crate::movie::is_asf(&data));
        let mut v = Video::open(data.clone()).expect("picture stream");
        assert_eq!((v.width, v.height), (1280, 720));
        let mut rgba = vec![0u8; (v.width * v.height * 4) as usize];
        let mut last = -1.0;
        for _ in 0..5 {
            let t = v.next_frame(&mut rgba).expect("frame");
            assert!(t > last);
            last = t;
        }
        assert!(rgba.chunks_exact(4).all(|p| p[3] == 255));
        assert!(rgba.iter().any(|&b| b != 0));
        let pcm = decode_audio(data.clone()).expect("sound");
        assert_eq!(pcm.rate, 44100);
        assert!(pcm.frames.len() > 44100 / 2);

        // The movie player reaches the same decoder; with
        // SILKY_TEST_MOVIE_PNG it keeps the picture 300 ms in.
        let mut m = crate::movie::Movie::open(data).expect("movie");
        m.seek_forward(300.0);
        assert!(!m.ended);
        if let Ok(out) = std::env::var("SILKY_TEST_MOVIE_PNG") {
            let file = std::fs::File::create(out).unwrap();
            let mut enc = png::Encoder::new(file, m.width, m.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header()
                .unwrap()
                .write_image_data(&m.rgba)
                .unwrap();
        }
    }
}
