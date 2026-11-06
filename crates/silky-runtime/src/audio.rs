//! Sound: a software mixer for music, sound effects and voice.
//!
//! The runtime starts and stops tracks; a frontend pulls mixed samples with
//! [`Mixer::fill`] from its audio callback. Without a frontend pulling,
//! nothing is decoded (see [`Mixer::disabled`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::{Error, Result};

/// The three sound classes of the engine, each with numbered channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bus {
    Music,
    Effect,
    Voice,
}

/// Decoded sound, as interleaved stereo f32.
#[derive(Debug)]
pub struct Pcm {
    pub rate: u32,
    pub frames: Vec<[f32; 2]>,
    /// Loop start and end in frames (end exclusive).
    pub loop_frames: Option<(usize, usize)>,
}

impl Pcm {
    pub fn from_wav(data: &[u8]) -> Result<Self> {
        let w = silky_format::wav::parse(data)?;
        let ch = w.channels as usize;
        let frames = w
            .samples
            .chunks_exact(ch)
            .map(|f| {
                let l = f[0] as f32 / 32768.0;
                let r = f.get(1).map_or(l, |&s| s as f32 / 32768.0);
                [l, r]
            })
            .collect::<Vec<_>>();
        let n = frames.len();
        let loop_frames = w
            .loop_frames
            .map(|(s, e)| (s as usize, (e as usize).min(n)))
            .filter(|(s, e)| s < e);
        Ok(Self {
            rate: w.sample_rate,
            frames,
            loop_frames,
        })
    }

    pub fn from_ogg(data: &[u8]) -> Result<Self> {
        let err = |e: lewton::VorbisError| Error::Invalid(format!("Ogg Vorbis: {e}"));
        let mut r =
            lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(data)).map_err(err)?;
        let ch = r.ident_hdr.audio_channels as usize;
        let rate = r.ident_hdr.audio_sample_rate;
        let mut frames = Vec::new();
        while let Some(packet) = r.read_dec_packet_itl().map_err(err)? {
            frames.extend(packet.chunks_exact(ch.max(1)).map(|f| {
                let l = f[0] as f32 / 32768.0;
                let r = f.get(1).map_or(l, |&s| s as f32 / 32768.0);
                [l, r]
            }));
        }
        Ok(Self {
            rate,
            frames,
            loop_frames: None,
        })
    }
}

#[derive(Debug)]
struct Track {
    pcm: Arc<Pcm>,
    /// Position in source frames.
    pos: f64,
    looping: bool,
    gain: f32,
    /// Gain change per output frame while fading out; the track ends at 0.
    fade: Option<f32>,
}

#[derive(Debug, Default)]
struct State {
    tracks: HashMap<(Bus, i32), Track>,
    /// Channels waiting for a sound decoded in the background, with the
    /// token of that request.
    pending: HashMap<(Bus, i32), u64>,
    next_token: u64,
}

/// Shared handle: the runtime controls it, the audio callback reads it.
#[derive(Debug, Clone)]
pub struct Mixer {
    state: Option<Arc<Mutex<State>>>,
    /// Output rate, for fade lengths.
    rate: u32,
}

impl Mixer {
    /// A mixer for a frontend that plays sound at `rate` Hz.
    pub fn new(rate: u32) -> Self {
        Self {
            state: Some(Arc::default()),
            rate,
        }
    }

    /// A mixer that ignores everything (headless runs).
    pub fn disabled() -> Self {
        Self {
            state: None,
            rate: 44100,
        }
    }

