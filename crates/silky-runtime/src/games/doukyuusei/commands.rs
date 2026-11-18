//! `Game` commands of 同級生 Remake, read from its executable. The game
//! numbers its commands its own way; see `docs/games/doukyuusei.md`.

use silky_format::map::MapFile;
use silky_vm::{Flow, Vm};

use super::{shown_text, Panel};
use crate::games::Call;
use crate::host::{host_error, Runtime};

/// `Game.24`: sub 0 (line id) takes a line of the script's text table for
/// the next `MesWnd.3`, sub 1 (name id) names the speaker from
/// `NAMES_<lang>.MAP`, sub 2 loads the script's table.
pub const TEXT: i32 = 24;
/// `Game.28(0)`: the title menu over `title_bg_<lang>.akb` (logo
/// included); returns 0 for a new game, 257..=260 for the extras.
pub const TITLE: i32 = 28;
/// `Game.22(name)`: the hero speaks; the plate shows the name entered at
/// the start (the executable reads its own copy, not the argument).
pub const HERO_SPEAKS: i32 = 22;
/// `Game.16(set)`: the room screen over the current background (see
/// [`super::room`]); `set` picks the button set. Returns verb × 256 +
/// object.
pub const ROOM: i32 = 16;
/// `Game.29(kind)`: the hours dial (see [`super::hours`]); kind 0 asks
/// how long to sleep, 1 how far to wind back, 2 how far to skip forward.
/// Returns the hours, 0 for back.
pub const HOURS: i32 = 29;
/// `Game.20(..., sub)`: the walking maps (see [`super::town`]). The
/// program takes its values from the last one back.
pub const TOWN: i32 = 20;
/// `Game.25()`: writes the calendar into the raw area (see
/// [`super::calendar`]).
pub const CALENDAR: i32 = 25;
/// `Game.50` to `Game.53`: the extras screens (the program runs one, then
/// the one it hands on to, until -1), also reached from the hero's
/// computer. Not done yet: they close at once, returning -1.
pub const EXTRAS: std::ops::RangeInclusive<i32> = 50..=53;
/// `Game.10(sub)`: sub 28 puts the status plate up, 29 takes it down, 30
/// draws it again (see [`super::plate`]); the other subs are not read
/// yet.
pub const SCREEN: i32 = 10;

/// `Game.13(frames, effect, picture)` with sub 1: lays a picture the
/// size of the screen (the faces coming in before the `Anime` class takes
/// them over) into the background over `frames`. The program leaves it
/// out when the settings skip scene effects, which they do not here.
pub const PICTURE_IN: i32 = 13;
/// `Game.17(file, 0)`: the `.area` file of the room screen's hot spots
/// (see [`super::room`]), kept until the next one.
pub const AREA: i32 = 17;
/// `Game.34(index, group, name)`: the face of a conversation changes;
/// starts `Anime` program `index` of `group` unless it already runs (see
/// [`crate::anime`]). The name is not used.
pub const FACE: i32 = 34;

impl Runtime {
    /// The status plate's values now, in 同級生 Remake.
    pub fn plate(&self, vm: &Vm) -> Option<super::plate::Plate> {
        (self.assets.game().title == silky_format::game::Title::Doukyuusei && self.scene.plate)
            .then(|| super::plate::Plate::read(vm, super::plate::GAME_MODE))
    }

