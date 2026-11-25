//! `Game` commands of なないろリンカネーション, read from its executable.

use silky_vm::{Flow, Vm};

use super::cg::ROWS;
use super::cgmode::CgMode;
use super::mode::{self, Mode};
use super::music::{self, MusicMode};
use super::{figures, stage};
use crate::audio::Bus;
use crate::games::kimaten1::Screen;
use crate::games::Call;
use crate::host::{Jolt, Pause, Runtime};
use crate::scene::Character;

/// Quit: asks, and ends the game on yes.
const QUIT: i32 = 3;
/// Standing characters.
const CHARACTER: i32 = 13;
/// CG registry.
const CG: i32 = 14;
/// Speaker name: `15(name)`.
const SPEAKER: i32 = 15;
/// Message window portrait.
const PORTRAIT: i32 = 12;
/// Title menu; returns the item picked, with the codes of the common
/// title menu.
const TITLE: i32 = 18;
/// `19(value, end, start)`: fills byte flags `start..end`.
const FILL: i32 = 19;
/// Capture of the screen for the CG registry.
const CAPTURE: i32 = 22;
/// Auto save.
const AUTO_SAVE: i32 = 17;
/// Chapter end.
const CHAPTER_END: i32 = 26;
/// Initialisation of the message window.
const INIT: i32 = 27;
/// Notice of the trial version.
const TRIAL: i32 = 40;
/// The CG mode.
const CG_MODE: i32 = 30;
/// The scene mode.
const SCENE_MODE: i32 = 31;
/// The music mode.
const MUSIC_MODE: i32 = 32;

/// The script variables the modes share (G bank): the resume flag of the CG
/// screen, the page or variant, the result of the screen (the picture to
/// resume on) and the screen's number (0 CG, 1 scene, 2 music).
const SCRIPT_CG: usize = 16;
const SCRIPT_PAGE: usize = 17;
const SCRIPT_RESULT: usize = 18;
const SCRIPT_SCREEN: usize = 19;

/// The script's number of a screen.
fn script_screen(mode: Mode) -> i32 {
    match mode {
        Mode::Cg => 0,
        Mode::Scene => 1,
        Mode::Music => 2,
    }
}

impl Runtime {
    /// Opens one of the game's own mode screens (`Game.30` and `Game.32`,
    /// and the tabs that go from one to the other). The BGM left playing is
    /// faded out by the music mode (the screen's own start); a mode
    /// opened over silence starts the mode BGM.
    pub(crate) fn nanairo_open_mode(&mut self, vm: &mut Vm, mode: Mode) {
        // Whether each tab is open, in the order of the strip.
        let tabs = [Mode::Cg, Mode::Music, Mode::Scene].map(|m| vm.var_b(m.tab_byte()));
        let screen = match mode {
            Mode::Cg => {
                self.cancelled = false;
                let flags: Vec<u8> = ROWS.iter().map(|r| vm.var_b(r.flag as usize)).collect();
                // With the resume flag at 1 the screen opens on the picker of
                // the picture in the result variable, and clears the flag.
                let first = (vm.var_g(SCRIPT_CG).int() == 1)
                    .then(|| vm.var_g(SCRIPT_RESULT).int().max(0) as usize);
                if first.is_some() {
                    vm.set_g(SCRIPT_CG, 0);
                }
                Screen::NanairoCg(Box::new(CgMode::open(&flags, tabs, first)))
            }
            Mode::Scene => {
                use super::scene::{SceneMode, FLAG_FIRST, SCENES};
                let mut flags = [0u8; SCENES];
                for (i, f) in flags.iter_mut().enumerate() {
                    *f = vm.var_b(FLAG_FIRST + i);
                }
                // The page of the scene picked last is kept in a script
                // variable.
                let page = vm.var_g(SCRIPT_PAGE).int().max(0) as usize;
                Screen::NanairoScene(Box::new(SceneMode::open(flags, tabs, page)))
            }
            Mode::Music => {
                let mut tracks = [0u8; music::ROWS];
                for (t, (flag, _)) in tracks.iter_mut().zip(music::TRACKS) {
                    *t = vm.var_b(flag);
                }
                let st = &self.system.settings;
                let screen = MusicMode::open(tracks, tabs, st.music_on, st.music);
                // What plays fades out over 32 steps down to -50 dB and
                // then stops; with the music off it is stopped at once.
                if self.system.settings.music_on {
                    self.audio.fade_out(Bus::Music, 0, 512);
                } else {
                    self.stop_music();
                }
                Screen::NanairoMusic(Box::new(screen))
            }
        };
        self.k1 = Some(screen);
        // Every pass of the mode loop starts the mode BGM when nothing
        // plays, so a mode opened over silence plays it at once.
        self.nanairo_mode_bgm();
    }

