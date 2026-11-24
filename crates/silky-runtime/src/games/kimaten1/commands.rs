//! きまぐれテンプテーション's own `Game` commands. Each is described in
//! `docs/game-commands.md`.

use silky_vm::{Flow, Value, Vm};

use crate::games::cmd::{PICTURE, SPEAKER, TITLE};
use crate::host::Runtime;

/// Commands handled as in the common set. Its other commands are its own;
/// those not below are reported as not carried out.
pub const SHARED_COMMANDS: [i32; 3] = [SPEAKER, PICTURE, TITLE];

/// E-mote model of the companion beside the protagonist.
const EMOTE: i32 = 10;
/// E-mote standing models by slot.
const EMOTES: i32 = 40;

impl Runtime {
    /// きまぐれテンプテーション's own `Game` commands, before the sub
    /// command is taken off `args`. `None` for the others.
    pub(crate) fn kimaten1_game(
        &mut self,
        vm: &mut Vm,
        command: i32,
        sub: i32,
        args: &[Value],
    ) -> Option<silky_vm::Result<Flow>> {
        match command {
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
}

/// Engine calls with nothing to do here, as in 景の海のアペイリア (same
/// engine build): the message window page (`MesWnd.1`), font sizes, the
/// one-line message, menus and selection settings set up at the start,
/// and its own backlog's size, clearing and voice marks.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        (2, 1) | (10, 0 | 1) | (24, 0 | 2) | (26, 2..=4) | (1, 0) | (22, 0..=2)
    )
}
