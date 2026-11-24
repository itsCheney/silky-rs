//! きまぐれテンプテーション's own `Game` commands. Each is described in
//! `docs/game-commands.md`.

use silky_vm::{Flow, Value, Vm};

use super::actsel::ActSelect;
use super::areasel::AreaSelect;
use super::itemsel::{self, ItemSelect};
use super::map::Map;
use super::screen::Screen;
use crate::audio::Bus;
use crate::games::cmd::{CHARACTER, PICTURE, SPEAKER, TITLE};
use crate::host::{Pause, Runtime};
use silky_format::game::Kind;

/// Commands handled as in the common set. Its other commands are its own;
/// those not below are reported as not carried out.
pub const SHARED_COMMANDS: [i32; 3] = [SPEAKER, PICTURE, CHARACTER];

/// E-mote model of the companion beside the protagonist.
const EMOTE: i32 = 10;
/// E-mote standing models by slot.
const EMOTES: i32 = 40;
/// Investigation over the background.
const AREA: i32 = 41;
/// The action menu; its argument says whether the skip button may show.
const ACTION: i32 = 42;
/// The options of the script as a list to pick from.
const OPTIONS: i32 = 45;
/// Choosing a piece of evidence.
const ITEM: i32 = 46;
/// Bytes: the notebook is unlocked, the skip button is unlocked, and its
/// tutorial has been seen.
const NOTEBOOK_UNLOCKED: usize = 899;
const TUTORIAL_UNLOCKED: usize = 0xbbf;
const TUTORIAL_SEEN: usize = 0xbc0;
/// Voice channels of `Game.961`, apart from the message voices.
const LONG_VOICE_CHANNELS: i32 = 200;
/// Frames the string reveal of `Game.1000` runs beyond its own number.
const STRING_EXTRA_FRAMES: i32 = 64;
const ITEM_FLAGS: [usize; 5] = itemsel::FLAGS;
/// The map of the apartment: `(1)` with a back button, `(0)` without.
const MAP: i32 = 43;
/// The notices: `(text, 0)` queues one, `(1)` shows the queue, `(2)` takes
/// the strips down, `(3)` clears the queue, `(4)` returns whether strips
/// are still up.
const NOTICE: i32 = 44;
/// E-mote CG models.
const EMOTE_CG: i32 = 48;

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
            // The title menu; its argument 1 opens the extras page (back
            // from one of the modes).
            TITLE => {
                self.k1_title_extras = sub == 1;
                self.wait_for(Pause::Title)
            }
            EMOTE if self.kimaten1_emote(args, sub) => self.done(),
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
                    std::array::from_fn(|i| b(super::actsel::PROGRESS_FIRST + i)),
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
            // The options the script registered, as a selection: answers
            // the number of the one picked from 1. The engine draws its
            // own list with a back button (answer 100) here; the selection
            // bars of the other screens stand in for it.
            OPTIONS => {
                self.choices = std::mem::take(&mut vm.option_labels);
                self.choice_status.clear();
                self.option_count = 0;
                self.pause = Some(Pause::Choice);
                self.handled += 1;
                Some(Ok(Flow::Yield))
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
            EMOTES if self.kimaten1_emotes(args, sub, vm) => self.done(),
            EMOTE_CG if self.kimaten1_cg(args, sub) => self.done(),
            MAP => match Map::new(&self.assets, sub == 1, |i| vm.var_b(i)) {
                Ok(map) => {
                    self.k1 = Some(Screen::Map(map));
                    self.wait_for(Pause::Kimaten1)
                }
                // Without its pictures there is no map to pick from: go
                // back, as the back button would.
                Err(_) => {
                    vm.push(super::map::BACK_RESULT);
                    self.done()
                }
            },
            NOTICE => {
                match (sub, args.first()) {
                    (0, Some(text)) => {
                        if let Some(text) = text.as_str() {
                            self.k1_notices.push(text);
                        }
                    }
                    (1, _) => self.k1_notices.show(),
                    (2, _) => self.k1_notices.clear(),
                    (3, _) => self.k1_notices.clear_queue(),
                    (4, _) => vm.push(self.k1_notices.busy() as i32),
                    _ => {}
                }
                self.done()
            }
            // Auto save point.
            14 => {
                self.auto_save = true;
                self.done()
            }
            // The screen shake, set up as in 景の海のアペイリア's engine.
            993 => {
                let scene = &mut self.scene;
                match sub {
                    0 => self.quake_mode = 0,
                    2 => self.quake_mode = args.first().map(Value::int).unwrap_or(0),
                    3 => scene.quake = Some(if self.quake_mode == 1 { 16 } else { 4 }),
                    1 => {
                        scene.quake = None;
                        scene.shake = (0, 0);
                    }
                    _ => return None,
                }
                self.done()
            }
            // A line of text (a picture strip) wiped in through a rule
            // mask: (hold, frames, rule, height, width, y). The wipe is
            // not drawn here; the script waits as long as the original
            // does, the strip being there from the start.
            1000 => {
                let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
                let ms = crate::host::frames_ms(n(1) + STRING_EXTRA_FRAMES) + n(0).max(0) as u32;
                if ms == 0 {
                    return self.done();
                }
                self.wait_for(Pause::Transition(crate::host::Transition {
                    ms,
                    rule: None,
                    spin: None,
                    live: true,
                }))
            }
            // The back picture fades in or out over eight frames.
            999 => {
                self.handled += 1;
                Some(Ok(self.transition(8, None)))
            }
            // (channel, file) plays a voice that the next message does
            // not cut; sub 1 (channel) stops it.
            961 => {
                match sub {
                    0 => {
                        let file = args.get(1).and_then(Value::as_str)?.to_string();
                        let ch = LONG_VOICE_CHANNELS + args.first().map(Value::int).unwrap_or(0);
                        if let Err(e) = self.play(Bus::Voice, Kind::Voice, ch, &file) {
                            return Some(Err(silky_vm::Error::Host(e.to_string())));
                        }
                    }
                    1 => {
                        let ch = LONG_VOICE_CHANNELS + args.first().map(Value::int).unwrap_or(0);
                        self.audio.stop(Bus::Voice, ch)
                    }
                    _ => return None,
                }
                self.done()
            }
            1006 if sub == 0 => {
                vm.push(0);
                self.done()
            }
            1009 => {
                vm.push(0);
                self.done()
            }
            // Nothing to do here: the handle lists and engine tables
            // (1006.11, 1010, 1011), the two engine hooks 10008 and 10009,
            // and 35.2.
            1006 | 1010 | 1011 | 10008 | 10009 => self.done(),
            35 if sub == 2 => self.done(),
            CHARACTER if matches!(sub, 5 | 6 | 8 | 9 | 11 | 12) => self.done(),
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
