//! `Game` commands of 先輩が私の妄想にドージンする, read from its executable.

use silky_vm::{Flow, Vm};

use super::omake::{self, Omake, Saved};
use crate::audio::Bus;
use crate::games::kimaten1::Screen;
use crate::games::Call;
use crate::host::{FollowUp, Pause, Runtime};

/// The doujinshi pages (`DOJIN.MES`).
const DOUJINSHI: i32 = 40;
/// The extras: the CG, scene and music modes.
const OMAKE_CG: i32 = 30;
const OMAKE_MUSIC: i32 = 32;
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
/// The second portrait window: 6 body, 7 face, 8 a picture over the face,
/// 9 show, 11 and 12 take it down.
const PORTRAIT2: i32 = 16;
/// The title menu; its argument is 1 to start on the extras page.
const TITLE: i32 = 110;

impl Runtime {
    /// The screen of the extras mode `mode` (the CG, scene or music mode),
    /// from the bytes of the script. `None` if its layout is missing.
    fn doujin_omake(&mut self, mode: usize, vm: &Vm) -> Option<Screen> {
        let areas = Omake::area_files().map(|name| {
            self.assets
                .data(name)
                .ok()
                .and_then(|d| silky_format::area::parse(&d).ok())
        });
        let [Some(a), Some(b), Some(c)] = areas else {
            return None;
        };
        let mut saved = Saved {
            seen: (0..super::cgdata::PICTURES.len().div_ceil(8))
                .map(|i| vm.var_b(omake::CG_SEEN + i))
                .collect(),
            volume: self.system.settings.music,
            ..Saved::default()
        };
        for (i, s) in saved.scenes.iter_mut().enumerate() {
            *s = vm.var_b(omake::SCENE_FIRST + i);
        }
        for (i, t) in saved.tracks.iter_mut().enumerate() {
            *t = vm.var_b(omake::MUSIC_FIRST + i);
        }
        for (t, b) in saved.tabs.iter_mut().zip(omake::TAB_BYTES) {
            *t = vm.var_b(b);
        }
        // The picture and scene modes play the opening's instrumental when
        // nothing is playing; the music mode leaves the sound alone.
        if mode < 2 && !self.audio.is_playing(Bus::Music, 0) {
            let _ = self.play_music(omake::BGM);
        }
        Some(Screen::Doujin(Box::new(Omake::open(
            mode,
            [a, b, c],
            saved,
        ))))
    }

    /// This game's own commands. `None` for the others.
    pub(crate) fn doujin_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match (c.command, c.sub) {
            (TITLE, _) => {
                self.k1_title_extras = c.int(0) == 1;
                self.wait_for(Pause::Title)
            }
            // The extras: CG, scene and music modes. The script gets -1 to
            // go back to the title, or the scene picked.
            (OMAKE_CG..=OMAKE_MUSIC, _) => {
                let Some(screen) = self.doujin_omake((c.command - OMAKE_CG) as usize, vm) else {
                    vm.push(omake::LEAVE);
                    return self.done();
                };
                self.k1 = Some(screen);
                self.wait_for(Pause::Kimaten1)
            }
            (PORTRAIT2, 6) => {
                scene.portrait2 = c.str(0).map(|b| (b, None));
                scene.portrait2_over = None;
                self.done()
            }
            (PORTRAIT2, 7) => {
                if let Some(p) = &mut scene.portrait2 {
                    p.1 = c.str(0);
                }
                self.done()
            }
            // (picture): drawn over the face, as cheeks or the like.
            (PORTRAIT2, 8) => {
                if scene.portrait2.is_some() {
                    scene.portrait2_over = c.str(0);
                }
                self.done()
            }
            (PORTRAIT2, 9) => {
                scene.portrait2_visible = true;
                self.done()
            }
            // Both take the window down: its pictures go, a new body must
            // be set before it can be shown again.
            (PORTRAIT2, 11 | 12) => {
                scene.portrait2 = None;
                scene.portrait2_over = None;
                scene.portrait2_visible = false;
                self.done()
            }
            // The doujinshi screen.
            (DOUJINSHI, _) => self.comic_game(vm, c),
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
            // Sprite.5 once, after the help picture is loaded: a rectangle
            // copy between layers, the picture is shown whole anyway.
            | (19, 5)
    )
}
