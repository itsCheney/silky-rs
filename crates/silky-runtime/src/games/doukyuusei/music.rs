//! The music mode of 同級生 Remake (`Game.52`, "Youth Medley"), the third
//! page of the extras.
//!
//! Read from the executable. Four discs list 58 tracks (16, 14, 14 and 14);
//! the track rows come from `music_disc<n>_<lang>.akb`, four columns of
//! 740 by 64 cells and four states each (open, hover, playing, locked).
//! A track is open when its byte variable is 1 or any ending of the album
//! is seen. Clicking an open track plays its file once, clicking the
//! playing one stops it; the record on the right shows the disc. The
//! volume slider (shown while the music switch is on) sets the music
//! volume of the settings.
//!
//! The side tabs answer 100 with global 12 naming the next page of the
//! extras (global 17 cleared); back answers -1. A secondary click stops
//! what plays, else leaves. The music stops on leaving.
//! See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::area::{self, Area};

use super::album::{self, endings};
use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

pub use super::album::{BACK, LEAVE, TAB, TO_PAGE};

/// Disc buttons (768 + disc) and the volume slider.
pub const DISC: i32 = 0x300;
pub const VOLUME: i32 = 0x400;

/// Globals the music mode writes.
pub mod globals {
    pub const PAGE_OF_EXTRAS: usize = 12;
    pub const PAGE: usize = 17;
}

/// First track of each disc and the tracks on it.
const OFFSETS: [usize; 4] = [0, 16, 30, 44];
const COUNTS: [usize; 4] = [16, 14, 14, 14];
pub const TRACKS: usize = 58;

/// Byte variable of each track.
const FLAGS: [usize; TRACKS] = [
    4121, 4131, 4126, 4127, 4128, 4135, 4110, 4140, 4120, 4124, 4143, 4163, 4161, 4159, 4160, 4100,
    4134, 4106, 4108, 4162, 4103, 4122, 4136, 4118, 4130, 4107, 4111, 4104, 4166, 4167, 4168, 4117,
    4116, 4129, 4113, 4102, 4142, 4115, 4145, 4109, 4125, 4112, 4141, 4119, 4150, 4153, 4152, 4156,
    4149, 4146, 4158, 4151, 4145, 4147, 4155, 4148, 4157, 4154,
];

/// Sound file of each track in the music archive.
pub const FILES: [&str; TRACKS] = [
    "photo",
    "sotugyou",
    "room1",
    "room2",
    "room3",
    "town",
    "ido_1",
    "waltz",
    "obakeya",
    "pool_2",
    "yuenchi",
    "cats",
    "bar",
    "kazuya",
    "kenji",
    "o_3",
    "taro",
    "g-g1",
    "g-g3",
    "comical",
    "do!hyan",
    "piano_3",
    "violin_3",
    "m_box",
    "sippori",
    "g-g2",
    "ITTA_ATO",
    "FAULT",
    "OP_Memory_Short",
    "ED_Story",
    "mai_4",
    "misa",
    "miho",
    "satomi",
    "kurumi_2",
    "chiharu",
    "yoshiko",
    "mako",
    "ako",
    "hiromi",
    "reiko",
    "kaori",
    "yayoi_2",
    "natuko",
    "z_mai_4",
    "z_misa",
    "z_miho",
    "z_satomi",
    "z_kurumi_2",
    "z_chiharu",
    "z_yoshiko",
    "z_mako",
    "z_ako",
    "z_hiromi",
    "z_reiko",
    "z_kaori",
    "z_yayoi2",
    "z_natuko",
];

/// The file name of a track in the archive.
pub fn file(track: usize) -> String {
    format!("{}.wav", FILES[track])
}

/// Slider range in pixels.
const SPAN: i32 = 506;
const SLIDER_LEFT: i32 = 1379;

/// The slider position of a pointer x.
pub fn slider_at(x: i32) -> i32 {
    (x - SLIDER_LEFT).clamp(0, SPAN)
}

#[derive(Debug, Clone)]
pub struct Music {
    /// With the update archive's pages (the scene tab).
    pub r18: bool,
    /// The disc shown.
    pub disc: usize,
    /// The track playing (counted over all discs).
    pub playing: Option<usize>,
    /// The volume slider is held.
    pub dragging: bool,
    /// The music switch and volume of the settings, as shown.
    pub on: bool,
    pub volume: f32,
    open: [bool; TRACKS],
    areas: Arc<Vec<Area>>,
    set: Vec<(usize, i32)>,
}