    /// Asks for an auto save, unless the player switched auto save off in
    /// the configuration (it is on in a new install). Also asked for by
    /// `Game.17` and the start of the calendar.
    pub(crate) fn nanairo_auto_save(&mut self) {
        if self.config_value("/Config/AutoSave/Enable", 1) != 0 {
            self.auto_save = true;
        }
    }

    /// Starts the BGM of the mode screens when the music is on and the
    /// music channel is silent. It is played as the music mode plays a
    /// track (once); every pass of the loop starts it again after it ends.
    pub(crate) fn nanairo_mode_bgm(&mut self) {
        if self.system.settings.music_on && !self.audio.is_playing(Bus::Music, 0) {
            let _ = self.play_music_once(music::BGM);
        }
    }

    /// What the scene mode leaves in the script's variables when the mode
    /// loop acts on its answer: a tab press clears the page variable and
    /// the script's screen number follows the screen; a scene picked is
    /// replayed by the script, which opens the screen again on the same
    /// page.
    pub(crate) fn nanairo_mode_left(
        &mut self,
        vm: &mut Vm,
        from: Mode,
        page: usize,
        next: mode::Next,
    ) {
        match next {
            mode::Next::Go(to) => {
                vm.set_g(SCRIPT_PAGE, 0);
                vm.set_g(SCRIPT_SCREEN, script_screen(to));
            }
            mode::Next::Leave(v) if from == Mode::Scene && v >= 0 => {
                vm.set_g(SCRIPT_PAGE, page as i32);
            }
            mode::Next::Leave(_) => {}
        }
    }

