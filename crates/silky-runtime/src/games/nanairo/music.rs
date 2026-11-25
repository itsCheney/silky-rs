//! The game's own music mode (`Game.32`): 35 rows in three columns, a stop
//! button, a volume bar and the frame the CG and scene screens share
//! (see [`super::mode`]).
//!
//! The script draws the background before it calls this; the screen draws
//! the frame and its own parts over it. A row plays its track once; the
//! row stays yellow and the speaker mark and the title show while it plays.
//! The BGM the script left playing is faded out on entry (or stopped, with
//! the music off), and a track picked is left playing on leaving. Whenever
//! the music channel is silent the screens start [`BGM`].

use std::sync::Arc;

use silky_format::akb::Image;

use super::mode::{
    self, Frame, Mode, Phase, BACK, CANCEL_SOUND, FRAME_MS, OK_SOUND, QUIT, SLIDE_OUT_FRAMES,
};
use crate::assets::Assets;
use crate::games::kimaten1::omake::Music;
use crate::games::kimaten1::{Click, Look};
use crate::render::Canvas;
use crate::Result;

pub const SHEET: &str = "MUSICmode.akb";
/// The BGM of the three modes, started whenever the music channel is silent
/// and the music is on (also the first row of the music mode).
pub const BGM: &str = "25.wav";

/// The tracks: the flag that opens each and its file, in the order the
/// rows are drawn.
pub const TRACKS: [(usize, &str); 35] = [
    (3074, "25.wav"),
    (3077, "28.wav"),
    (3050, "01.wav"),
    (3051, "02.wav"),
    (3052, "03.wav"),
    (3053, "04.wav"),
    (3073, "24.wav"),
    (3075, "26.wav"),
    (3076, "27.wav"),
    (3054, "05.wav"),
    (3071, "22.wav"),
    (3055, "06.wav"),
    (3070, "21.wav"),
    (3056, "07.wav"),
    (3057, "08.wav"),
    (3065, "16.wav"),
    (3058, "09.wav"),
    (3068, "19.wav"),
    (3059, "10.wav"),
    (3060, "11.wav"),
    (3066, "17.wav"),
    (3067, "18.wav"),
    (3061, "12.wav"),
    (3062, "13.wav"),
    (3063, "14.wav"),
    (3069, "20.wav"),
    (3064, "15.wav"),
    (3072, "23.wav"),
    (3082, "33.wav"),
    (3078, "29.wav"),
    (3079, "30.wav"),
    (3080, "31.wav"),
    (3084, "35.wav"),
    (3081, "32.wav"),
    (3083, "34.wav"),
];
pub const ROWS: usize = TRACKS.len();

/// Area ids: the rows are 0..35.
const STOP: i32 = 0x100;
const VOLUME: i32 = 0x200;

/// The volume runs 0..=267 on the bar; the knob follows the pointer from
/// 775.
pub const VOLUME_MAX: i32 = 267;
const VOLUME_LEFT: i32 = 775;

const ROW_W: i32 = 388;
const ROW_H: i32 = 27;

/// Top left of a row on the page.
pub fn row_pos(k: usize) -> (i32, i32) {
    let x = match k {
        0..=12 => 41,
        13..=25 => 476,
        _ => 880,
    };
    (x, 128 + 37 * (k % 13) as i32)
}

/// Where a row's strip (four states of 27 rows) starts in the sheet.
fn strip_src(k: usize) -> (u32, u32) {
    if k < 9 {
        (465 + 388 * (k % 3) as u32, 322 + 108 * (k / 3) as u32)
    } else {
        let j = k - 9;
        (77 + 388 * (j % 4) as u32, 646 + 108 * (j / 4) as u32)
    }
}

/// The speaker mark's x by column.
fn speaker_x(k: usize) -> i32 {
    match k {
        0..=12 => 7,
        13..=25 => 442,
        _ => 846,
    }
}

