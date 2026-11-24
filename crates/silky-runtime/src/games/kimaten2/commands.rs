//! きまぐれテンプテーション2's own `Game` commands. Each is described in
//! `docs/game-commands.md`.

use silky_format::game::{Kind, Title};
use silky_vm::{Flow, Value, Vm};

use crate::audio::Bus;
use crate::games::cmd::{PICTURE, SELECT, SPEAKER, TITLE};
use crate::host::{ActionButton, Pause, Runtime};

/// Commands handled as in the common set (30 to 32 open the CG, scene and
/// music modes, see [`super::omake`]). Its other commands are its own;
/// those not below are reported as not carried out.
pub const SHARED_COMMANDS: [i32; 7] = [SPEAKER, PICTURE, SELECT, TITLE, 30, 31, 32];

/// E-mote model in the message window.
const EMOTE: i32 = 10;
/// E-mote standing models by slot.
const EMOTES: i32 = 40;
/// Action menu.
const ACTION_MENU: i32 = 42;
/// Map.
const MAP: i32 = 43;
/// H mode.
const H_MODE: i32 = 49;
/// The costume screen of アンネの部屋.
const COSTUME: i32 = 51;

impl Runtime {
    /// きまぐれテンプテーション2's own `Game` commands, before the sub
    /// command is taken off `args`. `None` for the others.
    pub(crate) fn kimaten2_game(
        &mut self,
        vm: &mut Vm,
        command: i32,
        sub: i32,
        args: &[Value],
    ) -> Option<silky_vm::Result<Flow>> {
        match command {
            ACTION_MENU => {
                self.actions = (0..7)
                    .filter_map(|i| {
                        let v = vm.var_b(i) as u32;
                        (v != 0).then_some(ActionButton {
                            result: i as i32 + 1,
                            icon: v % 100 % 50,
                            tick: match v / 50 {
                                0 => 0,
                                1 => 1,
                                _ => 2,
                            },
                        })
                    })
                    .collect();
                if self.actions.is_empty() {
                    vm.push(-1);
                    return self.done();
                }
                self.wait_for(Pause::Actions)
            }
            MAP => {
                use super::map;
                // (voice set or -1, back button, which map)
                let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
                let (voice, back, second) = (n(0), n(1) == 1, n(2) != 0);
                if let Some(set) = map::VOICES
                    .get(voice.max(0) as usize)
                    .filter(|_| voice >= 0)
                {
                    let file = set[vm.var_b(100) as usize % 3];
                    if let Err(e) = self.play(Bus::Voice, Kind::Voice, 0, file) {
                        return Some(Err(e.into()));
                    }
                }
                self.map = Some(map::Map {
                    second,
                    back,
                    bytes: (0..30).map(|i| vm.var_b(i)).collect(),
                    panel: None,
                });
                self.wait_for(Pause::Map)
            }
            H_MODE => self.wait_for(Pause::HMode),
            COSTUME => self.wait_for(Pause::Costume(sub)),
            EMOTE if self.kimaten2_emote(args, sub) => self.done(),
            // Apply: crossfade over the longest time given.
            EMOTES if sub == 2 && args.len() == 1 => {
                let t = std::mem::take(&mut self.character_time);
                self.handled += 1;
                Some(Ok(self.transition(t, None)))
            }
            EMOTES if self.kimaten2_emotes(args, sub, vm) => self.done(),
            _ => None,
        }
    }

