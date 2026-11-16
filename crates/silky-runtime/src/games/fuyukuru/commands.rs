//! ふゆから、くるる。's own `Game` commands and the engine calls it makes
//! that need nothing here. Each is described in `docs/game-commands.md`.

use silky_vm::{Flow, Vm};

use crate::games::Call;
use crate::host::{FollowUp, Runtime};

/// ふゆから、くるる。's own commands.
mod cmd {
    /// `(colour, time)` with the rule last: fade out to a colour through a
    /// rule mask.
    pub const RULE_FADE_OUT: i32 = 111;
    /// Screen quake ("thread_shake").
    pub const QUAKE: i32 = 993;
    /// Zoom and white flash ("pic_flush").
    pub const FLUSH: i32 = 1005;
    /// `.vo` pictures that change with a voice line.
    pub const VOICE_PICTURES: i32 = 1007;
    /// Auto save point.
    pub const AUTO_SAVE: i32 = 14;
    /// `(mode, time)`: white wipe into and out of the future scenes.
    pub const FUTURE: i32 = 112;
    /// Heartbeat zoom ("doki").
    pub const PULSE: i32 = 113;
    /// Slash flash.
    pub const SLASH: i32 = 114;
    /// Snow: 0 starts it, 1 stops it.
    pub const SNOW: i32 = 115;
}

// Native classes named in [`nothing_to_do`].
const MESWND: i32 = 2;
const BACKLOG: i32 = 22;
const UI: i32 = 26;

