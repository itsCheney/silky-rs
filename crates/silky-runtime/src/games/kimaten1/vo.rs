//! きまぐれテンプテーション's `.vo` files (`Game.1007(file)`): changes of
//! the E-mote models a set time after a voice line starts.
//!
//! A file is a count and that many records one after the other. A record
//! starts with four i32: its kind, two values and the time in milliseconds
//! from the start of the line. The kinds:
//!
//! - 3 and 4, 572 bytes (52 with no models): a count of models (-1: take
//!   the model away), seven more i32, and that many 260 byte file names, the
//!   `_h0007` and `_h0813` files of a costume. Kind 3 is a standing model,
//!   its first value the slot; kind 4 is the companion beside the window.
//!   A model that is not there yet is put up, fading in over 12 frames
//!   (16 for the companion); one that is there is left as it is. With -1
//!   the model goes, fading out over 12 frames.
//! - 2, 284 bytes plus 265 per picture: pictures to lay in a layer; no file
//!   of the game lists one, so nothing is drawn for it.
//! - 0 and 1 (796 and 804 bytes) are not used by the game's files.
//!
//! The seven i32 after the count are, as far as they could be matched with
//! the calls of the script, a size in percent, the face, the extra timeline
//! of the face, and the `a` and main timelines (-1 for none), as the script
//! gives them to a standing model; the original puts a model up without
//! them, so they only dress a model that was missing.

use super::effects::{self, Fade};
use super::emote::{companion, standing, COMPANION, COMPANION_FADE};
use crate::scene::{Emote, Scene};

/// Frames a standing model fades over when the file puts it up or takes
/// it away.
const STANDING_FADE: i32 = 12;

const HEAD: usize = 16;
const NAME: usize = 260;

/// What a record does when its time comes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A standing model of `slot`: put up if missing.
    Stand {
        slot: i32,
        files: [String; 2],
        scale: i32,
        face: (i32, i32),
        sub: Option<i32>,
        pose: Option<i32>,
    },
    /// The model of `slot` fades out and goes.
    Leave { slot: i32 },
    /// The companion beside the message window: put up if it is another.
    Companion {
        files: [String; 2],
        face: (i32, i32),
        sub: Option<i32>,
        pose: Option<i32>,
    },
    /// The companion fades out and goes.
    LeaveCompanion,
}

