//! `.vo` files of 景の海のアペイリア: a picture that changes a set time
//! after a voice line starts (`Game.1007(file)`), read from the
//! executable and the files. The layout is older than
//! ふゆから、くるる。's ([`crate::games::fuyukuru::vo`]).
//!
//! A file is a head of three u32: the record count, what the records
//! change and the time in milliseconds from the start of the voice; then
//! that many 792-byte records:
//!
//! - slot (character slot, picture layer or hologram place) and size in
//!   percent (i32 each), then a flag (1: the third name is cheeks);
//! - body, face and cheeks file names (260 bytes each).
//!
//! What changes: 0 the message window portrait, 1 a standing character,
//! 2 the event picture (body) and its difference picture (face) in the
//! slot, 3 Apeiria on the hologram panel. The new picture fades in over
//! the old one; at the next message the change is made at once if its
//! time has not come yet.

use crate::games::fuyukuru::vo::Step;

const HEAD: usize = 12;
const RECORD: usize = 792;
const NAME: usize = 260;

fn int(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn name(bytes: &[u8], at: usize) -> Option<String> {
    let bytes = bytes.get(at..at + NAME)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
    (!s.is_empty()).then_some(s)
}

/// The changes of a `.vo` file, each with the milliseconds from the start
/// of the voice.
pub fn parse(bytes: &[u8]) -> Vec<(u32, Step)> {
    let (Some(count), Some(kind), Some(ms)) = (int(bytes, 0), int(bytes, 4), int(bytes, 8)) else {
        return Vec::new();
    };
    let at = ms.max(0) as u32;
    let mut out = Vec::new();
    for i in 0..count.clamp(0, 64) as usize {
        let Some(r) = bytes.get(HEAD + i * RECORD..HEAD + (i + 1) * RECORD) else {
            break;
        };
        let (slot, scale, flag) = (int(r, 0).unwrap_or(0), int(r, 4).unwrap_or(100), int(r, 8));
        let (Some(body), face) = (name(r, 12), name(r, 12 + NAME)) else {
            continue;
        };
        let cheeks = name(r, 12 + 2 * NAME).filter(|_| flag == Some(1));
        let step = match kind {
            0 => Step::Portrait { body, face, cheeks },
            1 => Step::Character {
                slot,
                scale,
                body,
                face,
                cheeks,
            },
            2 => Step::Picture {
                background: body,
                slot,
                layer: face,
            },
            3 => Step::Holo {
                place: slot,
                scale,
                body,
                faces: face.into_iter().chain(cheeks).collect(),
            },
            _ => continue,
        };
        out.push((at, step));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(kind: i32, ms: i32, ints: [i32; 3], names: [&str; 3]) -> Vec<u8> {
        let mut f = Vec::new();
        for v in [1, kind, ms].iter().chain(&ints) {
            f.extend_from_slice(&v.to_le_bytes());
        }
        for n in names {
            let mut b = n.as_bytes().to_vec();
            b.resize(NAME, 0);
            f.extend(b);
        }
        f
    }

    #[test]
    fn event_picture() {
        let f = file(
            2,
            1184,
            [0, 50, 0],
            ["ev0100.akb", "ev0100_h01.akb", "x.akb"],
        );
        assert_eq!(
            parse(&f),
            vec![(
                1184,
                Step::Picture {
                    background: "ev0100.akb".into(),
                    slot: 0,
                    layer: Some("ev0100_h01.akb".into())
                }
            )]
        );
    }

    #[test]
    fn character_with_cheeks() {
        let f = file(
            1,
            1700,
            [2, 80, 1],
            ["ch01_01_00.akb", "ch01_01_00_h04.akb", "c.akb"],
        );
        assert_eq!(
            parse(&f),
            vec![(
                1700,
                Step::Character {
                    slot: 2,
                    scale: 80,
                    body: "ch01_01_00.akb".into(),
                    face: Some("ch01_01_00_h04.akb".into()),
                    cheeks: Some("c.akb".into())
                }
            )]
        );
    }
}
