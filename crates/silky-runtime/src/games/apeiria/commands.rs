//! `Game` commands of 景の海のアペイリア, read from its executable.

use silky_vm::{Flow, Vm};

use super::mail::Mail;
use crate::games::Call;
use crate::host::Runtime;

/// The αMAIL screen (`docs/game-commands.md`).
const MAIL: i32 = 1012;

impl Runtime {
    /// This game's own commands. `None` for the others.
    pub(crate) fn apeiria_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        if c.command != MAIL {
            return None;
        }
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