impl Runtime {
    /// ふゆから、くるる。's own `Game` commands. `None` for the others,
    /// which go on to the common set.
    pub(crate) fn fuyukuru_game(
        &mut self,
        vm: &mut Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match (c.command, c.sub, c.args.len()) {
            (cmd::RULE_FADE_OUT, rule, 2) => {
                scene.cover = Some(c.int(0) as u32 & 0xff_ffff);
                self.handled += 1;
                return Some(Ok(self.transition(c.int(1), Some(rule.max(0) as u32))));
            }
            (cmd::SNOW, on @ (0 | 1), 0) => scene.snow = on == 0,
            // "pic_flush": the screen jumps to 1.2 times its size, turns
            // white and comes back over 16 frames.
            (cmd::FLUSH, 0, 0) => {
                scene.zoom = Some(120);
                self.then = Some(FollowUp::FlushWhite);
                self.handled += 1;
                return Some(Ok(self.transition(1, None)));
            }
            // "thread_shake": 0 sets up, 2 (size) picks 4 or 16 pixels, 3
            // starts the screen shaking up and down, 1 stops it.
            (cmd::QUAKE, 0, 0) => self.quake_mode = 0,
            (cmd::QUAKE, 2, 1) => self.quake_mode = c.int(0),
            (cmd::QUAKE, 3, 0) => {
                scene.quake = Some(if self.quake_mode == 1 { 16 } else { 4 });
            }
            (cmd::QUAKE, 1, 0) => {
                scene.quake = None;
                scene.shake = (0, 0);
            }
            // (file): pictures that change with the voice line; 1 ends them.
            (cmd::VOICE_PICTURES, 0, 1) => {
                let steps = c
                    .str(0)
                    .and_then(|f| self.assets.data(&f).ok())
                    .map(|b| super::vo::parse(&b))
                    .unwrap_or_default();
                self.vo_steps.clear();
                for (at, step) in steps {
                    if at == 0 {
                        step.apply(&mut self.scene);
                    } else {
                        self.vo_steps.push((at, step));
                    }
                }
            }
            (cmd::VOICE_PICTURES, 1, 0) => self.vo_steps.clear(),
            (cmd::AUTO_SAVE, -1, 0) => self.auto_save = true,
            // (time) with 0 first: the picture goes white through rule 17
            // ("future_in") while it shrinks away turning; with 1 or 2 it
            // comes back ("future_out"), growing while its turns slow.
            (cmd::FUTURE, mode @ 0..=2, 1) => {
                scene.cover = (mode == 0).then_some(0xff_ffff);
                self.handled += 1;
                let flow = self.transition(c.int(0), Some(17));
                if let Some(crate::host::Pause::Transition(t)) = &mut self.pause {
                    t.spin = Some(crate::host::Spin {
                        grow: mode != 0,
                        reverse: mode == 2,
                        frames: c.int(0),
                    });
                }
                return Some(Ok(flow));
            }
            // "doki": the screen jumps to 1.2 times its size and shrinks
            // back over 8 frames.
            (cmd::PULSE, -1, 0) => {
                scene.zoom = Some(120);
                self.then = Some(FollowUp::EndZoom { frames: 8 });
                self.handled += 1;
                return Some(Ok(self.transition(1, None)));
            }
            // "slash": the picture of sprite 6 (`Sprite.1`) comes in over a
            // black screen through rule 0 in 4 frames and goes in 6.
            (cmd::SLASH, -1, 0) => {
                self.handled += 1;
                let Some(name) = scene.layers.remove(&6) else {
                    return Some(Ok(Flow::Continue));
                };
                scene.flash = Some(name);
                self.then = Some(FollowUp::EndFlash { frames: 6, rule: 0 });
                return Some(Ok(self.transition(4, Some(0))));
            }
            // "char_swing" (time, pixels, across, slot): the character
            // swings out by the pixels (sideways if `across` is 1, else up
            // and down) over the time in frames, to the other side over
            // twice that and back over the time.
            (crate::games::cmd::CHARACTER, 15, 4) => {
                let (t, d) = (c.int(0).max(1), c.int(1));
                let at = |f: i32| -> i32 {
                    match f {
                        f if f <= t => d * f / t,
                        f if f <= 3 * t => d - 2 * d * (f - t) / (2 * t),
                        f => -d + d * (f - 3 * t) / t,
                    }
                };
                let across = c.int(2) == 1;
                let steps = (1..=4 * t)
                    .map(|f| if across { (at(f), 0) } else { (0, at(f)) })
                    .collect();
                self.jolts.push((
                    c.int(3),
                    crate::host::Jolt {
                        steps,
                        step_ms: 1000 / 60,
                    },
                ));
            }
            // Nothing to do here:
            // - 10.4 ends the scene when the original's "skip scenes"
            //   setting is on, and 10.5 turns that setting off; the
            //   setting is not offered.
            // - 12.7 and 961.1 reset engine tables at the start.
            // - 116 shows a picture again after a save is loaded (this
            //   runtime keeps the scene in the save); 1 takes it away.
            // - 1003 sets up the original's message window text.
            // - 1006.11 clears a list of animations no script adds to;
            //   it returns 0.
            (10, 4 | 5, 0) | (12, 7, 0) | (961, 1, 1) | (116, 1, 0) | (1003, _, _) => {}
            (1006, 11, 0) => vm.push(0),
            _ => return None,
        }
        self.done()
    }
}

/// Engine calls of ふゆから、くるる。 that need nothing from this runtime,
/// read from the executable and the library scripts.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        // Set up at the start: message window (MesWnd.1 clears its text
        // for a new page; this runtime lays out each message itself), font
        // size, input, sound channels, the one-line message, menus.
        (MESWND, 1)
            | (10, 0 | 1)
            | (12, 5)
            | (17, 0)
            | (24, 0 | 2)
            | (UI, 2..=4)
            // Select.0(-1): selection settings; the selection itself is
            // `Game.20`.
            | (1, 0)
            // The original rebuilds its scene after a load from a store of
            // variables (Save.2 reads one, 4 writes, 5 and 6 delete) and
            // keeps its own backlog (BackLog.0..3) and message records
            // (MesWnd.3 with `Game.23` text); this runtime saves the scene
            // and keeps its own backlog.
            | (27, 2 | 4 | 5 | 6)
            | (BACKLOG, 0..=3)
            | (MESWND, 3)
    )
}
