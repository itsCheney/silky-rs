//! `Game` commands of 先輩が私の妄想にドージンする, read from its executable.

use silky_vm::{Flow, Vm};

use crate::games::Call;
use crate::host::{FollowUp, Pause, Runtime};
use silky_vm::native::GAME;

/// The doujinshi pages (`DOJIN.MES`).
const DOUJINSHI: i32 = 40;
/// Effect objects; sub 0 creates one and returns its handle.
const EFFECT: i32 = 1006;
/// Pictures that change with the voice line (`.vo` files).
const VOICE_PICTURES: i32 = 1007;
/// Zoom and white flash.
const FLUSH: i32 = 1005;
/// The screen shake.
const QUAKE: i32 = 993;
/// Auto save point.
const AUTO_SAVE: i32 = 14;
/// The second portrait window: 6 body, 7 face, 9 show, 11 hide.
const PORTRAIT2: i32 = 16;
/// The title menu; its argument is 1 to start on the extras page.
const TITLE: i32 = 110;

impl Runtime {
    /// This game's own commands. `None` for the others.
    pub(crate) fn doujin_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match (c.command, c.sub) {
            (TITLE, _) => {
                self.k1_title_extras = c.int(0) == 1;
                self.wait_for(Pause::Title)
            }
            (PORTRAIT2, 6) => {
                scene.portrait2 = c.str(0).map(|b| (b, None));
                self.done()
            }
            (PORTRAIT2, 7) => {
                if let Some(p) = &mut scene.portrait2 {
                    p.1 = c.str(0);
                }
                self.done()
            }
            (PORTRAIT2, 9) => {
                scene.portrait2_visible = true;
                self.done()
            }
            (PORTRAIT2, 11) => {
                scene.portrait2_visible = false;
                self.done()
            }
            // Not carried out yet, but these subs leave a value for the
            // script.
            (DOUJINSHI, 0 | 1) => {
                self.skip(format!("Game.{}.{}", c.command, c.sub), vm, GAME, c.command);
                vm.push(0);
                Some(Ok(Flow::Continue))
            }
            // "breath" (file, mode, dx, dy, frames, rest): a puff of
            // breath drifting over the picture, from the file's start;
            // one replaces the one before and its handle is always 0.
            (EFFECT, 0) if c.args.len() == 6 => {
                let brt = c
                    .str(5)
                    .and_then(|f| self.assets.data(&f).ok())
                    .and_then(|b| super::breath::Brt::parse(&b));
                if let Some(brt) = brt {
                    let d = (c.int(3), c.int(2));
                    super::breath::start(&mut self.scene, &brt, d, c.int(1), c.int(0));
                }
                vm.push(0);
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            // The puff finishes its round and stops.
            (EFFECT, 10) => {
                super::breath::finish(&mut self.scene);
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            // The puff goes; 11 clears all effects of this kind.
            (EFFECT, 1 | 11) => {
                super::breath::clear(&mut self.scene);
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            // (file): pictures that change with the voice line; 1 ends them.
            (VOICE_PICTURES, 0) => {
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
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            (VOICE_PICTURES, 1) => {
                self.vo_steps.clear();
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            // The screen jumps to 1.2 times its size, turns white and
            // comes back over 16 frames.
            (FLUSH, 0) => {
                self.scene.zoom = Some(120);
                self.then = Some(FollowUp::FlushWhite);
                self.handled += 1;
                Some(Ok(self.transition(1, None)))
            }
            // Shake: only stopping is ever called; it ends any shaking.
            (QUAKE, 1) => {
                self.scene.quake = None;
                self.scene.shake = (0, 0);
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            // (time, rule): the picture on screen wipes away through a
            // rule picture over the time, to the picture under it.
            (12, 8) if c.args.len() == 2 => {
                self.handled += 1;
                Some(Ok(self.transition(c.int(0), Some(c.int(1).max(0) as u32))))
            }
            // The auto save point.
            (AUTO_SAVE, -1) => {
                self.auto_save = true;
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            // Accepted with nothing to do: a breath animation is never
            // attached to a standing character (13.18); the shake object
            // is only set up in unused scripts (993.0, 2, 3, 4); the
            // screen overlay (1003) and the timer list (961.1) are never
            // set up; 12.7 resets tables that start empty; the "skip
            // scenes" setting is not offered (10.4, 10.5) and 10.16 only
            // sets up.
            (13, 18) | (QUAKE, 0 | 2..=4) | (1003, _) | (961, 1) | (12, 7) | (10, 4 | 5 | 16) => {
                self.handled += 1;
                Some(Ok(Flow::Continue))
            }
            _ => None,
        }
    }
}

/// Engine calls this runtime has nothing to do for: the same set as
/// 景の海のアペイリア has, whose engine this build shares these calls with.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        // Set up at the start: the message window page (MesWnd.1), font
        // sizes, the one-line message, menus, selection settings.
        (2, 1) | (10, 0 | 1) | (24, 0 | 2) | (26, 2..=4) | (1, 0)
            // Its own backlog: size, clear, commit of the entry.
            | (22, 0..=2)
    )
}