    /// E-mote model in the message window (command 10): 6 load the two
    /// files of a costume, 7 face, 9 show, 11 hide, 12 and 13 pose, 16 `a`
    /// timeline, 17 a number kept on each model that this runtime does not
    /// use. False for the others. きまぐれテンプテーション's companion takes
    /// the same calls.
    pub(crate) fn kimaten2_emote(&mut self, args: &[Value], sub: i32) -> bool {
        let args = &args[..args.len().saturating_sub(1)];
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        let emote = &mut self.scene.emote;
        match (sub, args.len()) {
            (6, 2) => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_ascii_lowercase())
                    .collect();
                let first = names.iter().find(|n| n.contains("_h0007"));
                let second = names.iter().find(|n| n.contains("_h0813"));
                let (Some(a), Some(b)) = (first, second) else {
                    return false;
                };
                let files = [a.clone(), b.clone()];
                if emote.as_ref().map(|e| &e.files) != Some(&files) {
                    *emote = Some(crate::scene::Emote {
                        files,
                        ..Default::default()
                    });
                }
            }
            (7, 2) => emote.iter_mut().for_each(|e| e.face = Some((n(1), n(0)))),
            (12 | 13, 1) => emote.iter_mut().for_each(|e| e.pose = Some(n(0))),
            (16, 1) => emote.iter_mut().for_each(|e| e.sub = Some(n(0))),
            (17, 1) => {}
            // Scene skip point: with the original's "skip H scenes" setting
            // on, it ends the scene here. That setting is not offered, so
            // it does nothing, as with the setting off.
            (4, 0) => {}
            (9, 0) => emote.iter_mut().for_each(|e| e.visible = true),
            (11, 0) => emote.iter_mut().for_each(|e| e.visible = false),
            _ => return false,
        }
        true
    }

    /// Standing E-mote models (command 40; the slot is the last value
    /// except for sub 1): 0 and 12 load into a slot (frames, size in
    /// percent, the two files, slot; 12 has one more value first), 1 face
    /// (slot, extra, face), 15 main timeline, 14 `a` timeline, 13 a number
    /// kept on the model, 8 move by (dy, dx), 3 remove one (frames, slot),
    /// 4 remove all (frames), 16 the mouth track of the voice. The frames
    /// are kept for sub 2, which crossfades like command 13. False for the
    /// others. An H scene model starts in the second outfit's picture
    /// when byte 708 is 1.
    pub(crate) fn kimaten2_emotes(&mut self, args: &[Value], sub: i32, vm: &Vm) -> bool {
        let args = &args[..args.len().saturating_sub(1)];
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        let last = n(args.len().saturating_sub(1));
        match (sub, args.len()) {
            (0, 5) | (12, 6) => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_ascii_lowercase())
                    .collect();
                // Standing models pair `_h0007` (first file) with
                // `_h0813`; H mode models (`hg030100_10`, `_11`) come in
                // the same places, and the script keeps `_10` first.
                let (a, b) = match (
                    names.iter().find(|n| n.contains("_h0007")),
                    names.iter().find(|n| n.contains("_h0813")),
                ) {
                    (Some(a), Some(b)) => (a, b),
                    _ if names.len() == 2 && names.iter().all(|n| n.starts_with("hg")) => {
                        (names.iter().min().unwrap(), names.iter().max().unwrap())
                    }
                    _ => return false,
                };
                let scale = n(args.len() - 4);
                self.character_time = self.character_time.max(n(args.len() - 5));
                let place = crate::scene::Place::standing(last, scale, a);
                let files = [a.clone(), b.clone()];
                self.scene.emotes.insert(
                    last,
                    crate::scene::Emote {
                        files,
                        visible: true,
                        place: Some(place),
                        outfit: vm.bytes.get(super::hmode::OUTFIT_VAR) == Some(&1),
                        ..Default::default()
                    },
                );
            }
            (1, 3) => {
                if let Some(e) = self.scene.emotes.get_mut(&n(0)) {
                    e.face = Some((n(2), n(1)));
                }
            }
            (13..=15, 2) => {
                if let Some(e) = self.scene.emotes.get_mut(&last) {
                    match sub {
                        13 => e.next_pose = Some(n(0)),
                        14 => e.sub = Some(n(0)),
                        15 => e.pose = Some(n(0)),
                        _ => {}
                    }
                }
            }
            // (dy, dx, slot): moves the driver position.
            (8, 3) => {
                if let Some(p) = self
                    .scene
                    .emotes
                    .get_mut(&last)
                    .and_then(|e| e.place.as_mut())
                {
                    p.x += n(1);
                    p.y += n(0);
                }
            }
            (3, 2) => {
                self.scene.emotes.remove(&last);
                self.character_time = self.character_time.max(n(0));
            }
            (4, 1) => {
                self.scene.emotes.clear();
                self.character_time = self.character_time.max(n(0));
            }
            (16, 1) => {
                let Some(name) = args[0].as_str() else {
                    return false;
                };
                self.lip_track = None;
                self.lip = match self.assets.data(name) {
                    Ok(bytes) => {
                        let track = parse_lip(&bytes);
                        self.lip_track = Some(track.clone());
                        Some((track, 0.0))
                    }
                    Err(_) => {
                        self.note_unhandled(format!("missing {name}"));
                        None
                    }
                };
            }
            _ => return false,
        }
        true
    }

    /// Marks the picture on screen as seen in the CG mode when it is one of
    /// its pictures (the engine does this as a picture change is
    /// committed). Does nothing in the other games.
    pub(crate) fn register_cg(&mut self, vm: &mut Vm) {
        if self.assets.game().title != Title::Kimaten2 {
            return;
        }
        let Some(bg) = &self.scene.background else {
            return;
        };
        let layers = self.scene.overlays.values().map(String::as_str);
        if super::omake::register_cg(&mut vm.bytes, bg, layers).is_some() {
            vm.set_var_b(super::omake::CG_ANY, 1);
        }
    }
}

/// A mouth track: UTF-16 text of comma separated openings.
fn parse_lip(bytes: &[u8]) -> Vec<f32> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
        .trim_start_matches('\u{feff}')
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn lip_track() {
        let text: Vec<u8> = "\u{feff}+0.0, 0.45, 5.5,"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(super::parse_lip(&text), vec![0.0, 0.45, 5.5]);
    }
}
