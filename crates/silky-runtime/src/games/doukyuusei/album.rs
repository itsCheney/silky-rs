//! The album of 同級生 Remake (`Game.50`, the program's `CCGMode`), the
//! first page of the extras.
//!
//! Read from the executable. It starts on a grid of the 15 girls
//! (`cgmode.area`, cards of `chara_list_<lang>.akb` as on the character
//! picker, the 15th from (2212, 0)); a girl opens her pictures
//! (`cgview.area`), nine to a page. Each picture is a byte variable from the
//! program's tables (one with the update archive's pictures, one without),
//! open when it is 1. A closed one shows the girl's empty card of
//! `thumb_emp_<lang>.akb`; an open one its cell of the girl's
//! `cg_<name>[_r18].akb` sheet under a frame of `omake_p_<lang>.akb`. The
//! header shows how much of the whole album is open, in percent.
//!
//! Clicking an open picture answers the script, which shows it, with
//! globals 10 (girl from 1), 13 (place on the page from 1) and 17 (page
//! from 1) set and 16 (coming back) cleared; the script sets 16 again so
//! the album opens on that page. The side tabs answer 100 with global 12
//! naming the next page of the extras; back answers -1. The three time
//! buttons set global 19 (12:00, 16:00, 18:00), which the script uses to
//! show the picture at that time of day. After a girl's ending (flags in
//! [`CLEAR`]) a button opens all her pictures, after asking.
//! See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::area::{self, Area};

use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

pub const GIRLS: usize = 15;
pub const TAB: i32 = 0x100;
pub const BACK: i32 = 0x200;
pub const TIME: i32 = 0x300;
pub const PREVIOUS: i32 = 0x400;
pub const NEXT: i32 = 0x401;
pub const UNLOCK: i32 = 0x500;

/// What the side tabs and back answer.
pub const TO_PAGE: i32 = 100;
pub const LEAVE: i32 = -1;

/// Globals the album reads and writes.
pub mod globals {
    pub const GIRL: usize = 10;
    pub const PAGE_OF_EXTRAS: usize = 12;
    pub const PICTURE: usize = 13;
    pub const RETURNING: usize = 16;
    pub const PAGE: usize = 17;
    pub const TIME: usize = 19;
}