    /// This game's own commands. `None` for the others.
    pub(crate) fn nanairo_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        // Event pictures, effect pictures and scene movies.
        if matches!(c.command, 22 | 23 | 1001..=1003 | 1005 | 1006)
            && !(c.command == 22 && c.sub == 0)
        {
            return self.nanairo_effects(c);
        }
        match c.command {
            TITLE => {
                crate::games::nanairo::effects::clear_all(&mut self.scene.effects);
                self.wait_for(Pause::Title)
            }
            SPEAKER => {
                self.scene.speaker_name = c.str(0);
                self.done()
            }
            QUIT => self.wait_for(Pause::Quit),
            // Saves unless the player switched auto save off (it is on in
            // a new install); the frontend writes the save at the next
            // message, as for the other games' auto save points.
            AUTO_SAVE => {
                self.nanairo_auto_save();
                self.done()
            }
            // The notice shows pictures that this release does not carry,
            // so nothing is drawn and the call returns at once.
            TRIAL => self.done(),
            CHARACTER => self.nanairo_character(c),
            PORTRAIT => self.nanairo_portrait(c),
            CG => self.nanairo_cg(c, vm),
            FILL => {
                // The call has no sub command: the last value is the
                // first flag.
                let (value, end, start) = (c.int(0), c.int(1), c.sub);
                for i in start.max(0)..end.max(0) {
                    vm.set_var_b(i as usize, value as u8);
                }
                self.done()
            }
            CAPTURE => match c.sub {
                // The screen is copied for the save picture, and the
                // picture shown is registered.
                0 => {
                    self.scene.cg.register(vm);
                    self.done()
                }
                _ => None,
            },
            CHAPTER_END => {
                // Ends the message window before the next script; the
                // effect pictures do not go on into it.
                crate::games::nanairo::effects::clear_all(&mut self.scene.effects);
                self.scene.text = None;
                self.scene.speaker_name = None;
                self.done()
            }
            INIT => {
                self.scene.speaker_name = None;
                self.done()
            }
            CG_MODE => {
                self.nanairo_open_mode(vm, Mode::Cg);
                self.wait_for(Pause::Kimaten1)
            }
            SCENE_MODE => {
                self.nanairo_open_mode(vm, Mode::Scene);
                self.wait_for(Pause::Kimaten1)
            }
            MUSIC_MODE => {
                self.nanairo_open_mode(vm, Mode::Music);
                self.wait_for(Pause::Kimaten1)
            }
            // Object of the library's start up; nothing is read from it.
            10 => self.done(),
            // The ending's message overlay and the staff roll.
            50 => self.nanairo_ending(c),
            34 => self.nanairo_staff(vm, c),
            // The calendar of a date change.
            39 => self.nanairo_calendar(vm, c),
            // Takes every standing character away at once (with the scenes
            // skipped it does nothing). The scripts never call it.
            1004 => {
                if !self.scene_skip() {
                    self.scene.characters.clear();
                }
                self.done()
            }
            // Housekeeping with no picture: 1007 empties a cache of loaded
            // objects, 1008 and 1009 (called once for each of 132 slots at
            // start up) check for and create the blank 8 by 8 thumbnail
            // files of the save folder.
            1007..=1009 => self.done(),
            _ => None,
        }
    }

    /// Standing characters.
    fn nanairo_character(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let skip = self.scene_skip();
        let veil = super::effects::kirara_up(&mut self.scene.effects);
        let factor = self.system.settings.effect_scale();
        let petals = self
            .movie_playable(super::figures::PETALS_MOVIE)
            .then_some(super::figures::PETALS_MOVIE);
        let scene = &mut self.scene;
        match c.sub {
            // (time, size, name, slot)
            0 if c.args.len() == 4 => {
                let (size, slot) = (c.int(1), c.int(3));
                let body = c.str(2)?;
                self.character_time = self.character_time.max(c.int(0));
                // A figure of another character has its own order and
                // place; the same one keeps them.
                if !scene
                    .characters
                    .get(&slot)
                    .is_some_and(|old| stage::same_character(&old.body, &body))
                {
                    scene.figures.forget(slot);
                }
                let place = scene.figures.place(slot);
                let character = Character {
                    offset: stage::shifted(slot, place, size, &body),
                    body,
                    face: None,
                    scale: size,
                    cheeks: None,
                    alpha: 255,
                };
                scene.characters.insert(slot, character);
                scene.figures.placed(slot);
            }
            // (time, slot)
            1 if c.args.len() == 2 => {
                self.character_time = self.character_time.max(c.int(0));
                scene.characters.remove(&c.int(1));
                scene.figures.forget(c.int(1));
            }
            // Commit: every change crossfades over the longest time given,
            // and the hops, sways, bows and slides run at the same time;
            // the script waits for the last of them.
            2 => {
                for (slot, steps) in scene.figures.start() {
                    if skip {
                        continue;
                    }
                    self.character_time = self.character_time.max(steps.len() as i32);
                    self.jolts.push((slot, Jolt { steps, step_ms: 16 }));
                }
                let t = std::mem::take(&mut self.character_time);
                self.handled += 1;
                return Some(Ok(self.transition(t, None)));
            }
            // (flag, slot, name): the expression layer. The flag is not
            // read by the original. A face put on a figure that stands on
            // the screen blends in over 12 frames, and the commit waits
            // for it, unless the full-screen sprite is up (then it is at
            // once); a figure put in the same commit brings its face with
            // its body.
            5 if c.args.len() == 3 => {
                let slot = c.int(1);
                if let Some(ch) = scene.characters.get_mut(&slot) {
                    ch.face = c.str(2).filter(|f| !f.is_empty());
                    if !veil {
                        let frames = scene.figures.face_frames(slot, true);
                        self.character_time = self.character_time.max(frames);
                    }
                }
            }
            // (side, slot): the order the figure overlaps the others in.
            6 if c.args.len() == 2 => {
                if scene.characters.contains_key(&c.int(1)) {
                    scene.figures.set_side(c.int(1), c.int(0));
                }
            }
            // (time): remove all.
            7 if c.args.len() == 1 => {
                self.character_time = self.character_time.max(c.int(0));
                scene.characters.clear();
                scene.figures.forget_all();
            }
            // (place, slot): the figure slides to the place of another
            // slot over 12 frames.
            8 if c.args.len() == 2 => {
                let (place, slot) = (c.int(0), c.int(1));
                if let Some(ch) = scene.characters.get_mut(&slot) {
                    let now = stage::shifted(slot, place, ch.scale, &ch.body);
                    let shift = (ch.offset.0 - now.0, ch.offset.1 - now.1);
                    ch.offset = now;
                    scene.figures.set_place(slot, place);
                    scene.figures.slide(slot, shift);
                }
            }
            // (size, slot): zoom to a size, here at once.
            9 if c.args.len() == 2 => {
                let (size, slot) = (c.int(0), c.int(1));
                let place = scene.figures.place(slot);
                if let Some(ch) = scene.characters.get_mut(&slot) {
                    ch.scale = size;
                    ch.offset = stage::shifted(slot, place, size, &ch.body);
                }
            }
            // (slot): a hop (10) or a bow (11) of 78 pixels.
            10 | 11 if c.args.len() == 1 => {
                if scene.characters.contains_key(&c.int(0)) {
                    scene.figures.bow(c.int(0), c.sub - 10);
                }
            }
            // (pixels, axis, slot): a hop (axis 0) or a sway (axis 1).
            12 if c.args.len() == 3 => {
                if scene.characters.contains_key(&c.int(2)) {
                    scene.figures.hop(c.int(2), c.int(1), c.int(0));
                }
            }
            // (kind, fade, release, slot): a glow around the figure. With
            // the third value 0 it comes; with 1 it goes.
            13 if c.args.len() == 4 => {
                let (kind, fade, release, slot) = (c.int(0), c.int(1), c.int(2), c.int(3));
                let size = scene.characters.get(&slot).map(|ch| ch.scale);
                match (release, size) {
                    (0, Some(size)) => {
                        figures::attach(&mut scene.effects, &[(kind, slot, size)], factor);
                        if let Some(movie) = petals {
                            figures::petals_movie(&mut scene.effects, movie);
                        }
                    }
                    (1, _) if fade != 0 => figures::release(&mut scene.effects, factor),
                    (1, _) => figures::detach(&mut scene.effects),
                    _ => {}
                }
            }
            // The glow is taken away at once.
            14 if c.args.is_empty() => figures::detach(&mut scene.effects),
            // (_, fade, slot, slot): the glow of two figures: a green glow
            // behind 葵, petals for the others.
            15 if c.args.len() == 4 && c.int(1) == 0 => {
                let parts: Vec<(i32, i32, i32)> = [c.int(3), c.int(2)]
                    .into_iter()
                    .filter_map(|slot| {
                        let ch = scene.characters.get(&slot)?;
                        let kind = if ch.body.to_ascii_lowercase().starts_with("tao") {
                            0
                        } else {
                            1
                        };
                        Some((kind, slot, ch.scale))
                    })
                    .collect();
                if !parts.is_empty() {
                    figures::attach(&mut scene.effects, &parts, factor);
                    if let Some(movie) = petals {
                        figures::petals_movie(&mut scene.effects, movie);
                    }
                }
            }
            15 => {}
            _ => return None,
        }
        self.done()
    }

    /// The face window of the message window.
    fn nanairo_portrait(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match (c.sub, c.args.len()) {
            // (face, body): the face is drawn over the body when it
            // differs.
            (4, 2) => {
                let (face, body) = (c.str(0)?, c.str(1)?);
                let face = (!face.eq_ignore_ascii_case(&body)).then_some(face);
                scene.portrait = Some((body, face));
            }
            (5, 0) => scene.portrait_visible = true,
            (6, 0) => scene.portrait_visible = false,
            // The empty window.
            (7, 0) => scene.portrait = None,
            // A whole picture.
            (8, 1) => scene.portrait = c.str(0).map(|b| (b, None)),
            _ => return None,
        }
        self.done()
    }

    /// The CG registry.
    fn nanairo_cg(&mut self, c: &Call, vm: &mut Vm) -> Option<silky_vm::Result<Flow>> {
        let cg = &mut self.scene.cg;
        match (c.sub, c.args.len()) {
            (10, 1) => cg.start(&c.str(0)?),
            (1, 2) => {
                cg.part(c.int(0).max(0) as usize, &c.str(1)?);
                cg.set_picture(0, c.int(0), &c.str(1)?);
            }
            // The second table of pictures (three places).
            (12, 2) => cg.set_picture(1, c.int(0), &c.str(1)?),
            // Register the combination.
            (11, 0) => cg.register(vm),
            // Copy the background onto the screen, draw the pictures of
            // the tables over it and register the combination; 5 and 7
            // also fade, a time that is not kept.
            (4 | 5, 0) | (7, 1) => {
                cg.compose();
                cg.register(vm);
            }
            // Frees a place of the first or second table.
            (2, 1) => cg.clear_picture(0, c.int(0)),
            (3, 1) => cg.clear_picture(1, c.int(0)),
            _ => return None,
        }
        self.done()
    }
}

/// Engine calls with nothing to do here, checked on the executable: the
/// message window page (`MesWnd.1`), font sizes, the one-line message,
/// menus and selection settings set up at the start, and its own
/// backlog's size, clearing and entry marks, as in 景の海のアペイリア.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        (2, 1) | (10, 0 | 1) | (24, 0 | 2) | (26, 2..=4) | (1, 0) | (22, 0..=2)
    )
}
