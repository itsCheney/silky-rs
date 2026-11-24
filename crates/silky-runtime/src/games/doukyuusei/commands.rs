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
/// The value variable (g18) the scripts read the results of `Game.20` subs
/// 13 and 17 from; they write it themselves.
const RESULT: usize = 18;
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
/// `Game.23(name)`: the file name a script asks for in the language of
/// the game (`x.akb` -> `x_en.akb`, `_ch`, `_tw`), the name itself when
/// that picture does not exist. Takes no sub command (its one value is a
/// string), which is why the call is counted without one. The result goes
/// to the call that loads the picture.
pub const LOCALIZE_NAME: i32 = 23;
/// `Game.39(sub)`: the train of Misa's scenes (see [`super::train`]): sub
/// 0 makes the picture object, 1 takes it away, 2 brings it in and 3 sends
/// it out (the script waits), 4 puts it at its stopping place, 5 draws
/// `y04bb.akb` into the background.
pub const OVERLAY: i32 = 39;
/// `Game.33(..)`: the pan over a tall picture in sprite layer 1 (see
/// [`super::pan`]); the script waits, nothing is returned. The first value
/// is taken as the sub command.
pub const PAN: i32 = 33;
/// `Game.37(n)`: the "comment unlocked" pop-up (see [`super::unlocked`]);
/// `n` is the number of the comment picture. The script waits; nothing is
/// returned.
pub const UNLOCKED: i32 = 37;
/// The byte that keeps the status plate up when the message window comes
/// back (`Game.10(11)`).
const PLATE_KEEP: usize = 4084;

/// `Game.18()`: the name entry (see [`super::name`]); the name is kept in
/// the raw area, nothing is returned.
pub const NAME: i32 = 18;
/// `Game.30(kind)`: the modal notice (see [`super::notice`]); nothing is
/// returned.
pub const NOTICE: i32 = 30;
/// `Game.31()`: the petal intro of the very first start (see
/// [`super::intro`]); the script waits, nothing is returned.
pub const INTRO: i32 = 31;
/// `Game.5`: puts the name into the speaker plate.
pub const NAME_PLATE: i32 = 5;

/// `Game.13`: picture objects over the background, by id (the library
/// only uses id 0). Sub 0 `(frames, alpha, picture, id)` puts a picture
/// the size of the screen up (a standing character) at `alpha` percent;
/// sub 1 `(frames, id, picture)` lays a picture of the same size onto
/// object `id` (the faces), and does nothing without one; sub 4
/// `(frames)` fades every object out; sub 2 forgets those that faded out.
/// The fades take `frames` in the original while the program goes on;
/// here they are a cross-fade of the screen.
pub const PICTURE_IN: i32 = 13;
/// `Game.17(file, 0)`: the `.area` file of the room screen's hot spots
/// (see [`super::room`]), kept until the next one.
pub const AREA: i32 = 17;
/// `Game.34(index, group, name)`: the face of a conversation changes;
/// starts `Anime` program `index` of `group` unless it already runs (see
/// [`crate::anime`]). The name is not used.
pub const FACE: i32 = 34;
/// `Game.36(sub)`: the character picker (see [`super::picker`]); returns
/// the girl picked from 1, or 15 for back (sub 0 only).
pub const PICKER: i32 = 36;
/// `Game.27(sub)`: the game's mouse pointer.
pub const POINTER: i32 = 27;
/// `Game.35(sub)`: scene skip marks.
pub const SCENE_MARKS: i32 = 35;
/// `Game.50`: the album (see [`super::album`]); returns -1 to leave, 100
/// for another page of the extras, else a picture to show.
pub const ALBUM: i32 = 50;
/// `Game.51`: the memory reel (see [`super::reel`]); returns -1 to leave,
/// 100 for another page of the extras, 0 to show a scene.
pub const REEL: i32 = 51;
/// `Game.52`: the music mode (see [`super::music`]); returns -1 to leave,
/// 100 for another page of the extras.
pub const MUSIC: i32 = 52;
/// `Game.53`: the comments page (see [`super::comment`]); returns -1 to
/// leave, 100 for another page of the extras.
pub const COMMENT: i32 = 53;

