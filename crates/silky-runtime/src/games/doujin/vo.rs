//! `.vo` files of this game: pictures that change while a voice line
//! plays (`Game.1007(file, 0)`), read from the executable and the files.
//!
//! A file is a count (i32) and that many entries of different sizes, each
//! with a 16-byte head: type, a flag, the number of the next entry (or -1
//! for the last) and how long it shows in milliseconds (all i32).
//!
//! - type 0, the message window portrait: three 260-byte names (body,
//!   face and a third the game does not use);
//! - type 1, a standing character: slot and size in percent (i32 each),
//!   then the same three names;
//! - type 2, an event picture: percent and item count (i32 each), the
//!   background name (260 bytes), then the items of 265 bytes each: five
//!   bytes of slot and flags and the name of a difference picture.

use crate::games::fuyukuru::vo::Step;

const NAME: usize = 260;
const HEAD: usize = 16;
const ITEM: usize = 265;

fn name(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.get(..NAME)?;
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).into_owned();
    (!s.is_empty()).then_some(s)
}

fn int(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn tail(bytes: &[u8], at: usize) -> &[u8] {
    bytes.get(at..).unwrap_or_default()
}

/// The pictures of a `.vo` file in the order they show, each with the
/// milliseconds from the start of the voice.
pub fn parse(bytes: &[u8]) -> Vec<(u32, Step)> {
    let count = int(bytes, 0).unwrap_or(0).max(0) as usize;
    // Entries differ in size: find where each starts.
    let mut starts = Vec::new();
    let mut at = 4usize;
    for _ in 0..count {
        let Some(kind) = int(bytes, at) else { break };
        starts.push(at);
        at += HEAD
            + match kind {
                0 => 3 * NAME,
                1 => 8 + 3 * NAME,
                _ => {
                    let items = int(bytes, at + HEAD + 4).unwrap_or(0).max(0) as usize;
                    8 + NAME + items * ITEM
                }
            };
    }
    let mut out = Vec::new();
    let (mut time, mut i, mut seen) = (0u32, 0usize, 0usize);
    while let Some(&at) = starts.get(i) {
        seen += 1;
        if seen > count {
            break;
        }
        let (kind, next, ms) = (int(bytes, at), int(bytes, at + 8), int(bytes, at + 12));
        let body = at + HEAD;
        match kind {
            Some(0) => {
                if let Some(b) = name(tail(bytes, body)) {
                    out.push((
                        time,
                        Step::Portrait {
                            body: b,
                            face: name(tail(bytes, body + NAME)),
                            cheeks: None,
                        },
                    ));
                }
            }
            Some(1) => {
                if let Some(b) = name(tail(bytes, body + 8)) {
                    out.push((
                        time,
                        Step::Character {
                            slot: int(bytes, body).unwrap_or(0),
                            scale: int(bytes, body + 4).unwrap_or(100),
                            body: b,
                            face: name(tail(bytes, body + 8 + NAME)),
                            cheeks: None,
                        },
                    ));
                }
            }
            Some(2) => {
                let items = int(bytes, body + 4).unwrap_or(0).max(0) as usize;
                if let Some(background) = name(tail(bytes, body + 8)) {
                    let first = body + 8 + NAME;
                    if items == 0 {
                        out.push((
                            time,
                            Step::Picture {
                                background: background.clone(),
                                slot: 0,
                                layer: None,
                            },
                        ));
                    }
                    for k in 0..items {
                        let item = first + k * ITEM;
                        out.push((
                            time,
                            Step::Picture {
                                background: background.clone(),
                                slot: int(bytes, item).unwrap_or(0),
                                layer: name(tail(bytes, item + 5)),
                            },
                        ));
                    }
                }
            }
            _ => {}
        }
        time += ms.unwrap_or(0).max(0) as u32;
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

    fn names(list: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for n in list {
            let mut b = n.as_bytes().to_vec();
            b.resize(NAME, 0);
            v.extend(b);
        }
        v
    }

    fn ints(list: &[i32]) -> Vec<u8> {
        list.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn timeline() {
        let mut f = ints(&[3]);
        f.extend(ints(&[0, 0, 1, 1000]));
        f.extend(names(&["a.akb", "a_h1.akb", ""]));
        f.extend(ints(&[2, 0, 2, 500, 100, 1]));
        f.extend(names(&["ev.akb"]));
        f.extend([0u8; 5]);
        f.extend(names(&["ev_d.akb"]));
        f.extend(ints(&[1, 0, -1, 0, 2, 80]));
        f.extend(names(&["c.akb", "", ""]));
        let steps = parse(&f);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].0, 0);
        assert_eq!(steps[1].0, 1000);
        assert_eq!(
            steps[1].1,
            Step::Picture {
                background: "ev.akb".into(),
                slot: 0,
                layer: Some("ev_d.akb".into())
            }
        );
        assert_eq!(steps[2].0, 1500);
    }
}
