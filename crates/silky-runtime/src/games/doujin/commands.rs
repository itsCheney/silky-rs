//! `Game` commands of 先輩が私の妄想にドージンする, read from its executable.

use silky_vm::{Flow, Vm};

use crate::games::Call;
use crate::host::Runtime;
use silky_vm::native::GAME;

/// The doujinshi pages (`DOJIN.MES`).
const DOUJINSHI: i32 = 40;
/// Effect objects; sub 0 creates one and returns its handle.
const EFFECT: i32 = 1006;

impl Runtime {
    /// This game's own commands. `None` for the others.
    pub(crate) fn doujin_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        match (c.command, c.sub) {
            // Not carried out yet, but these subs leave a value for the
            // script; the others of the same command leave none.
            (DOUJINSHI, 0 | 1) | (EFFECT, 0) => {
                self.skip(format!("Game.{}.{}", c.command, c.sub), vm, GAME, c.command);
                vm.push(0);
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
