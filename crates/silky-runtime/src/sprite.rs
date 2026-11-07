//! Sprite animations (class 19, subs 16 to 28), read from the executable
//! of ふゆから、くるる。.
//!
//! The library scripts build a timed animation on a sprite, put the sprite
//! in a group and run the group:
//!
//! - 16 `(sprite)`: starts a new animation for the sprite.
//! - 17 `(frames, sprite)`: its length (at least 1).
//! - 18 `(flag, sprite)`: a second setting, not needed here.
//! - 21 `(to, from, sprite)`: opacity from `from` to `to` (0..255).
//! - 24 `(hard, appear, rule, sprite)`: the sprite appears (`appear` 1) or
//!   goes (0) through the rule mask `NN.bin`, with a hard edge if `hard`.
//! - 25 `()`: new group; returns its number.
//! - 26 `(sprite, group)`: adds the sprite to the group.
//! - 27 `(wait, group)`: runs the group, waiting for its end if `wait` 1.
//! - 28 `(group)`: deletes the group.
//!
//! Sprite 6 sits over the pictures. The scripts use it two ways: a new
//! picture wiped in through a rule mask (`transwait_view`, `trans_view`),
//! and a black fill faded to half opacity that dims the screen until the
//! next picture is drawn over it.

use std::collections::BTreeMap;

use silky_vm::Value;

#[derive(Debug, Default, Clone, Copy)]
struct Animation {
    frames: i32,
    rule: Option<u32>,
    opacity: Option<u8>,
}

/// What a run of a group shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The scene changes through a rule mask over `frames`.
    Wipe { frames: i32, rule: u32 },
    /// A colour (0xRRGGBB) at an opacity covers the pictures, reached over
    /// `frames`.
    Dim {
        frames: i32,
        color: u32,
        opacity: u8,
    },
}

/// Sprite fills, animations and groups.
#[derive(Debug, Default)]
pub struct Sprites {
    fills: BTreeMap<i32, u32>,
    animations: BTreeMap<i32, Animation>,
    groups: BTreeMap<i32, Vec<i32>>,
    next_group: i32,
}

/// Result of a sprite call.
pub enum Call {
    /// Carried out, nothing to show.
    Done,
    /// Carried out; push this value as the result.
    Value(i32),
    /// Run a group: the effects of its sprites.
    Run(Vec<Effect>),
}

impl Sprites {
    /// Carries out sprite call `sub` with `args` (in script order), or
    /// `None` for a call this does not cover.
    pub fn call(&mut self, sub: i32, args: &[Value]) -> Option<Call> {
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        Some(match (sub, args.len()) {
            // (1, colour, sprite, height, width, y, x): fill.
            (6, 7) => {
                self.fills.insert(n(2), n(1) as u32 & 0xff_ffff);
                Call::Done
            }
            (16, 1) => {
                self.animations.insert(n(0), Animation::default());
                Call::Done
            }
            (17, 2) => {
                self.animations.entry(n(1)).or_default().frames = n(0).max(1);
                Call::Done
            }
            (18, 2) => Call::Done,
            (21, 3) => {
                self.animations.entry(n(2)).or_default().opacity = Some(n(0).clamp(0, 255) as u8);
                Call::Done
            }
            (24, 4) => {
                self.animations.entry(n(3)).or_default().rule = Some(n(2).max(0) as u32);
                Call::Done
            }
            (25, 0) => {
                self.next_group += 1;
                self.groups.insert(self.next_group, Vec::new());
                Call::Value(self.next_group)
            }
            (26, 2) => {
                self.groups.entry(n(1)).or_default().push(n(0));
                Call::Done
            }
            (27, 2) => {
                let sprites = self.groups.get(&n(1)).cloned().unwrap_or_default();
                let effects = sprites
                    .iter()
                    .filter_map(|s| {
                        let a = self.animations.remove(s)?;
                        if let Some(rule) = a.rule {
                            return Some(Effect::Wipe {
                                frames: a.frames,
                                rule,
                            });
                        }
                        Some(Effect::Dim {
                            frames: a.frames,
                            color: self.fills.get(s).copied().unwrap_or(0),
                            opacity: a.opacity?,
                        })
                    })
                    .collect();
                Call::Run(effects)
            }
            (28, 1) => {
                self.groups.remove(&n(0));
                Call::Done
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: &[i32]) -> Vec<Value> {
        v.iter().map(|&i| Value::Int(i)).collect()
    }

    /// The library's `transwait_view` wipe and its dimming function.
    #[test]
    fn library_effects() {
        let mut s = Sprites::default();
        s.call(16, &int(&[6]));
        s.call(18, &int(&[1, 6]));
        s.call(24, &int(&[0, 1, 44, 6]));
        s.call(17, &int(&[12, 6]));
        let Some(Call::Value(g)) = s.call(25, &[]) else {
            panic!()
        };
        s.call(26, &int(&[6, g]));
        let Some(Call::Run(e)) = s.call(27, &int(&[1, g])) else {
            panic!()
        };
        assert_eq!(
            e,
            vec![Effect::Wipe {
                frames: 12,
                rule: 44
            }]
        );
        s.call(28, &int(&[g]));

        s.call(6, &int(&[1, -16777216, 6, 720, 1280, 0, 0]));
        s.call(16, &int(&[6]));
        s.call(21, &int(&[128, 0, 6]));
        s.call(17, &int(&[16, 6]));
        let Some(Call::Value(g)) = s.call(25, &[]) else {
            panic!()
        };
        s.call(26, &int(&[6, g]));
        let Some(Call::Run(e)) = s.call(27, &int(&[1, g])) else {
            panic!()
        };
        assert_eq!(
            e,
            vec![Effect::Dim {
                frames: 16,
                color: 0,
                opacity: 128
            }]
        );
    }
}
