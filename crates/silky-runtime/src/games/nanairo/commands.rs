//! `Game` commands of なないろリンカネーション, read from its executable.

use silky_vm::{Flow, Vm};

use super::stage;
use crate::games::Call;
use crate::host::{Pause, Runtime};
use crate::scene::Character;

/// Standing characters.
const CHARACTER: i32 = 13;
/// CG registry.
const CG: i32 = 14;
/// Speaker name: `15(name)`.
const SPEAKER: i32 = 15;
/// Message window portrait.
const PORTRAIT: i32 = 12;
/// Title menu; returns the item picked, with the codes of the common
/// title menu.
const TITLE: i32 = 18;
/// `19(value, end, start)`: fills byte flags `start..end`.
const FILL: i32 = 19;
/// Capture of the screen for the CG registry.
const CAPTURE: i32 = 22;
/// Chapter end.
const CHAPTER_END: i32 = 26;
/// Initialisation of the message window.
const INIT: i32 = 27;

impl Runtime {
    /// This game's own commands. `None` for the others.
    pub(crate) fn nanairo_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        // Event pictures, effect pictures and scene movies.
        if matches!(c.command, 22 | 23 | 1001..=1006) && !(c.command == 22 && c.sub == 0) {
            return self.nanairo_effects(c);
        }
        match c.command {
            TITLE => {
                crate::games::nanairo::effects::clear_all(&mut self.scene.effects);
                self.wait_for(Pause::Title)
            }
            SPEAKER => {
                self.scene.speaker_name = c.str(0);
                self.done()
            }
            CHARACTER => self.nanairo_character(c),
            PORTRAIT => self.nanairo_portrait(c),
            CG => self.nanairo_cg(c, vm),
            FILL => {
                // The call has no sub command: the last value is the
                // first flag.
                let (value, end, start) = (c.int(0), c.int(1), c.sub);
                for i in start.max(0)..end.max(0) {
                    vm.set_var_b(i as usize, value as u8);
                }
                self.done()
            }
            CAPTURE => match c.sub {
                // The screen is copied for the save picture, and the
                // picture shown is registered.
                0 => {
                    self.scene.cg.register(vm);
                    self.done()
                }
                _ => None,
            },
            CHAPTER_END => {
                // Ends the message window before the next script; the
                // effect pictures do not go on into it.
                crate::games::nanairo::effects::clear_all(&mut self.scene.effects);
                self.scene.text = None;
                self.scene.speaker_name = None;
                self.done()
            }
            INIT => {
                self.scene.speaker_name = None;
                self.done()
            }
            // Object of the library's start up; nothing is read from it.
            10 => self.done(),
            // The calendar screen of a date change and the ending's
            // overlay are not drawn here; the script goes on at once.
            39 | 50 => self.done(),
            1007..=1009 => self.done(),
            _ => None,
        }
    }

    /// Standing characters.
    fn nanairo_character(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match c.sub {
            // (time, size, name, slot)
            0 if c.args.len() == 4 => {
                let (size, slot) = (c.int(1), c.int(3));
                let body = c.str(2)?;
                self.character_time = self.character_time.max(c.int(0));
                let character = Character {
                    offset: stage::offset(slot, size, &body),
                    body,
                    face: None,
                    scale: size,
                    cheeks: None,
                    alpha: 255,
                };
                scene.characters.insert(slot, character);
            }
            // (time, slot)
            1 if c.args.len() == 2 => {
                self.character_time = self.character_time.max(c.int(0));
                scene.characters.remove(&c.int(1));
            }
            // Commit: every change crossfades over the longest time given.
            2 => {
                let t = std::mem::take(&mut self.character_time);
                self.handled += 1;
                return Some(Ok(self.transition(t, None)));
            }
            // (flag, slot, name): the expression layer. The flag takes 0
            // and 1; what it changes is not known.
            5 if c.args.len() == 3 => {
                if let Some(ch) = scene.characters.get_mut(&c.int(1)) {
                    ch.face = c.str(2).filter(|f| !f.is_empty());
                }
            }
            // (time): remove all.
            7 if c.args.len() == 1 => {
                self.character_time = self.character_time.max(c.int(0));
                scene.characters.clear();
            }
            // (size, slot): zoom to a size, here at once.
            9 if c.args.len() == 2 => {
                let (size, slot) = (c.int(0), c.int(1));
                if let Some(ch) = scene.characters.get_mut(&slot) {
                    ch.scale = size;
                    ch.offset = stage::offset(slot, size, &ch.body);
                }
            }
            // Side of a slot (6), flags of the figure (11), motion (12)
            // and effects attached to a figure (8, 13, 15) are not drawn;
            // 14 detaches the attached effect layer, of which there is
            // none here.
            6 | 8 | 11 | 12 | 13 | 14 | 15 => {}
            _ => return None,
        }
        self.done()
    }

    /// The face window of the message window.
    fn nanairo_portrait(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match (c.sub, c.args.len()) {
            // (face, body): the face is drawn over the body when it
            // differs.
            (4, 2) => {
                let (face, body) = (c.str(0)?, c.str(1)?);
                let face = (!face.eq_ignore_ascii_case(&body)).then_some(face);
                scene.portrait = Some((body, face));
            }
            (5, 0) => scene.portrait_visible = true,
            (6, 0) => scene.portrait_visible = false,
            // The empty window.
            (7, 0) => scene.portrait = None,
            // A whole picture.
            (8, 1) => scene.portrait = c.str(0).map(|b| (b, None)),
            _ => return None,
        }
        self.done()
    }

    /// The CG registry.
    fn nanairo_cg(&mut self, c: &Call, vm: &mut Vm) -> Option<silky_vm::Result<Flow>> {
        let cg = &mut self.scene.cg;
        match (c.sub, c.args.len()) {
            (10, 1) => cg.start(&c.str(0)?),
            (1, 2) => cg.part(c.int(0).max(0) as usize, &c.str(1)?),
            // Register the combination; 5 and 7 also set a time of the
            // transition that is not kept.
            (11, 0) | (5, 0) | (7, 1) => cg.register(vm),
            // Start of a scene picture (the full-screen rectangle), and
            // clearing the name tables of the slots: nothing to keep.
            (4, 0) | (2, 1) | (3, 1) => {}
            _ => return None,
        }
        self.done()
    }
}

/// Engine calls with nothing to do here, checked on the executable: the
/// message window page (`MesWnd.1`), font sizes, the one-line message,
/// menus and selection settings set up at the start, and its own
/// backlog's size, clearing and entry marks, as in 景の海のアペイリア.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        (2, 1) | (10, 0 | 1) | (24, 0 | 2) | (26, 2..=4) | (1, 0) | (22, 0..=2)
    )
}
