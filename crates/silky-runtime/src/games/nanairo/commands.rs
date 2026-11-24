//! `Game` commands of なないろリンカネーション, read from its executable.

use silky_vm::{Flow, Vm};

use crate::games::Call;
use crate::host::{Pause, Runtime};

/// Title menu; returns the item picked, with the codes of the common
/// title menu.
const TITLE: i32 = 18;

impl Runtime {
    /// This game's own commands. `None` for the others.
    pub(crate) fn nanairo_game(
        &mut self,
        _vm: &mut Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        match c.command {
            TITLE => self.wait_for(Pause::Title),
            _ => None,
        }
    }
}