    pub fn enabled(&self) -> bool {
        self.state.is_some()
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> Option<R> {
        let state = self.state.as_ref()?;
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        Some(f(&mut guard))
    }

    /// Starts a track on a channel, replacing what played there.
    pub fn play(&self, bus: Bus, channel: i32, pcm: Arc<Pcm>, looping: bool) {
        self.with(|s| {
            s.pending.remove(&(bus, channel));
            s.tracks.insert(
                (bus, channel),
                Track {
                    pcm,
                    pos: 0.0,
                    looping,
                    gain: 1.0,
                    fade: None,
                },
            );
        });
    }

    /// Starts a track once `decode` has run on another thread, unless the
    /// channel is stopped or given another sound first. Keeps clicks
    /// responsive while long voices decode.
    pub fn play_later(
        &self,
        bus: Bus,
        channel: i32,
        looping: bool,
        decode: impl FnOnce() -> Result<Arc<Pcm>> + Send + 'static,
    ) {
        let Some(token) = self.with(|s| {
            s.tracks.remove(&(bus, channel));
            s.next_token += 1;
            s.pending.insert((bus, channel), s.next_token);
            s.next_token
        }) else {
            return;
        };
        let mixer = self.clone();
        std::thread::spawn(move || match decode() {
            Ok(pcm) => {
                mixer.with(|s| {
                    if s.pending.get(&(bus, channel)) == Some(&token) {
                        s.pending.remove(&(bus, channel));
                        s.tracks.insert(
                            (bus, channel),
                            Track {
                                pcm,
                                pos: 0.0,
                                looping,
                                gain: 1.0,
                                fade: None,
                            },
                        );
                    }
                });
            }
            Err(e) => eprintln!("sound: {e}"),
        });
    }

    pub fn stop(&self, bus: Bus, channel: i32) {
        self.with(|s| {
            s.pending.remove(&(bus, channel));
            s.tracks.remove(&(bus, channel))
        });
    }

    /// Stops every channel of a bus.
    pub fn stop_bus(&self, bus: Bus) {
        self.with(|s| {
            s.pending.retain(|k, _| k.0 != bus);
            s.tracks.retain(|k, _| k.0 != bus)
        });
    }

    /// Fades a channel out over `ms` milliseconds, then stops it.
    pub fn fade_out(&self, bus: Bus, channel: i32, ms: u32) {
        let frames = (self.rate as u64 * ms as u64 / 1000).max(1) as f32;
        self.with(|s| {
            if let Some(t) = s.tracks.get_mut(&(bus, channel)) {
                t.fade = Some(t.gain / frames);
            }
        });
    }

    /// Whether a channel plays, or waits for its sound to be decoded.
    pub fn is_playing(&self, bus: Bus, channel: i32) -> bool {
        self.with(|s| {
            s.tracks.contains_key(&(bus, channel)) || s.pending.contains_key(&(bus, channel))
        })
        .unwrap_or(false)
    }

    /// Mixes into `out` (interleaved, `channels` per frame, at the rate
    /// given to [`Mixer::new`]), overwriting it.
    pub fn fill(&self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        let rate = self.rate as f64;
        self.with(|s| {
            s.tracks.retain(|_, t| {
                let step = t.pcm.rate as f64 / rate;
                let n = t.pcm.frames.len();
                for frame in out.chunks_exact_mut(channels.max(1)) {
                    let i = t.pos as usize;
                    if i >= n {
                        return false;
                    }
                    let j = (i + 1).min(n - 1);
                    let k = (t.pos - i as f64) as f32;
                    let (a, b) = (t.pcm.frames[i], t.pcm.frames[j]);
                    let l = (a[0] + (b[0] - a[0]) * k) * t.gain;
                    let r = (a[1] + (b[1] - a[1]) * k) * t.gain;
                    match frame {
                        [m] => *m += (l + r) * 0.5,
                        [fl, fr, ..] => {
                            *fl += l;
                            *fr += r;
                        }
                        [] => {}
                    }
                    if let Some(d) = t.fade {
                        t.gain -= d;
                        if t.gain <= 0.0 {
                            return false;
                        }
                    }
                    t.pos += step;
                    match (t.looping, t.pcm.loop_frames) {
                        (true, Some((start, end))) if t.pos >= end as f64 => {
                            t.pos -= (end - start) as f64;
                        }
                        (true, None) if t.pos >= n as f64 => t.pos -= n as f64,
                        _ => {}
                    }
                }
                true
            });
        });
        for v in out.iter_mut() {
            *v = v.clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(frames: usize, loop_frames: Option<(usize, usize)>) -> Arc<Pcm> {
        Arc::new(Pcm {
            rate: 100,
            frames: (0..frames).map(|i| [i as f32 / 100.0, 0.0]).collect(),
            loop_frames,
        })
    }

    #[test]
    fn plays_once_and_ends() {
        let m = Mixer::new(100);
        m.play(Bus::Effect, 0, pcm(3, None), false);
        let mut out = [0.0f32; 8];
        m.fill(&mut out, 2);
        assert_eq!(out[..6], [0.0, 0.0, 0.01, 0.0, 0.02, 0.0]);
        assert!(!m.is_playing(Bus::Effect, 0));
    }

    #[test]
    fn loops_between_loop_points() {
        let m = Mixer::new(100);
        m.play(Bus::Music, 0, pcm(10, Some((2, 4))), true);
        let mut out = [0.0f32; 6];
        m.fill(&mut out, 1);
        let got: Vec<i32> = out.iter().map(|v| (v * 200.0).round() as i32).collect();
        // Mono output is the mean of both sides.
        assert_eq!(got, [0, 1, 2, 3, 2, 3]);
    }

    #[test]
    fn fade_out_stops() {
        let m = Mixer::new(1000);
        m.play(Bus::Voice, 1, pcm(100, None), false);
        m.fade_out(Bus::Voice, 1, 10);
        let mut out = [0.0f32; 40];
        m.fill(&mut out, 2);
        assert!(!m.is_playing(Bus::Voice, 1));
    }
}
