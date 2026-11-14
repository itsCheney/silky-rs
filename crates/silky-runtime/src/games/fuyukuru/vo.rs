//! `.vo` files of ふゆから、くるる。: pictures that change while a voice
//! line plays (`Game.1007(file, 0)`, from the scripts' `vof` macro), read
//! from the executable and the files.
//!
//! A file is a count (u32) and that many 796-byte records:
//!
//! - type (u32): 0 for the message window portrait, 1 for a standing
//!   character;
//! - a flag (u32), not needed here;
//! - the next record's number, or -1 for the last (i32);
//! - how long the record shows, in milliseconds (u32);
//! - type 0: body and face file names (260 bytes each);
//! - type 1: slot and size in percent (i32 each), then body and face.

use crate::scene::{Character, Scene};

const RECORD: usize = 796;
const NAME: usize = 260;

/// One picture of a `.vo` timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Portrait {
        body: String,
        face: Option<String>,
    },
    Character {
        slot: i32,
        scale: i32,
        body: String,
        face: Option<String>,
    },
}

impl Step {
    /// Puts the picture in the scene.
    pub fn apply(&self, scene: &mut Scene) {
        match self.clone() {
            Step::Portrait { body, face } => {
                scene.portrait = Some((body, face));
            }
            Step::Character {
                slot,
                scale,
                body,
                face,
            } => {
                let offset = scene
                    .characters
                    .get(&slot)
                    .map(|c| c.offset)
                    .unwrap_or_default();
                scene.characters.insert(
                    slot,
                    Character {
                        body,
                        face,
                        scale,
                        offset,
                    },
                );
            }
        }
    }
}

fn name(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
    (!s.is_empty()).then_some(s)
}

fn int(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The pictures of a `.vo` file in the order they show, each with the
/// milliseconds from the start of the voice.
pub fn parse(bytes: &[u8]) -> Vec<(u32, Step)> {
    let count = int(bytes, 0).unwrap_or(0).max(0) as usize;
    let record = |i: usize| bytes.get(4 + i * RECORD..4 + (i + 1) * RECORD);
    let mut out = Vec::new();
    let (mut at, mut i) = (0u32, 0usize);
    while i < count && out.len() < count {
        let Some(r) = record(i) else { break };
        let (kind, next, ms) = (int(r, 0), int(r, 8), int(r, 12));
        let step = match kind {
            Some(0) => name(&r[16..16 + NAME]).map(|body| Step::Portrait {
                body,
                face: name(&r[16 + NAME..16 + 2 * NAME]),
            }),
            Some(1) => name(&r[24..24 + NAME]).map(|body| Step::Character {
                slot: int(r, 16).unwrap_or(0),
                scale: int(r, 20).unwrap_or(100),
                body,
                face: name(&r[24 + NAME..24 + 2 * NAME]),
            }),
            _ => None,
        };
        if let Some(step) = step {
            out.push((at, step));
        }
        at += ms.unwrap_or(0).max(0) as u32;
        match next {
            Some(n) if n >= 0 => i = n as usize,
            _ => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: i32, next: i32, ms: i32, ints: &[i32], names: [&str; 2]) -> Vec<u8> {
        let mut r = Vec::new();
        for v in [kind, 1, next, ms].iter().chain(ints) {
            r.extend_from_slice(&v.to_le_bytes());
        }
        for n in names {
            let mut b = n.as_bytes().to_vec();
            b.resize(NAME, 0);
            r.extend(b);
        }
        r.resize(RECORD, 0);
        r
    }

    #[test]
    fn timeline() {
        let mut file = 2u32.to_le_bytes().to_vec();
        file.extend(record(0, 1, 3096, &[], ["a_01_00.akb", "a_01_00_h17.akb"]));
        file.extend(record(
            1,
            -1,
            8476,
            &[0, 75],
            ["a_01_00.akb", "a_01_00_h19.akb"],
        ));
        let steps = parse(&file);
        assert_eq!(steps.len(), 2);
        assert_eq!(
            steps[0],
            (
                0,
                Step::Portrait {
                    body: "a_01_00.akb".into(),
                    face: Some("a_01_00_h17.akb".into())
                }
            )
        );
        assert_eq!(
            steps[1],
            (
                3096,
                Step::Character {
                    slot: 0,
                    scale: 75,
                    body: "a_01_00.akb".into(),
                    face: Some("a_01_00_h19.akb".into())
                }
            )
        );
    }
}