impl Runtime {
    /// 同級生 Remake: the easy mode is set (the settings' game mode; see
    /// [`super::plate::game_mode`]).
    pub fn easy_mode(&self) -> bool {
        !self.system.settings.classic_mode
    }

    /// The status plate's values now, in 同級生 Remake.
    pub fn plate(&self, vm: &Vm) -> Option<super::plate::Plate> {
        (self.assets.game().title == silky_format::game::Title::Doukyuusei && self.scene.plate)
            .then(|| super::plate::Plate::read(vm, super::plate::game_mode(!self.easy_mode())))
    }

    /// The intro is over: its colour picture is drawn over the whole
    /// background for good.
    pub(crate) fn end_intro(&mut self) {
        self.then = None;
        if let Some(intro) = self.scene.intro.take() {
            self.scene.background = Some(intro.background);
            self.scene.background_offset = (0, 0);
            self.scene.background_zoom = 0;
            self.scene.overlays.clear();
        }
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
                let hero = self.hero_name(vm);
                let stem = self.table_stem(&script);
                let line = match self.doukyuusei_table(&stem) {
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
            (NAME, _) => {
                let areas = crate::screens::area_file(&self.assets, "nameinput.area");
                self.panel = Some(Panel::Name(super::name::Name::new(
                    self.config.language,
                    areas,
                )));
                // The page shows the background the script put up; the
                // screen stays covered under it, as the scripts find
                // afterwards.
                self.cover_under_name = self.scene.cover.take();
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (NOTICE, kind) => {
                // A kind without a picture does nothing.
                let Some(n) = super::notice::Notice::new(kind) else {
                    return self.done();
                };
                self.panel = Some(Panel::Notice(n));
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (INTRO, _) => {
                let bg = format!(
                    "{}_{}.akb",
                    super::intro::BACKGROUND,
                    super::picture_suffix(self.config.language)
                );
                let rule = self
                    .assets
                    .data(&format!("{:02}.bin", super::intro::RULE))
                    .ok()
                    .map(std::sync::Arc::new);
                self.scene.intro = Some(super::intro::Intro::new(bg, rule));
                self.scene.text = None;
                self.then = Some(crate::host::FollowUp::EndIntro);
                self.handled += 1;
                let flow = self.transition(super::intro::FRAMES, None);
                match &mut self.pause {
                    Some(crate::host::Pause::Transition(t)) => t.live = true,
                    // Effects turned off: it is over at once.
                    _ => self.end_intro(),
                }
                Some(Ok(flow))
            }
            (NAME_PLATE, _) => {
                self.scene.speaker_name = Some(self.hero_name(vm));
                self.done()
            }
            (TOWN, sub) => self.doukyuusei_town(vm, c, sub),
            (PICKER, sub) => {
                match super::picker::Picker::load(&self.assets, sub, &vm.bytes) {
                    Ok(p) => self.panel = Some(Panel::Picker(p)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.wait_for(crate::host::Pause::Panel)
            }
            (ALBUM, _) => {
                let g = |i: usize| vm.var_g(i).int();
                match super::album::Album::load(&self.assets, &vm.bytes, g) {
                    Ok(a) => self.panel = Some(Panel::Album(a)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (REEL, _) => {
                let g = |i: usize| vm.var_g(i).int();
                match super::reel::Reel::load(&self.assets, &vm.bytes, g) {
                    Ok(r) => self.panel = Some(Panel::Reel(r)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (MUSIC, _) => {
                self.stop_music();
                let st = &self.system.settings;
                match super::music::Music::load(&self.assets, &vm.bytes, st.music, st.music_on) {
                    Ok(m) => self.panel = Some(Panel::Music(m)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (COMMENT, _) => {
                match super::comment::Comment::load(&self.assets, &vm.bytes) {
                    Ok(c) => self.panel = Some(Panel::Comment(c)),
                    Err(e) => return Some(Err(host_error(e.to_string()))),
                }
                self.scene.text = None;
                self.wait_for(crate::host::Pause::Panel)
            }
            (SCREEN, 28 | 29) => {
                self.scene.plate = c.sub == 28;
                self.done()
            }
            (SCREEN, 30) => self.done(),
            // The text on the message window goes; the window stays.
            (SCREEN, 10) => {
                if let Some(text) = &mut self.scene.text {
                    text.clear();
                }
                self.done()
            }
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
            // The message window comes back; the status plate goes down
            // unless byte 4084 keeps it. The window is shown here whenever
            // a line is up.
            (SCREEN, 11) => {
                if vm.bytes.get(PLATE_KEEP).copied().unwrap_or(0) == 0 {
                    self.scene.plate = false;
                }
                self.done()
            }
            // 14 puts the window's layers away, which here is the same as
            // no line being up. Subs 12, 13 and 17 to 27 (18 included) do
            // nothing in the program, nor do those below 10.
            (SCREEN, 12..=14 | 17..=27) => self.done(),
            (SCREEN, sub) if sub < 10 => self.done(),
            (UNLOCKED, n) => {
                // A number without a picture does nothing.
                let Some(u) = super::unlocked::Unlocked::new(n) else {
                    return self.done();
                };
                // The message window stays up under the pop-up.
                self.panel = Some(Panel::Unlocked(u));
                self.wait_for(crate::host::Pause::Panel)
            }
            (PAN, _) => {
                // The first value was taken as the sub command.
                let mut values: Vec<i32> = c.args.iter().map(silky_vm::Value::int).collect();
                if c.sub >= 0 {
                    values.push(c.sub);
                }
                let fast = self.system.settings.effect_speed != 0;
                let layer = super::pan::LAYER;
                let Some(mut pan) = super::pan::Pan::new(&values, fast) else {
                    return self.done();
                };
                if !self.scene.layers.contains_key(&layer) {
                    return self.done();
                }
                // The window ends at the end position; the wait shows it
                // on its way.
                self.scene.layer_view.insert(layer, pan.to);
                let ms = pan.total_ms();
                if ms == 0 {
                    return self.done();
                }
                pan.progress(0, ms);
                self.scene.pan = Some(pan);
                self.pause = Some(crate::host::Pause::Transition(crate::host::Transition {
                    ms,
                    rule: None,
                    spin: None,
                    live: true,
                }));
                self.handled += 1;
                Some(Ok(Flow::Yield))
            }
            (OVERLAY, 0) => {
                self.scene.train = Some(super::train::Train::new());
                self.done()
            }
            (OVERLAY, 1) => {
                self.scene.train = None;
                self.done()
            }
            // The moves wait for the train to get there; the frontend
            // draws it on its way (a live wait). Without the object they
            // do nothing.
            (OVERLAY, sub @ (2 | 3)) => {
                let Some(train) = &mut self.scene.train else {
                    return self.done();
                };
                let mv = if sub == 2 {
                    train.arrive();
                    super::train::Move::Arrive
                } else {
                    train.leave();
                    super::train::Move::Leave
                };
                self.handled += 1;
                let flow = self.transition(mv.frames(), None);
                if let Some(crate::host::Pause::Transition(t)) = &mut self.pause {
                    t.live = true;
                }
                Some(Ok(flow))
            }
            (OVERLAY, 4) => {
                if let Some(train) = &mut self.scene.train {
                    train.park();
                }
                self.done()
            }
            (OVERLAY, 5) => {
                self.scene.background = Some(super::train::BACKGROUND.to_string());
                self.done()
            }
            (LOCALIZE_NAME, _) if matches!(c.args.last(), Some(silky_vm::Value::Str(_))) => {
                let len = c.args.len();
                let name = c.str(len - 1)?;
                for v in &c.args[..len - 1] {
                    vm.push(v.clone());
                }
                let local = crate::assets::localized_name_with(
                    &name,
                    super::picture_suffix(self.config.language),
                );
                let found = self.assets.exists(silky_format::game::Kind::Layer, &local);
                vm.push(if found { local } else { name }.as_str());
                self.done()
            }
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
            (PICTURE_IN, 0) => {
                let name = back_str(c, 1)?;
                let (id, alpha, frames) = (back_int(c, 0), back_int(c, 2), back_int(c, 3));
                let face = None;
                let picture = crate::scene::Picture { name, face, alpha };
                self.scene.pictures.insert(id, picture);
                self.handled += 1;
                Some(Ok(self.transition(frames, None)))
            }
            (PICTURE_IN, 1) => {
                let name = back_str(c, 0)?;
                let (id, frames) = (back_int(c, 1), back_int(c, 2));
                self.handled += 1;
                let Some(p) = self.scene.pictures.get_mut(&id) else {
                    return Some(Ok(Flow::Continue));
                };
                p.face = Some(name);
                Some(Ok(self.transition(frames, None)))
            }
            (PICTURE_IN, 4) => {
                self.handled += 1;
                if self.scene.pictures.is_empty() {
                    return Some(Ok(Flow::Continue));
                }
                self.scene.pictures.clear();
                Some(Ok(self.transition(back_int(c, 0), None)))
            }
            (PICTURE_IN, 2) => self.done(),
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
            // The game's own mouse pointer on (0) and off (1), and the
            // marks of scenes that the "skip scenes" setting jumps over
            // (35: 0 marks, 1 clears); neither changes what is shown here.
            (POINTER | SCENE_MARKS, _) => self.done(),
            (HERO_SPEAKS, _) => {
                self.scene.speaker_name = Some(self.hero_name(vm));
                self.done()
            }
            (TEXT, 2) => {
                // The script names its table itself; a variant of a script
                // names the plain one's.
                let name = match back_str(c, 0) {
                    Some(n) => n.to_ascii_lowercase(),
                    None => vm.script_name().unwrap_or("").to_ascii_lowercase(),
                };
                self.table_stem = Some(name.clone());
                if let Err(e) = self.doukyuusei_table(&name) {
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
        use super::town::{labels_shown, step_ms, Town};
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
                match sub {
                    0 => town.log.grid = Some(name),
                    4 => town.log.ccd = Some(name),
                    _ => town.log.places = Some((name, back_int(c, 1))),
                }
            }
            1 => {
                let (start, w, facing) = (back_int(c, 0), back_int(c, 1), back_int(c, 2));
                town.place(start, w.max(0) as usize, facing, &mut vm.raw);
                if let Some(p) = town.log.placed.get_mut(w.max(0) as usize) {
                    *p = true;
                }
            }
            5 => town.set_program(walker, back_int(c, 1)),
            7 => {
                town.set_flags(walker, back_int(c, 1));
                if let Some(f) = town.log.flags.get_mut(walker) {
                    *f = Some(back_int(c, 1) as u8);
                }
            }
            14 => {
                town.step_walker(walker, back_int(c, 1));
                self.refresh_town(&vm.bytes);
            }
            // Walk to a target (15), end the walk (16), still walking (17).
            15 => {
                let (x, y) = (back_int(c, 1) as u16, back_int(c, 2) as u16);
                town.walk_to(walker, x, y, |i| vm.var_b(i as usize) != 0);
            }
            16 => town.end_walk(),
            17 => vm.set_g(RESULT, if town.walking() { 0xffff } else { 0 }),
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
                    town.load_picture(name, vm);
                    self.refresh_town(&vm.bytes);
                } else {
                    town.over = name;
                }
            }
            31 => town.restore(walker, &mut vm.raw),
            // Draw the map into the background.
            2 | 3 | 6 | 10 | 11 => {
                self.scene.town = Some(
                    self.town
                        .as_ref()
                        .unwrap()
                        .view(labels_shown(&vm.bytes, self.easy_mode())),
                );
                self.scene.background = None;
            }
            // A place the hero stands in, as after a step.
            13 => {
                let t = self.town.as_mut().unwrap();
                let id = t.reached.take().unwrap_or_else(|| t.find_place());
                vm.set_g(RESULT, id as i32);
            }
            // One step of every walker; the place reached waits for sub 13.
            9 => {
                let input = super::town::Input {
                    hold: vm.var_b(super::town::CANCEL_VAR) != 0,
                    ..Default::default()
                };
                town.reached = Some(town.tick(&input, &mut vm.raw).unwrap_or(0xffff));
                self.refresh_town(&vm.bytes);
                vm.push(0);
            }
            // The frame wait: a step's time since the last tick.
            26 => {
                let ms = step_ms(self.system.settings.map_speed);
                return self.wait_for(crate::host::Pause::Transition(crate::host::Transition {
                    ms: ms as u32,
                    rule: None,
                    spin: None,
                    live: true,
                }));
            }
            // Something of the program's own.
            8 | 33 => vm.push(0),
            30 => {
                self.scene.town = Some(
                    self.town
                        .as_ref()
                        .unwrap()
                        .view(labels_shown(&vm.bytes, self.easy_mode())),
                );
                self.scene.background = None;
                self.scene.text = None;
                self.panel = Some(super::Panel::Town);
                self.scene.plate = true;
                return self.wait_for(crate::host::Pause::Panel);
            }
            // The program's own tile drawing; 24 (a `.pal` file) is not one
            // of this build's sub commands.
            24 | 29 => {}
            // The place labels of game mode 1, filled again for the story now.
            32 => {
                town.refill_labels(vm);
                self.refresh_town(&vm.bytes);
            }
            34 => {
                self.town = None;
                self.scene.town = None;
            }
            _ => return None,
        }
        self.done()
    }

    /// What goes when the flag list's jump starts an event's script: the
    /// walking map's state and the screens of the menu, the actions waiting
    /// for an answer, and the message that was up. The picture (the map
    /// stays on screen until the script covers it), the music and the text
    /// table stay for the script to change.
    pub fn leave_for_event(&mut self) {
        self.town = None;
        self.panel = None;
        self.map = None;
        self.actions.clear();
        self.pause = None;
        self.scene.text = None;
        self.last_text = None;
        self.option_count = 0;
        self.choices.clear();
        self.choice_status.clear();
        self.lip = None;
        self.jolts.clear();
        self.clicked = false;
    }

    /// The map drawn again after its walkers moved, when it is shown.
    pub(crate) fn refresh_town(&mut self, bytes: &[u8]) {
        use super::town::labels_shown;
        if let (Some(_), Some(t)) = (&self.scene.town, &self.town) {
            self.scene.town = Some(t.view(labels_shown(bytes, self.easy_mode())));
        }
    }

    /// Whether an automatic walk is going.
    pub fn town_walking(&self) -> bool {
        self.town.as_ref().is_some_and(|t| t.walking())
    }

    /// A click or key during the automatic walk (see `Town::cancel_walk`).
    pub fn cancel_town_walk(&mut self) {
        if let Some(t) = &mut self.town {
            t.cancel_walk();
        }
    }

    /// A step of the walk (`Game.20(30)`); the place reached, if any.
    pub fn town_tick(&mut self, vm: &mut Vm, input: &super::town::Input) -> Option<i32> {
        let t = self.town.as_mut()?;
        let found = t.tick(input, &mut vm.raw);
        self.refresh_town(&vm.bytes);
        found.map(i32::from)
    }
}