    pub(crate) fn doukyuusei_game(
        &mut self,
        vm: &mut Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        match (c.command, c.sub) {
            (TEXT, 0) => {
                let id = c.int(0);
                let script = vm.script_name().unwrap_or("").to_ascii_lowercase();
                self.message_script = script.clone();
                let hero = self.hero_name();
                let line = match self.doukyuusei_table(&script) {
                    Ok(t) => t.get(0, id as u32).map(|l| shown_text(l, &hero)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                };
                let Some(text) = line else {
                    return Some(Err(host_error(format!("{script}: no text line {id}"))));
                };
                self.pending_text = Some(text);
                self.last_text = Some(id.max(0) as usize);
                self.done()
            }
            (TEXT, 1) => {
                let id = c.int(0);
                let name = format!("names_{}.map", self.config.language.suffix());
                self.scene.speaker_name = self
                    .assets
                    .data(&name)
                    .ok()
                    .and_then(|b| MapFile::parse(&b).ok())
                    .and_then(|m| m.get(0, id as u32).map(str::to_string));
                self.done()
            }
            (TITLE, _) => {
                let lang = super::picture_suffix(self.config.language);
                self.scene.background = Some(format!("title_bg_{lang}.akb"));
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Title)
            }
            (ROOM, _) => {
                let bg = self.scene.background.clone();
                let area = self.scene.room_area.clone();
                match super::room::Room::load(
                    &self.assets,
                    area.as_deref(),
                    bg.as_deref(),
                    // The one value is taken as the sub command.
                    c.sub,
                    self.config.language,
                ) {
                    Ok(r) => {
                        self.panel = Some(Panel::Room(r));
                        self.scene.plate = true;
                    }
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (HOURS, _) => {
                match super::hours::Hours::load(&self.assets, c.sub) {
                    Ok(h) => self.panel = Some(Panel::Hours(h)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (TOWN, sub) => self.doukyuusei_town(vm, c, sub),
            (SCREEN, 28 | 29) => {
                self.scene.plate = c.sub == 28;
                self.done()
            }
            (SCREEN, 30) => self.done(),
            // The speaker's icon: 15 shows icon n (from 1), 16 takes it
            // away. Values below 10 do nothing in the program.
            (SCREEN, 15) => {
                let n = (back_int(c, 0) - 1).max(0);
                self.scene.speaker_icon = (n < 43).then_some(n as u32);
                self.done()
            }
            (SCREEN, 16) => {
                self.scene.speaker_icon = None;
                self.done()
            }
            (SCREEN, sub) if sub < 10 => self.done(),
            (AREA, 0) => {
                self.scene.room_area = back_str(c, 0);
                self.done()
            }
            (AREA, 1) => {
                let area = self.scene.room_area.clone();
                match super::room::Room::plain(&self.assets, area.as_deref(), self.config.language)
                {
                    Ok(r) => self.panel = Some(Panel::Room(r)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (PICTURE_IN, 1) => {
                let Some(name) = back_str(c, 0) else {
                    return None;
                };
                let frames = back_int(c, 2);
                let slot = self
                    .scene
                    .overlays
                    .keys()
                    .next_back()
                    .map_or(0, |k| k + 1)
                    .max(1000);
                self.scene.overlays.insert(slot, name);
                self.handled += 1;
                Some(Ok(self.transition(frames, None)))
            }
            (FACE, _) => {
                let (index, group) = (back_int(c, 2), back_int(c, 1));
                if !self.scene.anime.running(group, index) {
                    let ax = match self.ax() {
                        Ok(ax) => ax,
                        Err(e) => return Some(Err(host_error(e.to_string()))),
                    };
                    let Some(ax) = ax else {
                        return Some(Err(host_error("Game.34 before an .ax file")));
                    };
                    self.scene.anime.stop_group(group);
                    self.scene.anime.set(&ax, group, index);
                    self.scene.anime.start(group, index);
                }
                self.done()
            }
            (command, _) if EXTRAS.contains(&command) => {
                *self.unhandled.entry(format!("Game.{command}")).or_default() += 1;
                vm.push(-1);
                self.done()
            }
            (CALENDAR, _) => {
                super::calendar::write(&mut vm.raw);
                self.done()
            }
            (HERO_SPEAKS, _) => {
                self.scene.speaker_name = Some(self.hero_name());
                self.done()
            }
            (TEXT, 2) => {
                let script = vm.script_name().unwrap_or("").to_ascii_lowercase();
                if let Err(e) = self.doukyuusei_table(&script) {
                    return Some(Err(host_error(e.to_string())));
                }
                self.done()
            }
            _ => None,
        }
    }
}

/// Value `k` of a call counted from the last one back (the order the
/// program takes them in).
fn back(c: &Call, k: usize) -> Option<&silky_vm::Value> {
    c.args.len().checked_sub(k + 1).and_then(|i| c.args.get(i))
}

fn back_int(c: &Call, k: usize) -> i32 {
    back(c, k).map(silky_vm::Value::int).unwrap_or(0)
}

fn back_str(c: &Call, k: usize) -> Option<String> {
    back(c, k)
        .and_then(silky_vm::Value::as_str)
        .map(str::to_string)
}

impl Runtime {
    /// `Game.20`: the walking maps.
    fn doukyuusei_town(
        &mut self,
        vm: &mut Vm,
        c: &Call,
        sub: i32,
    ) -> Option<silky_vm::Result<Flow>> {
        use super::town::Town;
        let town = self.town.get_or_insert_with(Town::default);
        let walker = back_int(c, 0).max(0) as usize;
        let failed = |e: crate::Error| Some(Err(host_error(e.to_string())));
        match sub {
            0 | 4 | 12 => {
                let Some(name) = back_str(c, 0) else {
                    return Some(Err(host_error(format!("Game.20({sub}) without a file"))));
                };
                let data = match self.assets.data(&name) {
                    Ok(d) => d,
                    Err(e) => return failed(e),
                };
                let town = self.town.as_mut().unwrap();
                let r = match sub {
                    0 => town.load_grid(&data),
                    4 => town.load_walkers(data.to_vec()),
                    _ => town.load_places(&data, back_int(c, 1)),
                };
                if let Err(e) = r {
                    return failed(e);
                }
            }
            1 => {
                let (start, w, facing) = (back_int(c, 0), back_int(c, 1), back_int(c, 2));
                town.place(start, w.max(0) as usize, facing, &mut vm.raw);
            }
            5 => town.set_program(walker, back_int(c, 1)),
            7 => town.set_flags(walker, back_int(c, 1)),
            14 => {
                town.step_walker(walker, back_int(c, 1));
                self.refresh_town();
            }
            20 => town.follow(walker, back_int(c, 1)),
            22 => town.set_facing(walker, back_int(c, 1)),
            25 => {
                let slot = back_int(c, 1);
                if let (Some(name), 0..=1) = (back_str(c, 0), slot) {
                    town.sheets[slot as usize] = Some(name);
                }
            }
            27 | 28 => {
                let name = back_str(c, 0);
                if sub == 27 {
                    town.picture = name;
                } else {
                    town.over = name;
                }
            }
            31 => town.restore(walker, &mut vm.raw),
            // Draw the map into the background.
            2 | 3 | 6 | 10 | 11 => {
                self.scene.town = Some(self.town.as_ref().unwrap().view());
                self.scene.background = None;
            }
            // A place the hero stands in, as after a step.
            13 => {
                let id = self.town.as_mut().unwrap().find_place();
                vm.push(id as i32);
            }
            // One step of the walkers (8, 9); a walk to a target (17)
            // that is not read yet; something of the program's own (33).
            8 | 9 | 17 | 33 => vm.push(0),
            30 => {
                self.scene.town = Some(self.town.as_ref().unwrap().view());
                self.scene.background = None;
                self.scene.text = None;
                self.panel = Some(super::Panel::Town);
                self.scene.plate = true;
                return self.wait_for(crate::host::Pause::Panel);
            }
            // The place names drawn over the map (only in a mode of the
            // settings not offered) and the program's own tile drawing.
            29 | 32 => {}
            34 => {
                self.town = None;
                self.scene.town = None;
            }
            _ => return None,
        }
        self.done()
    }

    /// The map drawn again after its walkers moved, when it is shown.
    pub(crate) fn refresh_town(&mut self) {
        if let (Some(_), Some(t)) = (&self.scene.town, &self.town) {
            self.scene.town = Some(t.view());
        }
    }

    /// A step of the walk (`Game.20(30)`); the place reached, if any.
    pub fn town_tick(&mut self, vm: &mut Vm, input: &super::town::Input) -> Option<i32> {
        let t = self.town.as_mut()?;
        let found = t.tick(input, &mut vm.raw);
        self.refresh_town();
        found.map(i32::from)
    }
}
