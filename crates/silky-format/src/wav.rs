//! RIFF WAVE files as used for music and sound effects: 16-bit PCM, with
//! an optional `smpl` chunk giving the loop of background music.

use crate::error::{invalid, Result};

const FORMAT: &str = "WAV";

#[derive(Debug, Clone)]
pub struct Wav {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved samples.
    pub samples: Vec<i16>,
    /// Loop start and end in frames (end exclusive), from the `smpl` chunk.
    pub loop_frames: Option<(u64, u64)>,
}

pub fn parse(data: &[u8]) -> Result<Wav> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return invalid(FORMAT, "not a RIFF WAVE file");
    }
    let u16_at = |o: usize| u16::from_le_bytes([data[o], data[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
    let mut fmt = None;
    let mut pcm: Option<&[u8]> = None;
    let mut loop_frames = None;
    let mut o = 12;
    while o + 8 <= data.len() {
        let id = &data[o..o + 4];
        let size = u32_at(o + 4) as usize;
        let body = &data[o + 8..data.len().min(o + 8 + size)];
        match id {
            b"fmt " if body.len() >= 16 => {
                fmt = Some((
                    u16_at(o + 8),
                    u16_at(o + 10),
                    u32_at(o + 12),
                    u16_at(o + 22),
                ));
            }
            b"data" => pcm = Some(body),
            // smpl: 9 header words, then loops of 6 words: id, type,
            // start, end (inclusive), fraction, play count.
            b"smpl" if body.len() >= 36 + 24 && u32_at(o + 8 + 28) > 0 => {
                let l = o + 8 + 36;
                let (start, end) = (u32_at(l + 8) as u64, u32_at(l + 12) as u64);
                if end > start {
                    loop_frames = Some((start, end + 1));
                }
            }
            _ => {}
        }
        o += 8 + size + (size & 1);
    }
    let Some((format, channels, sample_rate, bits)) = fmt else {
        return invalid(FORMAT, "no fmt chunk");
    };
    if format != 1 || bits != 16 || channels == 0 {
        return invalid(
            FORMAT,
            format!("format {format}, {bits} bits, {channels} channels: only 16-bit PCM is read"),
        );
    }
    let Some(pcm) = pcm else {
        return invalid(FORMAT, "no data chunk");
    };
    let samples = pcm
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    Ok(Wav {
        sample_rate,
        channels,
        samples,
        loop_frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut c = id.to_vec();
        c.extend((body.len() as u32).to_le_bytes());
        c.extend(body);
        c
    }

    #[test]
    fn pcm_with_loop() {
        let mut fmt = Vec::new();
        for v in [1u16, 2] {
            fmt.extend(v.to_le_bytes());
        }
        fmt.extend(44100u32.to_le_bytes());
        fmt.extend((44100u32 * 4).to_le_bytes());
        for v in [4u16, 16] {
            fmt.extend(v.to_le_bytes());
        }
        let mut smpl = vec![0u8; 28];
        smpl.extend(1u32.to_le_bytes());
        smpl.extend(0u32.to_le_bytes());
        for v in [0u32, 0, 1, 2, 0, 0] {
            smpl.extend(v.to_le_bytes());
        }
        let pcm: Vec<u8> = [1i16, -1, 2, -2, 3, -3]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let mut body = b"WAVE".to_vec();
        body.extend(chunk(b"fmt ", &fmt));
        body.extend(chunk(b"data", &pcm));
        body.extend(chunk(b"smpl", &smpl));
        let file = chunk(b"RIFF", &body);
        let w = parse(&file).unwrap();
        assert_eq!((w.sample_rate, w.channels), (44100, 2));
        assert_eq!(w.samples, [1, -1, 2, -2, 3, -3]);
        assert_eq!(w.loop_frames, Some((1, 3)));
    }
}
