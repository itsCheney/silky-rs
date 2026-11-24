//! きまぐれテンプテーション's own `Game` commands. Each is described in
//! `docs/game-commands.md`.

use silky_vm::{Flow, Value, Vm};

use super::actsel::ActSelect;
use super::areasel::AreaSelect;
use super::itemsel::{self, ItemSelect};
use super::screen::Screen;
use crate::games::cmd::{PICTURE, SPEAKER, TITLE};
use crate::host::{Pause, Runtime};

/// Commands handled as in the common set. Its other commands are its own;
/// those not below are reported as not carried out.
pub const SHARED_COMMANDS: [i32; 3] = [SPEAKER, PICTURE, TITLE];

/// E-mote model of the companion beside the protagonist.
const EMOTE: i32 = 10;
/// E-mote standing models by slot.
const EMOTES: i32 = 40;
/// Investigation over the background.
const AREA: i32 = 41;
/// The action menu; its argument says whether the skip button may show.
const ACTION: i32 = 42;
/// Choosing a piece of evidence.
const ITEM: i32 = 46;
/// Bytes: the notebook is unlocked, the skip button is unlocked, and its
/// tutorial has been seen.
const NOTEBOOK_UNLOCKED: usize = 899;
const TUTORIAL_UNLOCKED: usize = 0xbbf;
const TUTORIAL_SEEN: usize = 0xbc0;
const ITEM_FLAGS: [usize; 5] = itemsel::FLAGS;

impl Runtime {
    /// The areas of one of the `.area` files, `None` if it is missing.
    fn k1_areas(&self, name: &str) -> Option<Vec<silky_format::area::Area>> {
        silky_format::area::parse(&self.assets.data(&name.to_ascii_uppercase()).ok()?).ok()
    }

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
            ACTION => {
                let areas = self.k1_areas("actselect.area")?;
                let b = |i: usize| vm.var_b(i);
                self.k1 = Some(Screen::Action(ActSelect::open(
                    areas,
                    [b(0), b(1), b(2), b(3)],
                    sub == 1,
                    b(TUTORIAL_UNLOCKED) != 0,
                    b(TUTORIAL_SEEN) != 0,
                    b(NOTEBOOK_UNLOCKED) != 0,
                )));
                self.wait_for(Pause::Kimaten1)
            }
            AREA => {
                let name = args.get(1).and_then(Value::as_str)?.to_string();
                let areas = self.k1_areas("areaselect.area")?;
                let map = self.assets.data(&name.to_ascii_uppercase()).ok()?;
                self.k1 = Some(Screen::Area(AreaSelect::open(
                    areas,
                    map,
                    vm.var_b(NOTEBOOK_UNLOCKED) != 0,
                )));
                self.wait_for(Pause::Kimaten1)
            }
            ITEM => {
                let areas = self.k1_areas("itemselect.area")?;
                let mut flags = [0u8; 5];
                for (f, i) in flags.iter_mut().zip(ITEM_FLAGS) {
                    *f = vm.var_b(i);
                }
                self.k1 = Some(Screen::Item(ItemSelect::open(
                    areas,
                    flags,
                    vm.var_b(NOTEBOOK_UNLOCKED) != 0,
                )));
                self.wait_for(Pause::Kimaten1)
            }
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