impl Music {
    /// `bytes` are the byte variables; `volume` and `on` the music
    /// setting.
    pub fn load(assets: &Assets, bytes: &[u8], volume: f32, on: bool) -> Result<Self> {
        let areas = area::parse(&assets.data("musicmode.area")?)?;
        let mut m = Self {
            r18: album::r18(assets),
            disc: 0,
            playing: None,
            dragging: false,
            on,
            volume,
            open: [false; TRACKS],
            areas: Arc::new(areas),
            set: Vec::new(),
        };
        m.refresh(bytes);
        Ok(m)
    }

    /// Reads which tracks are open.
    pub fn refresh(&mut self, bytes: &[u8]) {
        let seen = endings(bytes).iter().any(|&c| c);
        self.open = std::array::from_fn(|i| seen || bytes.get(FLAGS[i]).copied().unwrap_or(0) == 1);
    }

    pub fn take_globals(&mut self) -> Vec<(usize, i32)> {
        std::mem::take(&mut self.set)
    }

    /// The tabs there are: the fourth only with the update archive.
    fn tabs(&self) -> i32 {
        if self.r18 {
            4
        } else {
            3
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        self.areas
            .iter()
            .filter(|a| match a.id >> 8 {
                0 => (a.id as usize) < COUNTS[self.disc],
                1 => (a.id & 0xff) < self.tabs(),
                3 => true,
                4 => self.on,
                _ => true,
            })
            .find(|a| a.contains(x, y))
            .map(|a| a.id)
    }

    /// The tab of the music page.
    fn current_tab(&self) -> i32 {
        if self.r18 {
            2
        } else {
            1
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        let (kind, i) = (id >> 8, (id & 0xff) as usize);
        match kind {
            0 if i < COUNTS[self.disc] => {
                let t = OFFSETS[self.disc] + i;
                if !self.open[t] {
                    Click::None
                } else if self.playing == Some(t) {
                    self.playing = None;
                    Click::Track(None)
                } else {
                    self.playing = Some(t);
                    Click::Track(Some(t))
                }
            }
            1 if (i as i32) < self.tabs() && i as i32 != self.current_tab() => {
                let page = match (i, self.r18) {
                    (0, _) => 0,
                    (1, _) => 1,
                    _ => 3,
                };
                self.set = vec![(globals::PAGE, 0), (globals::PAGE_OF_EXTRAS, page)];
                Click::Result(TO_PAGE)
            }
            2 => Click::Result(LEAVE),
            3 if i < 4 => {
                self.disc = i;
                Click::None
            }
            _ => Click::None,
        }
    }

    /// The secondary click: stops the music that plays (`None`), else
    /// leaves.
    pub fn cancel(&mut self) -> Option<i32> {
        match self.playing.take() {
            Some(_) => None,
            None => Some(LEAVE),
        }
    }

    /// The music setting at slider position `v`.
    pub fn volume_at(x: i32) -> f32 {
        slider_at(x) as f32 / SPAN as f32
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed) of an id.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let parts = assets.localized_image("omake_p.akb", language)?;
        canvas.draw_part(&parts, (0, 136, 1920, 68), 0, 0, false, 255);
        let record = assets.image("record.akb")?;
        canvas.draw_part(&record, (0, 0, 745, 983), 1175, 0, false, 255);
        // The original spins the label; it is drawn still here.
        let frames = assets.image("recorddisc.akb")?;
        let f = u32::from(self.playing.is_some());
        canvas.draw_part(&frames, (0, f * 769, 722, 769), 1198, 0, false, 255);
        let label =
            assets.localized_image(&format!("recorddisc{}_s.akb", self.disc + 1), language)?;
        canvas.draw_part(&label, (0, 0, 450, 450), 1425, 84, false, 255);

        let rows = assets.localized_image(&format!("music_disc{}.akb", self.disc + 1), language)?;
        for i in 0..COUNTS[self.disc] {
            let t = OFFSETS[self.disc] + i;
            let row = if !self.open[t] {
                3
            } else if self.playing == Some(t) {
                2
            } else {
                u32::from(state(i as i32) > 0)
            };
            let (sx, sy) = ((i % 4) as u32 * 740, ((i & !3) as u32 + row) * 64);
            let (x, y) = ((i % 2) as i32 * 26 + 305, 84 + i as i32 * 59);
            canvas.draw_part(&rows, (sx, sy, 740, 64), x, y, false, 255);
        }
        for i in 0..4u32 {
            let s = if i as usize == self.disc {
                2
            } else {
                state(DISC | i as i32).min(1)
            };
            let sy = (s + 4 * (i / 3)) * 67 + 1075;
            canvas.draw_part(
                &parts,
                (i % 3 * 172 + 568, sy, 172, 68),
                1197 + i as i32 * 180,
                887,
                false,
                255,
            );
        }
        if self.on {
            let v = (self.volume.clamp(0.0, 1.0) * SPAN as f32).round() as i32;
            canvas.draw_part(&parts, (0, 1256, 520, 55), 1372, 783, false, 255);
            canvas.draw_part(&parts, (0, 1311, (v + 7) as u32, 55), 1372, 783, false, 255);
            let s = state(VOLUME).min(2);
            canvas.draw_part(
                &parts,
                (520, s * 76 + 1256, 44, 76),
                1357 + v,
                772,
                false,
                255,
            );
        }
        for i in 0..self.tabs() {
            let s = if i == self.current_tab() {
                2
            } else {
                state(TAB | i).min(2)
            };
            let sx = if self.r18 {
                i as u32 * 237
            } else {
                [0, 474, 711][i as usize]
            };
            canvas.draw_part(
                &parts,
                (sx, s * 163 + 272, 237, 163),
                0,
                i * 170 + 132,
                false,
                255,
            );
        }
        let s = state(BACK).min(2);
        canvas.draw_part(&parts, (948, s * 123 + 272, 218, 123), 10, 953, false, 255);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn music(r18: bool) -> Music {
        let mut m = Music {
            r18,
            disc: 0,
            playing: None,
            dragging: false,
            on: true,
            volume: 0.5,
            open: [false; TRACKS],
            areas: Arc::new(Vec::new()),
            set: Vec::new(),
        };
        m.refresh(&[]);
        m
    }

    #[test]
    fn tables() {
        assert_eq!(COUNTS.iter().sum::<usize>(), TRACKS);
        for d in 1..4 {
            assert_eq!(OFFSETS[d], OFFSETS[d - 1] + COUNTS[d - 1]);
        }
        assert_eq!(slider_at(0), 0);
        assert_eq!(slider_at(5000), 506);
    }

    #[test]
    fn tracks() {
        let mut m = music(true);
        // Locked tracks do nothing.
        assert!(matches!(m.click(0), Click::None));
        let mut bytes = vec![0u8; 5000];
        bytes[FLAGS[1]] = 1;
        bytes[FLAGS[16]] = 1;
        m.refresh(&bytes);
        assert!(matches!(m.click(1), Click::Track(Some(1))));
        assert_eq!(m.playing, Some(1));
        assert!(matches!(m.click(1), Click::Track(None)));
        assert!(matches!(m.click(1), Click::Track(Some(1))));
        // A secondary click stops, then leaves.
        assert_eq!(m.cancel(), None);
        assert_eq!(m.cancel(), Some(LEAVE));
        // Disc buttons switch the disc and keep the music.
        m.click(1);
        assert!(matches!(m.click(DISC | 1), Click::None));
        assert_eq!(m.disc, 1);
        assert_eq!(m.playing, Some(1));
        assert!(matches!(m.click(0), Click::Track(Some(16))));
        // An ending opens all.
        let mut m = music(true);
        bytes[album::CLEAR[0]] = 1;
        m.refresh(&bytes);
        assert!(m.open.iter().all(|&o| o));
    }

    #[test]
    fn tabs() {
        let mut m = music(true);
        assert!(matches!(m.click(TAB | 2), Click::None));
        assert!(matches!(m.click(TAB | 1), Click::Result(TO_PAGE)));
        assert!(m.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 1)));
        assert!(matches!(m.click(TAB), Click::Result(TO_PAGE)));
        assert!(m.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 0)));
        assert!(matches!(m.click(TAB | 3), Click::Result(TO_PAGE)));
        let set = m.take_globals();
        assert!(set.contains(&(globals::PAGE_OF_EXTRAS, 3)) && set.contains(&(globals::PAGE, 0)));
        assert!(matches!(m.click(BACK), Click::Result(LEAVE)));
        let mut m = music(false);
        assert!(matches!(m.click(TAB | 1), Click::None));
        assert!(matches!(m.click(TAB | 3), Click::None));
        assert!(matches!(m.click(TAB | 2), Click::Result(TO_PAGE)));
        assert!(m.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 3)));
    }
}