const STOP_AT: (i32, i32) = (233, 615);
const TITLE_AT: (i32, i32) = (307, 630);
const PLATE: (i32, i32, i32, i32) = (220, 610, 840, 50);

/// The areas (left, top, right, bottom), as `SOUNDMODE.AREA` has them.
fn rect(id: i32) -> Option<(i32, i32, i32, i32)> {
    Some(match id {
        0..=34 => {
            let (x, y) = row_pos(id as usize);
            (x, y, x + ROW_W, y + ROW_H)
        }
        STOP => (233, 615, 273, 655),
        VOLUME => (769, 630, 1049, 658),
        0xf00 => (25, 6, 118, 117),
        0xf01 => (166, 6, 259, 117),
        0xf02 => (305, 6, 398, 117),
        BACK => (1077, 605, 1170, 716),
        QUIT => (1178, 605, 1271, 716),
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    No,
    /// Asked to leave; the slide out starts when the clock is next seen.
    Pending(i32),
    At(u64, i32),
}

#[derive(Debug, Clone)]
pub struct MusicMode {
    tracks: [u8; ROWS],
    tabs: [u8; 3],
    music_on: bool,
    volume: i32,
    playing: Option<usize>,
    /// The plate has holes where the title and the stop button were,
    /// after a stop.
    holed: bool,
    leave: Leave,
    sound: Option<&'static str>,
    music: Option<Music>,
    new_volume: Option<f32>,
}

impl MusicMode {
    /// `tracks` are the flags 3050..3084 (1 when open), `tabs` the three tab bytes in strip order
    /// (CG, music, scene), `volume` the music volume 0..1.
    pub fn open(tracks: [u8; ROWS], tabs: [u8; 3], music_on: bool, volume: f32) -> Self {
        MusicMode {
            tracks,
            tabs,
            music_on,
            volume: (volume.clamp(0.0, 1.0) * VOLUME_MAX as f32).round() as i32,
            playing: None,
            holed: false,
            leave: Leave::No,
            sound: None,
            music: None,
            new_volume: None,
        }
    }

    fn unlocked(&self, k: usize) -> bool {
        self.tracks[k] == 1
    }

    fn tab_open(&self, i: usize) -> bool {
        self.tabs[i] != 0
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let inside =
            |id: i32| rect(id).is_some_and(|(l, t, r, b)| x >= l && x < r && y >= t && y < b);
        (0..ROWS as i32)
            .chain([STOP, VOLUME, 0xf00, 0xf01, 0xf02, BACK, QUIT])
            .filter(|&id| id != STOP || self.playing.is_some())
            .find(|&id| inside(id))
    }

    /// Where an arrow key puts the pointer: the button it is on is `at`.
    pub fn arrow(&self, at: Option<i32>, key: super::keys::Arrow) -> Option<i32> {
        if self.leave != Leave::No {
            return None;
        }
        super::keys::music(at, key, self.playing.is_some(), self.music_on)
    }

    /// The middle of a button, where the pointer goes; on the volume bar it
    /// is the knob's place.
    pub fn centre(&self, id: i32) -> Option<(i32, i32)> {
        if id == VOLUME {
            let (l, t, _, _) = rect(VOLUME)?;
            return Some((l + 14 + self.volume, t + 14));
        }
        rect(id).map(super::keys::middle)
    }

    pub fn enabled(&self, id: i32) -> bool {
        match id {
            0..=34 => self.unlocked(id as usize) && self.playing != Some(id as usize),
            STOP => self.playing.is_some(),
            VOLUME => self.music_on,
            0xf00..=0xf02 => {
                let i = (id - mode::TAB_FIRST) as usize;
                i != Mode::Music.tab() && self.tab_open(i)
            }
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.leave != Leave::No {
            return Click::None;
        }
        match id {
            0..=34 => {
                let k = id as usize;
                if self.unlocked(k) && self.playing != Some(k) {
                    self.sound = Some(OK_SOUND);
                    self.playing = Some(k);
                    self.holed = false;
                    self.music = Some(Music::Play(TRACKS[k].1.to_string()));
                }
                Click::None
            }
            STOP if self.playing.is_some() => {
                self.sound = Some(OK_SOUND);
                self.stop();
                Click::None
            }
            0xf00 | 0xf02 => {
                let i = (id - mode::TAB_FIRST) as usize;
                if self.tab_open(i) {
                    self.sound = Some(OK_SOUND);
                    let to = if i == Mode::Cg.tab() {
                        Mode::Cg
                    } else {
                        Mode::Scene
                    };
                    self.leave = Leave::Pending(to.code());
                }
                Click::None
            }
            BACK => {
                self.sound = Some(OK_SOUND);
                self.leave = Leave::Pending(mode::LEAVE);
                Click::None
            }
            QUIT => {
                self.sound = Some(OK_SOUND);
                Click::Bar(crate::ui::bar::QUIT)
            }
            _ => Click::None,
        }
    }

    fn stop(&mut self) {
        self.playing = None;
        self.holed = true;
        self.music = Some(Music::Stop);
    }

    /// A press or drag on the volume bar.
    pub fn press_at(&mut self, id: i32, x: i32, _y: i32) {
        if id == VOLUME && self.music_on && self.leave == Leave::No {
            self.volume = (x - VOLUME_LEFT).clamp(0, VOLUME_MAX);
            self.new_volume = Some(self.volume as f32 / VOLUME_MAX as f32);
        }
    }

    /// A right click or Escape: a track playing is stopped first, then the
    /// screen is left with the cancel sound.
    pub fn cancel(&mut self) -> Click {
        if self.leave != Leave::No {
            return Click::None;
        }
        if self.playing.is_some() {
            self.stop();
        } else {
            self.sound = Some(CANCEL_SOUND);
            self.leave = Leave::Pending(mode::LEAVE);
        }
        Click::None
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    pub fn take_music(&mut self) -> Option<Music> {
        self.music.take()
    }

    pub fn take_volume(&mut self) -> Option<f32> {
        self.new_volume.take()
    }

    /// The picked track ended: the screen stops it (no next track).
    pub fn track_ended(&mut self) -> bool {
        if self.playing.is_some() {
            self.stop();
            self.music = None;
            true
        } else {
            false
        }
    }

    pub fn track_playing(&self) -> bool {
        self.playing.is_some()
    }

    /// Headless run: the first thing to press.
    pub fn auto_pick(&self) -> Option<i32> {
        Some(BACK)
    }

    /// Whether the frame is sliding in or out at `ms` since the screen
    /// opened.
    pub fn animating(&self, ms: u64) -> bool {
        self.leave != Leave::No || ms < mode::SLIDE_IN_FRAMES * FRAME_MS
    }

    /// Runs the slide out; the answer comes when it is over.
    pub fn tick(&mut self, ms: u64) -> Option<Click> {
        match self.leave {
            Leave::No => None,
            Leave::Pending(a) => {
                self.leave = Leave::At(ms, a);
                None
            }
            Leave::At(t0, a) => {
                (ms >= t0 + SLIDE_OUT_FRAMES * FRAME_MS).then_some(Click::Result(a))
            }
        }
    }

    /// A run without time: the answer the slide out holds back, now.
    pub fn settle(&mut self) -> Option<Click> {
        match self.leave {
            Leave::No => None,
            Leave::Pending(a) | Leave::At(_, a) => Some(Click::Result(a)),
        }
    }

    fn state(&self, look: &Look, id: i32) -> u32 {
        if look.pressed == Some(id) {
            2
        } else if look.hover == Some(id) {
            1
        } else {
            0
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image(SHEET)?;
        let phase = match self.leave {
            Leave::At(t0, _) => Phase::at(look.ms, Some(t0), true),
            _ => Phase::at(look.ms, None, true),
        };
        let alpha = phase.page_alpha(look.ms);
        let frame = Frame {
            sheet: &sheet,
            mode: Mode::Music,
            open: [self.tab_open(0), true, self.tab_open(2)],
            hover: look.hover,
            pressed: look.pressed,
            phase,
        };
        self.draw_page(canvas, &sheet, look, alpha);
        // The frame lies over the page.
        frame.draw(canvas);
        Ok(())
    }

    fn draw_plate(&self, canvas: &mut Canvas, sheet: &Arc<Image>, alpha: u8) {
        let (px, py, pw, ph) = PLATE;
        let part = |canvas: &mut Canvas, l: i32, t: i32, r: i32, b: i32| {
            if r > l && b > t {
                canvas.draw_part(
                    sheet,
                    (
                        465 + (l - px) as u32,
                        91 + (t - py) as u32,
                        (r - l) as u32,
                        (b - t) as u32,
                    ),
                    l,
                    t,
                    false,
                    alpha,
                );
            }
        };
        let (right, bottom) = (px + pw, py + ph);
        if !self.holed {
            part(canvas, px, py, right, bottom);
            return;
        }
        // Without the title and the stop button.
        part(canvas, px, py, 233, bottom);
        part(canvas, 233, py, 273, 615);
        part(canvas, 233, 655, 273, bottom);
        part(canvas, 273, py, 307, bottom);
        part(canvas, 307, py, 695, 630);
        part(canvas, 307, 657, 695, bottom);
        part(canvas, 695, py, right, bottom);
    }

    fn draw_page(&self, canvas: &mut Canvas, sheet: &Arc<Image>, look: &Look, alpha: u8) {
        self.draw_plate(canvas, sheet, alpha);
        for k in 0..ROWS {
            let (x, y) = row_pos(k);
            if !self.unlocked(k) {
                canvas.draw_part(sheet, (465, 295, 388, 27), x, y, false, alpha);
                continue;
            }
            let state = if self.playing == Some(k) || look.pressed == Some(k as i32) {
                2
            } else if look.hover == Some(k as i32) {
                1
            } else {
                0
            };
            let (sx, sy) = strip_src(k);
            canvas.draw_part(sheet, (sx, sy + 27 * state, 388, 27), x, y, false, alpha);
        }
        if let Some(k) = self.playing {
            let (_, y) = row_pos(k);
            canvas.draw_part(sheet, (465, 271, 28, 24), speaker_x(k), y, false, alpha);
            let (sx, sy) = strip_src(k);
            canvas.draw_part(
                sheet,
                (sx, sy, 388, 27),
                TITLE_AT.0,
                TITLE_AT.1,
                false,
                alpha,
            );
            let st = self.state(look, STOP);
            canvas.draw_part(
                sheet,
                (465, 141 + 40 * st, 40, 40),
                STOP_AT.0,
                STOP_AT.1,
                false,
                alpha,
            );
        }
        self.draw_volume(canvas, sheet, look, alpha);
    }

    fn draw_volume(&self, canvas: &mut Canvas, sheet: &Arc<Image>, look: &Look, alpha: u8) {
        canvas.draw_part(sheet, (1007, 111, 295, 28), 761, 630, false, alpha);
        if !self.music_on {
            canvas.draw_part(sheet, (573, 141, 280, 14), 769, 637, false, alpha);
            return;
        }
        let v = self.volume;
        canvas.draw_part(
            sheet,
            (573, 141, (v + 6) as u32, 14),
            769,
            637,
            false,
            alpha,
        );
        canvas.draw_part(
            sheet,
            (579 + v as u32, 155, (274 - v) as u32, 14),
            769 + v + 6,
            637,
            false,
            alpha,
        );
        let knob = u32::from(look.pressed == Some(VOLUME) || look.hover == Some(VOLUME));
        canvas.draw_part(
            sheet,
            (545, 141 + 28 * knob, 28, 28),
            761 + v,
            630,
            false,
            alpha,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_all() -> MusicMode {
        MusicMode::open([1; ROWS], [1; 3], true, 200.0 / 267.0)
    }

    #[test]
    fn arrow_keys_move_between_buttons() {
        use super::super::keys::Arrow;
        let m = open_all();
        assert_eq!(m.arrow(None, Arrow::Down), Some(0));
        assert_eq!(m.arrow(Some(0), Arrow::Down), Some(1));
        let (x, y) = m.centre(1).unwrap();
        let (l, t, r, b) = rect(1).unwrap();
        assert!((l..=r).contains(&x) && (t..=b).contains(&y));
        // The volume bar answers with the knob.
        let (kx, _) = m.centre(VOLUME).unwrap();
        let (l, _, _, _) = rect(VOLUME).unwrap();
        assert_eq!(kx, l + 14 + m.volume);
    }

    #[test]
    fn rows_flags_and_files() {
        let mut flags: Vec<usize> = TRACKS.iter().map(|t| t.0).collect();
        flags.sort();
        assert_eq!(flags, (3050..=3084).collect::<Vec<_>>());
        let mut files: Vec<String> = TRACKS.iter().map(|t| t.1.to_string()).collect();
        files.sort();
        let want: Vec<String> = (1..=35).map(|n| format!("{n:02}.wav")).collect();
        assert_eq!(files, want);
        assert_eq!(TRACKS[2], (3050, "01.wav"));
        assert_eq!(TRACKS[0], (3074, "25.wav"));
    }

    #[test]
    fn row_places_and_strips() {
        assert_eq!(row_pos(0), (41, 128));
        assert_eq!(row_pos(12), (41, 128 + 37 * 12));
        assert_eq!(row_pos(13), (476, 128));
        assert_eq!(row_pos(26), (880, 128));
        assert_eq!(row_pos(34), (880, 128 + 37 * 8));
        assert_eq!(strip_src(0), (465, 322));
        assert_eq!(strip_src(4), (465 + 388, 322 + 108));
        assert_eq!(strip_src(9), (77, 646));
        assert_eq!(strip_src(13), (77, 646 + 108));
        assert_eq!(speaker_x(13), 442);
    }

    #[test]
    fn hit_areas() {
        let mut m = open_all();
        assert_eq!(m.hit(50, 130), Some(0));
        assert_eq!(m.hit(500, 130), Some(13));
        assert_eq!(m.hit(900, 130 + 37 * 8), Some(34));
        assert_eq!(m.hit(900, 130 + 37 * 9), None);
        assert_eq!(m.hit(800, 640), Some(VOLUME));
        assert_eq!(m.hit(30, 10), Some(0xf00));
        assert_eq!(m.hit(200, 10), Some(0xf01));
        assert_eq!(m.hit(350, 10), Some(0xf02));
        assert_eq!(m.hit(1100, 650), Some(BACK));
        assert_eq!(m.hit(1200, 650), Some(QUIT));
        // The stop button exists only while a track plays.
        assert_eq!(m.hit(250, 630), None);
        m.click(3);
        assert_eq!(m.hit(250, 630), Some(STOP));
    }

    #[test]
    fn play_and_stop() {
        let mut m = open_all();
        assert_eq!(m.click(2), Click::None);
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        assert_eq!(m.take_music(), Some(Music::Play("01.wav".into())));
        assert!(m.track_playing());
        // The playing row does nothing again.
        assert!(!m.enabled(2));
        m.click(2);
        assert_eq!(m.take_sound(), None);
        assert_eq!(m.take_music(), None);
        // Another row replaces it.
        m.click(0);
        assert_eq!(m.take_music(), Some(Music::Play("25.wav".into())));
        m.click(STOP);
        assert_eq!(m.take_music(), Some(Music::Stop));
        assert!(!m.track_playing() && m.holed);
        // Picking again fills the plate.
        m.click(1);
        assert!(!m.holed);
    }

    #[test]
    fn locked_rows_do_nothing() {
        let mut t = [0; ROWS];
        t[5] = 1;
        let mut m = MusicMode::open(t, [0; 3], true, 0.5);
        assert!(!m.enabled(4) && m.enabled(5));
        m.click(4);
        assert_eq!(m.take_music(), None);
        assert_eq!(m.take_sound(), None);
        m.click(5);
        assert_eq!(m.take_music(), Some(Music::Play("04.wav".into())));
    }

    #[test]
    fn a_track_that_ended_is_stopped() {
        let mut m = open_all();
        m.click(7);
        m.take_music();
        assert!(m.track_ended());
        assert!(!m.track_playing());
        assert_eq!(m.take_music(), None);
        assert!(!m.track_ended());
    }

    #[test]
    fn volume_follows_the_pointer() {
        let mut m = open_all();
        assert_eq!(m.volume, 200);
        m.press_at(VOLUME, 775 + 100, 640);
        assert_eq!(m.volume, 100);
        assert_eq!(m.take_volume(), Some(100.0 / 267.0));
        m.press_at(VOLUME, 700, 640);
        assert_eq!(m.volume, 0);
        m.press_at(VOLUME, 1200, 640);
        assert_eq!(m.volume, 267);
        assert_eq!(m.take_volume(), Some(1.0));
        // Off: the bar does not move.
        let mut m = MusicMode::open([1; ROWS], [1; 3], false, 0.5);
        m.press_at(VOLUME, 800, 640);
        assert_eq!(m.take_volume(), None);
        assert!(!m.enabled(VOLUME));
    }

    #[test]
    fn tabs_leave_with_the_mode_codes() {
        let mut m = open_all();
        // The music tab itself does nothing.
        assert!(!m.enabled(0xf01));
        m.click(0xf01);
        assert_eq!(m.leave, Leave::No);
        m.click(0xf00);
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        // The answer waits for the slide out.
        assert_eq!(m.tick(1000), None);
        assert_eq!(m.tick(1000 + 11 * 16), None);
        assert_eq!(m.tick(1000 + 12 * 16), Some(Click::Result(0x100)));
        let mut m = open_all();
        m.click(0xf02);
        assert_eq!(m.settle(), Some(Click::Result(0x102)));
        assert_eq!(mode::next(Mode::Music, 0x102), mode::Next::Go(Mode::Scene));
    }

    #[test]
    fn dead_tabs_do_nothing() {
        let mut m = MusicMode::open([1; ROWS], [0, 1, 0], true, 0.5);
        assert!(!m.enabled(0xf00) && !m.enabled(0xf02));
        m.click(0xf00);
        m.click(0xf02);
        assert_eq!(m.settle(), None);
        assert_eq!(m.take_sound(), None);
    }

    #[test]
    fn back_quit_and_cancel() {
        let mut m = open_all();
        assert_eq!(m.click(BACK), Click::None);
        assert_eq!(m.settle(), Some(Click::Result(-1)));
        let mut m = open_all();
        assert_eq!(m.click(QUIT), Click::Bar(crate::ui::bar::QUIT));
        assert_eq!(m.take_sound().as_deref(), Some("ses_001.wav"));
        // A right click stops a playing track without a sound, then leaves
        // with the cancel sound.
        let mut m = open_all();
        m.click(3);
        m.take_sound();
        m.take_music();
        assert_eq!(m.cancel(), Click::None);
        assert_eq!(m.take_music(), Some(Music::Stop));
        assert_eq!(m.take_sound(), None);
        assert_eq!(m.settle(), None);
        m.cancel();
        assert_eq!(m.take_sound().as_deref(), Some("ses_002.wav"));
        assert_eq!(m.settle(), Some(Click::Result(-1)));
    }

    #[test]
    fn slide_in_runs_on_the_clock() {
        let m = open_all();
        assert!(m.animating(0));
        assert!(m.animating(13 * 16));
        assert!(!m.animating(14 * 16));
    }
}