/// Pictures by girl with the update archive (byte variables; 0 is a
/// place that stays empty).
const PICTURES_R18: [&[usize]; GIRLS] = [
    &[
        4200, 4201, 4202, 4203, 4204, 4205, 4206, 4207, 4208, 4209, 4210, 4211, 4213, 4214, 4215,
        4216, 4217, 4218, 4219, 4220,
    ],
    &[
        4277, 4221, 4222, 4223, 4224, 4225, 4226, 4227, 4228, 4229, 4230, 4231, 4232, 4233, 4234,
        4235,
    ],
    &[
        4249, 4250, 4251, 4252, 4253, 4254, 4255, 4256, 4257, 4258, 4259, 4260, 4261, 4262,
    ],
    &[
        4236, 4237, 4238, 4239, 4240, 4241, 4242, 4243, 4244, 4245, 4246, 4247, 4248, 0,
    ],
    &[
        4263, 4264, 4467, 4265, 4266, 4267, 4268, 4269, 4270, 4271, 4272, 4273,
    ],
    &[
        4316, 4317, 4318, 4319, 4320, 4321, 4322, 4323, 4324, 4325, 4326, 4327, 4328, 4329, 4330,
    ],
    &[
        4303, 4304, 4305, 4306, 4307, 4308, 4309, 4310, 4311, 4312, 4313, 4314, 4315,
    ],
    &[
        4274, 4275, 4276, 4278, 4279, 4280, 4281, 4282, 4283, 4284, 4285, 4286, 4287,
    ],
    &[
        4357, 4358, 4359, 4360, 4361, 4362, 4363, 4364, 4365, 4367, 4368, 4369,
    ],
    &[
        4370, 4371, 4372, 4373, 4374, 4375, 4376, 4377, 4378, 4379, 4380, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    &[
        4331, 4468, 4332, 4333, 4334, 4335, 4336, 4337, 4338, 4339, 4340, 4341, 4342,
    ],
    &[
        4381, 4382, 4383, 4384, 4385, 4386, 4387, 4388, 4399, 4389, 4390, 4391, 4392, 4393, 4394,
    ],
    &[
        4288, 4289, 4290, 4291, 4292, 4293, 4294, 4295, 4296, 4297, 4298, 4299, 4300, 4301, 4302,
    ],
    &[
        4343, 4345, 4346, 4347, 4348, 4349, 4350, 4351, 4352, 4353, 4354, 4355, 4356,
    ],
    &[
        4453, 4454, 4455, 4456, 4457, 4458, 4459, 4460, 4461, 4462, 4463, 4464, 4465, 4466,
    ],
];

/// Pictures by girl without the update archive.
const PICTURES: [&[usize]; GIRLS] = [
    &[4201, 4202, 4207, 4209, 4210, 4211, 4213, 4219, 4220, 0],
    &[4277, 4221, 4222, 4235],
    &[4249, 4250, 4251, 4253, 4254, 4255, 4257, 4261],
    &[4236, 4238, 4241, 4242, 4248],
    &[4263, 4266, 4272, 0],
    &[4316, 4323, 4324, 4329],
    &[4303, 4304, 4305, 4306, 4307, 4314],
    &[4274, 4275, 4276, 4278, 4279, 4280, 4281, 4282, 4286, 4287],
    &[4358, 4359, 4360, 4361, 4362, 4368, 4369],
    &[4370, 4371, 4372, 4373, 4378, 4379, 4380],
    &[4331, 4468, 4334, 4340, 4341],
    &[4381, 4382, 4385, 4386, 4390, 4391, 4392],
    &[
        4288, 4289, 4290, 4291, 4292, 4294, 4295, 4296, 4297, 4299, 4300, 4301, 4302,
    ],
    &[4343, 4345, 4346, 4347, 4348, 4355, 4356],
    &[4453],
];

/// Pages by girl, with and without the update archive.
const PAGES_R18: [usize; GIRLS] = [3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2];
const PAGES: [usize; GIRLS] = [1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 2, 1, 1];

/// Picture sheets by girl (`cg_<name>.akb`, `_r18` added with the update
/// archive).
const SHEETS: [&str; GIRLS] = [
    "v10", "l02", "16_x", "m13", "q01", "t11", "39_x", "n06", "e04", "53", "30_x", "j03", "c01",
    "24_x", "y15",
];

/// Ending flags (byte variables) by girl; the 15th girl's button shows
/// after any of them.
pub const CLEAR: [usize; 14] = [
    1779, 1788, 1806, 1796, 1813, 1742, 1751, 1823, 1766, 1731, 1758, 1838, 1829, 1726,
];

const CARD: (u32, u32) = (316, 305);
const THUMB: (u32, u32) = (490, 275);
/// Picture frame: size and its place in `omake_p` (normal, hover,
/// pressed rows).
const FRAME: (u32, u32) = (490, 303);
const FRAME_X: u32 = 1180;
const FRAME_Y: [u32; 3] = [272, 575, 878];

#[derive(Debug, Clone)]
pub struct Album {
    /// With the update archive's pictures.
    pub r18: bool,
    /// The girl whose pictures are shown, or the grid.
    pub girl: Option<usize>,
    pub page: usize,
    /// Time of day for the pictures: 0 noon, 1 evening, 2 night.
    pub time: usize,
    open: Vec<Vec<bool>>,
    clear: [bool; 14],
    grid: Arc<Vec<Area>>,
    view: Arc<Vec<Area>>,
    /// Globals to set, taken by the caller after a click.
    set: Vec<(usize, i32)>,
}

impl Album {
    /// `g` reads a global; `returning` (global 16 is 1) opens on the girl
    /// and page of globals 10 and 17.
    pub fn load(assets: &Assets, bytes: &[u8], g: impl Fn(usize) -> i32) -> Result<Self> {
        let r18 = assets.exists(
            silky_format::game::Kind::Layer,
            &format!("cg_{}_r18.akb", SHEETS[0]),
        );
        let grid = area::parse(&assets.data("cgmode.area")?)?;
        let view = area::parse(&assets.data("cgview.area")?)?;
        let mut a = Self {
            r18,
            girl: None,
            page: 0,
            time: 0,
            open: Vec::new(),
            clear: [false; 14],
            grid: Arc::new(grid),
            view: Arc::new(view),
            set: Vec::new(),
        };
        a.refresh(bytes);
        if g(globals::RETURNING) == 1 {
            let girl = (g(globals::GIRL) - 1).clamp(0, GIRLS as i32 - 1) as usize;
            a.girl = Some(girl);
            a.page = (g(globals::PAGE) - 1).clamp(0, a.pages(girl) as i32 - 1) as usize;
        }
        let t = g(globals::TIME);
        a.time = if t < 1600 {
            0
        } else if t < 1800 {
            1
        } else {
            2
        };
        Ok(a)
    }

    /// Reads which pictures are open and which endings are seen.
    pub fn refresh(&mut self, bytes: &[u8]) {
        let b = |i: usize| bytes.get(i).copied().unwrap_or(0);
        self.open = self
            .pictures()
            .iter()
            .map(|l| l.iter().map(|&i| b(i) == 1).collect())
            .collect();
        self.clear = CLEAR.map(|i| b(i) != 0);
    }

    fn pictures(&self) -> &'static [&'static [usize]; GIRLS] {
        if self.r18 {
            &PICTURES_R18
        } else {
            &PICTURES
        }
    }

    fn pages(&self, girl: usize) -> usize {
        if self.r18 {
            PAGES_R18[girl]
        } else {
            PAGES[girl]
        }
    }

    /// The pictures of a girl, to open them all.
    pub fn girl_pictures(&self, girl: usize) -> &'static [usize] {
        self.pictures()[girl]
    }

    /// Globals set by the last click.
    pub fn take_globals(&mut self) -> Vec<(usize, i32)> {
        std::mem::take(&mut self.set)
    }

    fn cleared(&self, girl: usize) -> bool {
        match self.clear.get(girl) {
            Some(&c) => c,
            None => self.clear.iter().any(|&c| c),
        }
    }

    fn all_open(&self, girl: usize) -> bool {
        self.open[girl].iter().all(|&o| o)
    }

    /// The picture at a place of the page, if it is open.
    fn slot_open(&self, girl: usize, slot: usize) -> bool {
        slot < 9
            && self.open[girl]
                .get(self.page * 9 + slot)
                .copied()
                .unwrap_or(false)
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let areas = if self.girl.is_some() {
            &self.view
        } else {
            &self.grid
        };
        areas.iter().find(|a| a.contains(x, y)).map(|a| a.id)
    }

    pub fn click(&mut self, id: i32) -> Click {
        let (kind, i) = (id >> 8, (id & 0xff) as usize);
        match (kind, self.girl) {
            (1, _) if i > 0 && i < 4 => {
                let page = match (i, self.r18) {
                    (1, true) => 1,
                    (1, false) | (2, true) => 2,
                    _ => 3,
                };
                self.set = vec![
                    (globals::RETURNING, 0),
                    (globals::PAGE, 0),
                    (globals::PAGE_OF_EXTRAS, page),
                ];
                Click::Result(TO_PAGE)
            }
            (2, None) => Click::Result(LEAVE),
            (2, Some(_)) => {
                self.girl = None;
                Click::None
            }
            (3, Some(_)) if i < 3 && i != self.time => {
                self.time = i;
                self.set = vec![(globals::TIME, [1200, 1600, 1800][i])];
                Click::None
            }
            (0, None) if i < GIRLS => {
                self.girl = Some(i);
                self.page = 0;
                Click::None
            }
            (0, Some(girl)) if self.slot_open(girl, i) => {
                self.set = vec![
                    (globals::RETURNING, 0),
                    (globals::GIRL, girl as i32 + 1),
                    (globals::PICTURE, i as i32 + 1),
                    (globals::PAGE, self.page as i32 + 1),
                ];
                Click::Result(0)
            }
            (4, Some(_)) if i == 0 && self.page > 0 => {
                self.page -= 1;
                Click::None
            }
            (4, Some(girl)) if i == 1 && self.page + 1 < self.pages(girl) => {
                self.page += 1;
                Click::None
            }
            (5, Some(girl)) if self.cleared(girl) && !self.all_open(girl) => Click::Unlock,
            _ => Click::None,
        }
    }

    /// The secondary click: back to the grid, or nothing (the script's
    /// back button leaves).
    pub fn cancel(&mut self) -> bool {
        self.girl.take().is_some()
    }

    /// How much of the album is open, in percent as the header shows it.
    pub fn percent(&self) -> u32 {
        let total: usize = self.open.iter().map(Vec::len).sum();
        let seen: usize = self.open.iter().flatten().filter(|&&o| o).count();
        if seen == total {
            100
        } else if seen == 0 {
            0
        } else {
            ((seen as f32 / total as f32 * 100.0) as u32).max(1)
        }
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
        canvas.draw_part(&parts, (0, 0, 1920, 68), 0, 0, false, 255);
        self.draw_percent(canvas, &parts);
        let side = |canvas: &mut Canvas| {
            for i in 0..4u32 {
                let s = if i == 0 {
                    2
                } else {
                    state(TAB | i as i32).min(2)
                };
                let sx = if self.r18 {
                    i * 237
                } else if i == 3 {
                    continue;
                } else {
                    [0, 474, 711][i as usize]
                };
                canvas.draw_part(
                    &parts,
                    (sx, s * 163 + 272, 237, 163),
                    0,
                    i as i32 * 170 + 132,
                    false,
                    255,
                );
            }
            let s = state(BACK).min(2);
            canvas.draw_part(&parts, (948, s * 123 + 272, 218, 123), 10, 953, false, 255);
        };
        let Some(girl) = self.girl else {
            let list = assets.localized_image("chara_list.akb", language)?;
            for i in 0..GIRLS as u32 {
                let s = state(i as i32).min(2);
                let (sx, sy) = if i == 14 {
                    (2212, 0)
                } else {
                    (i % 7 * CARD.0, i / 7 * 1220)
                };
                let (x, y) = ((i % 5 * 337 + 240) as i32, (i / 5 * 313 + 98) as i32);
                canvas.draw_part(
                    &list,
                    (sx, sy + s * CARD.1, CARD.0, CARD.1),
                    x,
                    y,
                    false,
                    255,
                );
            }
            side(canvas);
            return Ok(());
        };
        let empty = assets.localized_image("thumb_emp.akb", language)?;
        let name = format!(
            "cg_{}{}.akb",
            SHEETS[girl],
            if self.r18 { "_r18" } else { "" }
        );
        let sheet = assets.image(&name)?;
        let g = girl as u32;
        let (tw, th) = THUMB;
        for slot in 0..9u32 {
            let k = self.page * 9 + slot as usize;
            if k >= self.open[girl].len() {
                break;
            }
            let (col, row) = (slot % 3, slot / 3);
            let (x, y) = ((col * 541 + 285) as i32, (row * 300 + 101) as i32);
            canvas.draw_part(&empty, (g % 4 * tw, g / 4 * th, tw, th), x, y, false, 255);
            if self.open[girl][k] {
                let cy = (row + self.page as u32 * 3) * th;
                canvas.draw_part(&sheet, (col * tw, cy, tw, th), x, y, false, 255);
                let s = state(slot as i32).min(2) as usize;
                let (fw, fh) = FRAME;
                canvas.draw_part(&parts, (FRAME_X, FRAME_Y[0], fw, fh), x, y - 23, false, 255);
                if s > 0 {
                    canvas.draw_part(&parts, (FRAME_X, FRAME_Y[s], fw, fh), x, y - 23, false, 255);
                }
            }
        }
        side(canvas);
        for i in 0..3u32 {
            let s = if i as usize == self.time {
                2
            } else {
                state(TIME | i as i32).min(2)
            };
            canvas.draw_part(
                &parts,
                (280 + i * 96, s * 83 + 924, 96, 83),
                (1053 + i * 120) as i32,
                995,
                false,
                255,
            );
        }
        let last = self.pages(girl) - 1;
        let s = if self.page == 0 {
            3
        } else {
            state(PREVIOUS).min(2)
        };
        canvas.draw_part(&parts, (0, s * 78 + 924, 140, 78), 1494, 999, false, 255);
        let s = if self.page == last {
            3
        } else {
            state(NEXT).min(2)
        };
        canvas.draw_part(&parts, (140, s * 78 + 924, 140, 78), 1694, 999, false, 255);
        if self.cleared(girl) {
            let sy = if self.all_open(girl) {
                521
            } else {
                state(UNLOCK).min(2) * 83 + 272
            };
            canvas.draw_part(&parts, (1670, sy, 280, 83), 285, 995, false, 255);
        }
        Ok(())
    }

    /// The percentage, right to left from x 1798, then the sign.
    fn draw_percent(&self, canvas: &mut Canvas, parts: &Arc<silky_format::akb::Image>) {
        let mut v = self.percent();
        let mut x = 1798;
        loop {
            canvas.draw_part(parts, (568 + v % 10 * 35, 977, 35, 49), x, 8, false, 255);
            x -= 44;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        canvas.draw_part(parts, (918, 977, 45, 49), 1843, 8, false, 255);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn album(r18: bool) -> Album {
        let mut a = Album {
            r18,
            girl: None,
            page: 0,
            time: 0,
            open: Vec::new(),
            clear: [false; 14],
            grid: Arc::new(Vec::new()),
            view: Arc::new(Vec::new()),
            set: Vec::new(),
        };
        a.refresh(&[]);
        a
    }

    #[test]
    fn tables() {
        // The pages hold every picture; the program's tables also carry
        // empty places (0), some past the last page.
        for (list, pages) in [(&PICTURES_R18, &PAGES_R18), (&PICTURES, &PAGES)] {
            for g in 0..GIRLS {
                let n = list[g].iter().filter(|&&i| i != 0).count();
                assert!(
                    n <= pages[g] * 9 && list[g].len() > (pages[g] - 1) * 9,
                    "girl {g}"
                );
            }
        }
    }

    #[test]
    fn clicks() {
        let mut a = album(true);
        assert_eq!(a.percent(), 0);
        assert!(matches!(a.click(BACK), Click::Result(LEAVE)));
        assert!(matches!(a.click(3), Click::None));
        assert_eq!(a.girl, Some(3));
        // Closed pictures do nothing.
        assert!(matches!(a.click(0), Click::None));
        let mut bytes = vec![0u8; 5000];
        bytes[4237] = 1;
        a.refresh(&bytes);
        assert!(matches!(a.click(1), Click::Result(0)));
        let set = a.take_globals();
        assert!(set.contains(&(globals::GIRL, 4)) && set.contains(&(globals::PICTURE, 2)));
        assert_eq!(a.percent(), 1);
        assert!(matches!(a.click(NEXT), Click::None));
        assert_eq!(a.page, 1);
        assert!(matches!(a.click(NEXT), Click::None));
        assert_eq!(a.page, 1);
        assert!(matches!(a.click(UNLOCK), Click::None));
        bytes[1796] = 1;
        a.refresh(&bytes);
        assert!(matches!(a.click(UNLOCK), Click::Unlock));
        assert!(matches!(a.click(TAB | 2), Click::Result(TO_PAGE)));
        assert!(a.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 2)));
        assert!(a.cancel());
        assert!(!a.cancel());
    }
}
