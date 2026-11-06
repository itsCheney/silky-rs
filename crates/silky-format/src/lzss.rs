//! The LZSS variant used by Silky's archives and AKB images.
//!
//! Classic Okumura layout: 4096-byte window initialised to zero, writing
//! starts at 4096 - 18, flag bytes are read LSB first, a set bit is a
//! literal, a clear bit is a 12-bit window position plus a 4-bit length
//! (length + 3 bytes).

const WINDOW: usize = 4096;
const MAX_MATCH: usize = 18;
const MIN_MATCH: usize = 3;

/// Decompresses `src` until `expected` bytes were produced or the input ends.
/// The result is shorter than `expected` only if the input is truncated.
pub fn decompress(src: &[u8], expected: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(expected);
    let mut window = [0u8; WINDOW];
    let mut pos = WINDOW - MAX_MATCH;
    let mut i = 0;
    let mut flags: u32 = 0;
    while out.len() < expected {
        flags >>= 1;
        if flags & 0x100 == 0 {
            let Some(&b) = src.get(i) else { break };
            i += 1;
            flags = u32::from(b) | 0xff00;
        }
        if flags & 1 != 0 {
            let Some(&b) = src.get(i) else { break };
            i += 1;
            out.push(b);
            window[pos] = b;
            pos = (pos + 1) & (WINDOW - 1);
        } else {
            let (Some(&lo), Some(&hi)) = (src.get(i), src.get(i + 1)) else {
                break;
            };
            i += 2;
            let start = usize::from(lo) | (usize::from(hi & 0xf0) << 4);
            let len = usize::from(hi & 0x0f) + MIN_MATCH;
            for k in 0..len {
                if out.len() == expected {
                    break;
                }
                let b = window[(start + k) & (WINDOW - 1)];
                out.push(b);
                window[pos] = b;
                pos = (pos + 1) & (WINDOW - 1);
            }
        }
    }
    out
}

/// Encodes `data` as literals only. Valid input for [`decompress`]; used to
/// build test fixtures without shipping game data.
pub fn store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 8 + 1);
    for chunk in data.chunks(8) {
        out.push(0xff);
        out.extend_from_slice(chunk);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_round_trip() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        assert_eq!(decompress(&store(&data), data.len()), data);
    }

    #[test]
    fn back_reference_into_zeroed_window() {
        // Flag 0b10: one literal 'A', then a match of 3 bytes at window
        // position 4078 (where 'A' was written), giving "AAAA".
        let pos = (WINDOW - MAX_MATCH) as u16;
        let src = [
            0b0000_0001,
            b'A',
            (pos & 0xff) as u8,
            ((pos >> 4) & 0xf0) as u8,
        ];
        assert_eq!(decompress(&src, 4), b"AAAA");
        // Reading before anything was written gives the zero fill.
        let src = [0b0000_0000, 0x00, 0x00];
        assert_eq!(decompress(&src, 3), [0, 0, 0]);
    }

    #[test]
    fn truncated_input_stops() {
        assert_eq!(decompress(&[0xff, 1, 2], 10), [1, 2]);
    }
}
