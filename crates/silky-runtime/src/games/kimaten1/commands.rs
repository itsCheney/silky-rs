//! きまぐれテンプテーション's own `Game` commands. Each is described in
//! `docs/game-commands.md`.

use silky_vm::{Flow, Value, Vm};

use super::actsel::ActSelect;
use super::areasel::AreaSelect;
use super::effects;
use super::exselect::{self, ExSelect};
use super::fx;
use super::itemsel::{self, ItemSelect};
use super::map::Map;
use super::omake::{self, Omake, Saved};
use super::screen::Screen;
use super::staff::Staff;
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
const ITEM_FLAGS: [usize; 5] = itemsel::FLAGS;
/// The map of the apartment: `(1)` with a back button, `(0)` without.
const MAP: i32 = 43;
/// The notices: `(text, 0)` queues one, `(1)` shows the queue, `(2)` takes
/// the strips down, `(3)` clears the queue, `(4)` returns whether strips
/// are still up.
const NOTICE: i32 = 44;
/// E-mote CG models.
const EMOTE_CG: i32 = 48;
/// The staff roll.
const STAFF: i32 = 990;

impl Runtime {
    /// The screen of the extras mode `mode`, from the bytes of the script.
    fn k1_omake(&mut self, mode: usize, vm: &Vm) -> Option<Screen> {
        let names = Omake::area_files();
        let areas = [
            self.k1_areas(names[0])?,
            self.k1_areas(names[1])?,
            self.k1_areas(names[2])?,
        ];
        let dialog = self.k1_areas("dialog.area")?;
        let mut saved = Saved {
            seen: (0..super::cgdata::PICTURES.len().div_ceil(8))
                .map(|i| vm.var_b(omake::CG_SEEN + i))
                .collect(),
            volume: self.system.settings.music,
            quiet_cell: vm.var_g(16).int() == 1,
            ..Saved::default()
        };
        for (i, s) in saved.scenes.iter_mut().enumerate() {
            *s = vm.var_b(omake::SCENE_FIRST + i);
        }
        for (i, t) in saved.tracks.iter_mut().enumerate() {
            let heard = self.system.music.contains(&omake::track_file(i));
            *t = u8::from(vm.var_b(omake::MUSIC_FIRST + i) == 1 || heard);
        }
        for (t, b) in saved.tabs.iter_mut().zip(omake::TAB_BYTES) {
            *t = vm.var_b(b);
        }
        if !self.audio.is_playing(Bus::Music, 0) {
            let _ = self.play_music(omake::BGM);
        }
        Some(Screen::Omake(Box::new(Omake::open(
            mode, areas, dialog, saved,
        ))))
    }

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
        effects::sweep(&mut self.scene);
        // With the scene skip setting on the original does nothing for
        // these calls (the arguments are taken and that is all): the
        // companion's load, face, show, fade and timelines, every standing
        // and CG model call, the strip wipe, the shake, the voice-driven
        // mouth list, and the notice calls that queue, show or clear the
        // queue. The window calls of `Game.10`, the notices' take-down and
        // busy check, and the rest run as usual.
        if self.scene_skip() {
            let skipped = match command {
                EMOTE => matches!(sub, 6..=9 | 11..=13 | 15..=18),
                EMOTES | EMOTE_CG | 1000 | 993 | 1007 => true,
                NOTICE => matches!(sub, 0 | 1 | 3),
                _ => false,
            };
            if skipped {
                return self.k1_done();
            }
        }
        match command {
            // The title menu; its argument 1 opens the extras page (back
            // from one of the modes).
            TITLE => {
                self.k1_title_extras = sub == 1;
                self.wait_for(Pause::Title)
            }
            EMOTE if self.kimaten1_emote(args, sub) => self.k1_done(),
            ACTION => {
                let areas = self.k1_areas("actselect.area")?;
                let b = |i: usize| vm.var_b(i);
                self.k1 = Some(Screen::Action(
                    ActSelect::open(
                        areas,
                        [b(0), b(1), b(2), b(3)],
                        sub == 1,
                        b(TUTORIAL_UNLOCKED) != 0,
                        b(TUTORIAL_SEEN) != 0,
                        b(NOTEBOOK_UNLOCKED) != 0,
                        std::array::from_fn(|i| b(super::actsel::PROGRESS_FIRST + i)),
                    )
                    .with_popups(
                        self.k1_areas("tutorial.area").unwrap_or_default(),
                        self.k1_areas("dialog.area").unwrap_or_default(),
                    ),
                ));
                self.wait_for(Pause::Kimaten1)
            }
            AREA => {
                let name = args.get(1).and_then(Value::as_str)?.to_string();
                let areas = self.k1_areas("areaselect.area")?;
                let map = self.assets.data(&name.to_ascii_uppercase()).ok()?;
                let progress: Vec<u8> = (0..super::actsel::PROGRESS_LEN)
                    .map(|i| vm.var_b(super::actsel::PROGRESS_FIRST + i))
                    .collect();
                self.k1 = Some(Screen::Area(AreaSelect::open(
                    areas,
                    map,
                    vm.var_b(NOTEBOOK_UNLOCKED) != 0,
                    &progress,
                )));
                self.wait_for(Pause::Kimaten1)
            }
            // The options the script registered, as a selection: answers
            // the number of the one picked plus one, 100 for back.
            OPTIONS => {
                let labels = std::mem::take(&mut vm.option_labels);
                let numbers = std::mem::take(&mut self.option_numbers);
                self.option_count = 0;
                let areas = self.k1_areas("areaselect.area")?;
                let entries = numbers
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (labels.get(i).cloned().unwrap_or_default(), n))
                    .collect();
                let flags = numbers
                    .iter()
                    .map(|&n| {
                        usize::try_from(n)
                            .is_ok_and(|n| vm.raw.get(exselect::FLAG_FIRST + n).copied() == Some(1))
                    })
                    .collect();
                self.k1 = Some(Screen::Ex(
                    ExSelect::open(areas, entries, vm.var_b(NOTEBOOK_UNLOCKED) != 0)
                        .with_flags(flags),
                ));
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
            // The extras: CG, scene and music modes.
            30..=32 => {
                let Some(screen) = self.k1_omake((command - 30) as usize, vm) else {
                    vm.push(omake::LEAVE);
                    return self.done();
                };
                self.k1 = Some(screen);
                self.wait_for(Pause::Kimaten1)
            }
            // The staff roll: 0 after the normal endings, 1 after the true
            // one. Answers 1 when it is over.
            STAFF => {
                let true_end = sub == 1;
                let _ = self.play_music_once(Staff::track(true_end));
                self.k1 = Some(Screen::Staff(Staff::open(true_end, vm.var_g(55).int())));
                self.wait_for(Pause::Kimaten1)
            }
            // The viewer of アンネ.
            33 => {
                let (Some(areas), Some(dialog)) = (
                    self.k1_areas("anneviewer.area"),
                    self.k1_areas("dialog.area"),
                ) else {
                    vm.push(omake::LEAVE);
                    return self.done();
                };
                let mut viewer = super::viewer::Viewer::open(areas, dialog, |i| vm.var_b(i));
                viewer.enter(&mut self.scene);
                if !self.audio.is_playing(Bus::Music, 0) {
                    let _ = self.play_music(super::viewer::BGM);
                }
                self.k1 = Some(Screen::Viewer(Box::new(viewer)));
                self.wait_for(Pause::Kimaten1)
            }
            // Apply: crossfade over the longest time given.
            EMOTES if sub == 2 && args.len() == 1 => {
                let t = std::mem::take(&mut self.character_time);
                self.handled += 1;
                Some(Ok(self.transition(t, None)))
            }
            EMOTES if self.kimaten1_emotes(args, sub, vm) => self.k1_done(),
            EMOTE_CG if self.kimaten1_cg(args, sub) => self.k1_done(),
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
            // mask: (hold, frames, rule, height, width, y, x).
            1000 => {
                let bg = self.scene.background.clone()?;
                let v: Vec<i32> = (0..7)
                    .map(|i| args.get(i).map(Value::int).unwrap_or(0))
                    .collect();
                let ms = effects::strip(&mut self.scene.effects, &bg, v.try_into().ok()?);
                if ms == 0 {
                    return self.done();
                }
                // Enter, Ctrl and a click end the wait (and the wipe) at once.
                self.k1_keywait = true;
                self.wait_for(Pause::Transition(crate::host::Transition {
                    ms: ms as u32,
                    rule: None,
                    spin: None,
                    live: true,
                }))
            }
            // The back picture fades in (0) or out (1) over eight frames;
            // 2 shows it and 3 puts it away at once.
            999 => {
                let bg = self.scene.background.clone();
                effects::overlay(&mut self.scene.effects, sub, bg.as_deref());
                self.k1_block(0);
                self.k1_done()
            }
            // A looping screen effect (soot, a waving screen, ...): the
            // type is under the action (0 starts it, 1 and 2 stop it).
            1001 => {
                if !self.scene_skip() {
                    let ty = args.first().map(Value::int).unwrap_or(0);
                    let bg = self.scene.background.clone();
                    fx::effect(&mut self.scene.effects, ty, sub, bg.as_deref());
                    if sub == 0 && ty == 6 {
                        let _ = self.play_sound("seg_025a.wav");
                    }
                    self.k1_block(0);
                }
                self.k1_done()
            }
            // The quick flashes and fades; the script waits for them.
            1002 => {
                if !self.scene_skip() {
                    let arg = args.first().map(Value::int).unwrap_or(0);
                    let bg = self.scene.background.clone();
                    fx::flash(&mut self.scene.effects, sub, arg, bg.as_deref());
                    self.k1_block(0);
                }
                self.k1_done()
            }
            // The spiral transition: the script never turns its switch on,
            // so there is nothing to play.
            989 => self.k1_done(),
            // The evidence cards (one, or two) fade in, stay and fade out.
            47 => {
                let cards: Vec<i32> = args[..args.len().saturating_sub(1)]
                    .iter()
                    .map(Value::int)
                    .collect();
                effects::evidence(&mut self.scene.effects, &cards);
                self.k1_block(0);
                self.k1_done()
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
            // (file): E-mote models change a set time into the voice line
            // (see [`super::vo`]). The story never ends the list with sub
            // 1; it drops what is left.
            1007 => {
                self.vo_steps = match (sub, args.first().and_then(Value::as_str)) {
                    (0, Some(file)) => self
                        .assets
                        .data(&file.to_ascii_uppercase())
                        .map(|b| super::vo::parse(&b))
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
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
            // and 35.2. 1003 takes no sub command in the engine (it calls
            // one fixed method of the game object, whatever the script
            // passes) and only the library function that starts the
            // screen-skip setup uses it.
            1003 | 1006 | 1010 | 1011 | 10008 | 10009 => self.done(),
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
