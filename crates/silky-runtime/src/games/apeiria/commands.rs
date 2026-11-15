//! `Game` commands of 景の海のアペイリア, read from its executable.

use silky_vm::{Flow, Vm};

use super::holo::Holo;
use super::mail::Mail;
use crate::games::Call;
use crate::host::Runtime;

/// The αMAIL screen (`docs/game-commands.md`).
const MAIL: i32 = 1012;
/// Apeiria on the hologram panel.
const HOLO: i32 = 1013;

impl Runtime {
    /// This game's own commands. `None` for the others.
    pub(crate) fn apeiria_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        match c.command {
            MAIL => self.mail(vm, c),
            HOLO => self.holo(c),
            _ => None,
        }
    }

    /// `Game.1013`. The original skips the pictures while the scene skip
    /// setting is on; this runtime has it off.
    fn holo(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let holo = self.scene.holo.get_or_insert_with(Holo::default);
        match (c.sub, c.args.len()) {
            // (time, size, place, body): a new picture; faces start again.
            (0, 4) => {
                holo.body = c.str(3)?;
                holo.scale = c.int(1);
                holo.place = c.int(2);
                holo.faces.clear();
                holo.offset = (0, 0);
            }
            // (file): an expression or cheeks drawn over the picture.
            (1, 1) => {
                let f = c.str(0)?;
                holo.faces.retain(|x| !x.eq_ignore_ascii_case(&f));
                holo.faces.push(f);
            }
            // (how): 0 with light particles, 1 at once, 2 fading in.
            (2, 1) => holo.shown = !holo.body.is_empty(),
            // (time): fade out, then 5: gone.
            (3, 1) | (5, 0) => {
                holo.shown = false;
                holo.body.clear();
                holo.faces.clear();
            }
            // (dx, dy, time): move her picture.
            (4, 3) => holo.offset = (holo.offset.0 + c.int(0), holo.offset.1 + c.int(1)),
            // (half): panel picture.
            (6, 1) => holo.panel = c.int(0),
            _ => return None,
        }
        self.done()
    }

    fn mail(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let mail = &mut self.scene.mail;
        match (c.sub, c.args.len()) {
            // (tray): set up the list of the inbox (0) or the sent
            // tray (1); its newest mail is byte 1200 or 1400.
            (0, 1) => {
                let tray = c.int(0);
                let newest = vm.var_b(if tray == 0 { 1200 } else { 1400 });
                *mail = Some(Mail {
                    tray,
                    newest,
                    ..Mail::default()
                });
            }
            (1, 0) => mail.get_or_insert_with(Mail::default).shown = true,
            (2, 0) => *mail = None,
            (3, 1) => {
                let text = c.str(0)?.replace("\\n", "\n");
                mail.get_or_insert_with(Mail::default).body = Some(text);
            }
            (4, 0) => {
                if let Some(m) = mail {
                    m.body = None;
                }
            }
            // (count): "n new mails received".
            (5, 1) => mail.get_or_insert_with(Mail::default).notice = Some(c.int(0)),
            (6, 0) => {
                if let Some(m) = mail {
                    m.notice = None;
                }
            }
            // (row) picks a row of the list, 9 lets it go.
            (8, 1) => mail.get_or_insert_with(Mail::default).selected = Some(c.int(0)),
            (9, 0) => {
                if let Some(m) = mail {
                    m.selected = None;
                }
            }
            // (mail): byte 1200, the newest mail of the inbox.
            (11, 1) => vm.set_var_b(1200, c.int(0) as u8),
            _ => return None,
        }
        self.done()
    }
}

/// Engine calls this runtime has nothing to do for, checked on the
/// executable: the same set as ふゆから、くるる。 has, with this game's
/// numbers.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        // Set up at the start: the message window page (MesWnd.1), font
        // sizes, the one-line message, menus, selection settings.
        (2, 1) | (10, 0 | 1) | (24, 0 | 2) | (26, 2..=4) | (1, 0)
            // The store of variables the original rebuilds its scene from
            // after a load (Save.2 reads, 4 writes, 5 and 6 delete); this
            // runtime saves the scene itself.
            | (27, 2 | 4 | 5 | 6)
            // Its own backlog: size, clear, voice marks of the entry.
            | (22, 0..=2)
            // Settings read from the XML configuration (scene skip, auto
            // read, ...), read as 0 (off).
            | (25, 0 | 1)
    )
}