fn int(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn name(b: &[u8], at: usize) -> Option<String> {
    let b = b.get(at..at + NAME)?;
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let s = String::from_utf8_lossy(&b[..end]).to_ascii_lowercase();
    (!s.is_empty()).then_some(s)
}

fn some(v: i32) -> Option<i32> {
    (v >= 0).then_some(v)
}

/// The changes of a `.vo` file, each with the milliseconds from the start
/// of the voice line, in the order of the file.
pub fn parse(bytes: &[u8]) -> Vec<(u32, super::super::fuyukuru::vo::Step)> {
    use super::super::fuyukuru::vo::Step;
    let mut out = Vec::new();
    let Some(count) = int(bytes, 0) else {
        return out;
    };
    let mut at = 4;
    for _ in 0..count.clamp(0, 64) {
        let (Some(kind), Some(slot), Some(ms)) =
            (int(bytes, at), int(bytes, at + 4), int(bytes, at + 12))
        else {
            break;
        };
        let models = int(bytes, at + HEAD).unwrap_or(0);
        let size = match kind {
            0 => 0x31c,
            1 => 0x324,
            2 => 0x11c + 0x109 * int(bytes, at + 0x14).unwrap_or(0).max(0) as usize,
            _ => 0x34 + NAME * models.max(0) as usize,
        };
        let v = |i: usize| int(bytes, at + HEAD + 4 * i).unwrap_or(-1);
        let change = match kind {
            3 | 4 if models < 0 => Some(if kind == 3 {
                Change::Leave { slot }
            } else {
                Change::LeaveCompanion
            }),
            3 | 4 => {
                let n = |i: usize| name(bytes, at + 0x34 + i * NAME);
                match (n(0), n(1)) {
                    (Some(a), Some(b)) if a.contains("_h0007") && b.contains("_h0813") => {
                        let files = [a, b];
                        let face = (v(3).max(0), v(4).max(0));
                        Some(if kind == 3 {
                            Change::Stand {
                                slot,
                                files,
                                scale: v(2).max(1),
                                face,
                                sub: some(v(6)),
                                pose: some(v(7)),
                            }
                        } else {
                            Change::Companion {
                                files,
                                face,
                                sub: some(v(6)),
                                pose: some(v(7)),
                            }
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(c) = change {
            out.push((ms.max(0) as u32, Step::Emote(c)));
        }
        at += size;
    }
    out
}

impl Change {
    /// Makes the change in the scene.
    pub fn apply(&self, scene: &mut Scene) {
        match self.clone() {
            Change::Stand {
                slot,
                files,
                scale,
                face,
                sub,
                pose,
            } => {
                if scene.emotes.contains_key(&slot) {
                    return;
                }
                scene.emotes.insert(
                    slot,
                    Emote {
                        files,
                        face: Some(face),
                        sub,
                        pose,
                        visible: true,
                        place: Some(standing(slot, scale)),
                        ..Default::default()
                    },
                );
                effects::fade(&mut scene.effects, slot, Fade::In(STANDING_FADE));
            }
            Change::Leave { slot } => {
                effects::drop_fade(&mut scene.effects, slot);
                if let Some(old) = scene.emotes.remove(&slot) {
                    let gk = effects::leave(scene, slot, old);
                    effects::fade(&mut scene.effects, gk, Fade::Out(STANDING_FADE));
                    effects::bind(scene, gk);
                }
            }
            Change::Companion {
                files,
                face,
                sub,
                pose,
            } => {
                if scene.emote.as_ref().is_some_and(|e| e.files == files) {
                    return;
                }
                let old = scene.emote.take().filter(|e| e.visible);
                scene.emote = Some(Emote {
                    files,
                    face: Some(face),
                    sub,
                    pose,
                    visible: true,
                    place: Some(companion()),
                    ..Default::default()
                });
                effects::drop_fade(&mut scene.effects, COMPANION);
                if let Some(old) = old {
                    let gk = effects::leave(scene, COMPANION, old);
                    effects::fade(&mut scene.effects, gk, Fade::Out(COMPANION_FADE));
                    effects::bind(scene, gk);
                }
                effects::fade(&mut scene.effects, COMPANION, Fade::In(COMPANION_FADE));
            }
            Change::LeaveCompanion => {
                effects::drop_fade(&mut scene.effects, COMPANION);
                if let Some(old) = scene.emote.take().filter(|e| e.visible) {
                    let gk = effects::leave(scene, COMPANION, old);
                    effects::fade(&mut scene.effects, gk, Fade::Out(COMPANION_FADE));
                    effects::bind(scene, gk);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: i32, slot: i32, ms: i32, models: i32, names: &[&str]) -> Vec<u8> {
        let mut r = Vec::new();
        for v in [kind, slot, 1, ms, models, 0, 50, 8, 1, -1, 10, 172, -1] {
            r.extend_from_slice(&v.to_le_bytes());
        }
        for n in names {
            let mut b = n.as_bytes().to_vec();
            b.resize(NAME, 0);
            r.extend(b);
        }
        r
    }

    #[test]
    fn standing_model_and_removal() {
        let mut f = 2i32.to_le_bytes().to_vec();
        f.extend(record(3, 1, 2554, 2, &["ta00_h0007.psb", "ta00_h0813.psb"]));
        f.extend(record(3, 1, 6000, -1, &[]));
        let steps = parse(&f);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].0, 2554);
        let mut scene = Scene::default();
        let super::super::super::fuyukuru::vo::Step::Emote(a) = steps[0].1.clone() else {
            panic!()
        };
        a.apply(&mut scene);
        assert_eq!(scene.emotes[&1].files[0], "ta00_h0007.psb");
        assert_eq!(scene.emotes[&1].face, Some((8, 1)));
        let super::super::super::fuyukuru::vo::Step::Emote(b) = steps[1].1.clone() else {
            panic!()
        };
        b.apply(&mut scene);
        assert!(scene.emotes.is_empty());
    }
}
