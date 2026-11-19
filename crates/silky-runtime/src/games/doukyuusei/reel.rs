//! The memory reel of 同級生 Remake (`Game.51`, the program's scene mode),
//! the second page of the extras; the hero's computer opens it as well.
//!
//! Read from the executable. It is built like the album (see
//! [`super::album`]): a grid of the 15 girls (`scmode.area`, the same
//! cards), and a girl opens her scenes (`cgview.area`), nine to a page.
//! Each scene is a byte variable from the program's tables (one with the
//! update archive's scenes, one without), open when it is not 0 (the album
//! asks for 1). A closed one shows the girl's empty card of
//! `thumb_emp_<lang>.akb`; an open one its cell of the girl's
//! `sc_<name>[_r18].akb` sheet, its caption from `scene_ch[_r18]_<lang>.akb`
//! and a frame of `omake_p_<lang>.akb`.
//!
//! When the screen comes from the hero's computer (global 14 is 1) it has
//! no header, side tabs or unlock button, and the girls' pictures show no
//! percentage. Clicking an open scene answers 0 with global 10 (the girl,
//! counted from 1) and global 13 (the scene, counted from 1 over all
//! pages) set and global 16 (coming back) cleared; the script plays it and
//! sets 16 again so the reel opens on that girl and page. The side tabs
//! answer 100 with global 12 naming the next page of the extras; back
//! answers -1.
//! See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::area::{self, Area};

use super::album::{self, cleared, draw_cards, draw_percent, endings, GIRLS};
use super::room::Click;
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;

pub use super::album::{BACK, LEAVE, NEXT, PREVIOUS, TAB, TO_PAGE, UNLOCK};

/// Globals the reel reads and writes.
pub mod globals {
    pub const GIRL: usize = 10;
    pub const PAGE_OF_EXTRAS: usize = 12;
    pub const SCENE: usize = 13;
    /// Opened from the hero's computer (1) instead of the extras.
    pub const COMPUTER: usize = 14;
    pub const RETURNING: usize = 16;
    pub const PAGE: usize = 17;
}

/// Scenes by girl with the update archive (byte variables).
const SCENES_R18: [&[usize]; GIRLS] = [
    &[1767, 1768, 1774, 1775, 1776, 1777, 1778, 1779],
    &[1780, 1781, 1782, 1784, 1785, 1864, 1786, 1787, 1788],
    &[1797, 1798, 1799, 1800, 1802, 1805, 1806],
    &[1789, 1791, 1792, 1793, 1794, 1795, 1796],
    &[1807, 1809, 1810, 1811, 1812, 1813],
    &[1732, 1733, 1734, 1735, 1736, 1742],
    &[1743, 1744, 1745, 1746, 1747, 1749, 1750, 1751],
    &[1814, 1815, 1816, 1817, 1820, 1821, 1823],
    &[1759, 1760, 1761, 1762, 1764, 1765, 1766],
    &[1727, 1728, 1729, 1730, 1731],
    &[1752, 1753, 1755, 1756, 1757, 1758],
    &[1830, 1831, 1832, 1833, 1834, 1836, 1837, 1838],
    &[1824, 1825, 1826, 1827, 1828, 1829],
    &[1720, 1721, 1723, 1724, 1725, 1726],
    &[
        1839, 1840, 1841, 1842, 1843, 1844, 1845, 1846, 1847, 1848, 1849, 1850, 1851,
    ],
];

/// Scenes by girl without the update archive.
const SCENES: [&[usize]; GIRLS] = [
    &[1767, 1768, 1774, 1775, 1776, 1778, 1779],
    &[1780, 1781, 1782, 1784, 1785, 1787, 1788],
    &[1797, 1798, 1799, 1800, 1805, 1806],
    &[1789, 1791, 1792, 1793, 1794, 1795, 1796],
    &[1807, 1809, 1812, 1813],
    &[1732, 1733, 1734, 1735, 1736, 1742],
    &[1743, 1744, 1745, 1746, 1747, 1750, 1751],
    &[1814, 1815, 1816, 1817, 1820, 1821, 1823],
    &[1760, 1761, 1762, 1764, 1766],
    &[1727, 1728, 1730, 1731],
    &[1752, 1753, 1755, 1756, 1758],
    &[1830, 1831, 1832, 1834, 1836, 1837, 1838],
    &[1824, 1825, 1826, 1827, 1828, 1829],
    &[1720, 1721, 1723, 1726],
    &[],
];

/// Pages by girl, with and without the update archive.
const PAGES_R18: [usize; GIRLS] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2];
const PAGES: [usize; GIRLS] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0];

/// Scene sheets by girl (`sc_<name>.akb`, `_r18` added with the update
/// archive).
const SHEETS: [&str; GIRLS] = [
    "mai", "misa", "satomi", "miho", "kuru", "ako", "mako", "chiha", "kao", "yayo", "hiro",
    "natsu", "yoshi", "reiko", "sp",
];

const THUMB: (u32, u32) = (490, 275);
/// Caption size.
const CAPTION: (u32, u32) = (490, 50);
/// Picture frame: size and its place in `omake_p` (normal, hover,
/// pressed rows).
const FRAME: (u32, u32) = (490, 303);
const FRAME_X: u32 = 1180;
const FRAME_Y: [u32; 3] = [272, 575, 878];

#[derive(Debug, Clone)]
pub struct Reel {
    /// With the update archive's scenes.
    pub r18: bool,
    /// Opened from the hero's computer, not the extras.
    pub computer: bool,
    /// The girl whose scenes are shown, or the grid.
    pub girl: Option<usize>,
    pub page: usize,
    open: Vec<Vec<bool>>,
    clear: [bool; 14],
    grid: Arc<Vec<Area>>,
    view: Arc<Vec<Area>>,
    /// Globals to set, taken by the caller after a click.
    set: Vec<(usize, i32)>,
}

impl Reel {
    /// `g` reads a global; `returning` (global 16 is 1) opens on the girl
    /// and page of globals 10 and 13.
    pub fn load(assets: &Assets, bytes: &[u8], g: impl Fn(usize) -> i32) -> Result<Self> {
        let r18 = album::r18(assets);
        let grid = area::parse(&assets.data("scmode.area")?)?;
        let view = area::parse(&assets.data("cgview.area")?)?;
        let mut r = Self {
            r18,
            computer: g(globals::COMPUTER) == 1,
            girl: None,
            page: 0,
            open: Vec::new(),
            clear: [false; 14],
            grid: Arc::new(grid),
            view: Arc::new(view),
            set: Vec::new(),
        };
        r.refresh(bytes);
        if g(globals::RETURNING) == 1 {
            let girl = (g(globals::GIRL) - 1).clamp(0, GIRLS as i32 - 1) as usize;
            r.girl = Some(girl);
            r.page = ((g(globals::SCENE) - 1).max(0) / 9) as usize;
            r.page = r.page.min(r.pages(girl).saturating_sub(1));
        }
        Ok(r)
    }

    /// Reads which scenes are open and which endings are seen.
    pub fn refresh(&mut self, bytes: &[u8]) {
        let b = |i: usize| bytes.get(i).copied().unwrap_or(0);
        self.open = self
            .scenes()
            .iter()
            .map(|l| l.iter().map(|&i| b(i) != 0).collect())
            .collect();
        self.clear = endings(bytes);
    }

    fn scenes(&self) -> &'static [&'static [usize]; GIRLS] {
        if self.r18 {
            &SCENES_R18
        } else {
            &SCENES
        }
    }

    fn pages(&self, girl: usize) -> usize {
        if self.r18 {
            PAGES_R18[girl]
        } else {
            PAGES[girl]
        }
    }

    /// The cards of the grid: the 15th only with the update archive.
    fn cards(&self) -> usize {
        if self.r18 {
            GIRLS
        } else {
            GIRLS - 1
        }
    }

    /// The scenes of a girl, to open them all.
    pub fn girl_scenes(&self, girl: usize) -> &'static [usize] {
        self.scenes()[girl]
    }

    /// Globals set by the last click.
    pub fn take_globals(&mut self) -> Vec<(usize, i32)> {
        std::mem::take(&mut self.set)
    }

    fn all_open(&self, girl: usize) -> bool {
        self.open[girl].iter().all(|&o| o)
    }

    /// Whether the scene at a place of the page is open.
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
            // The second tab is the reel itself, the current page.
            (1, _) if i < 4 && i != 1 && !self.computer => {
                self.set = if i == 3 {
                    vec![(globals::PAGE_OF_EXTRAS, 3)]
                } else {
                    vec![
                        (globals::RETURNING, 0),
                        (globals::PAGE, 0),
                        (globals::PAGE_OF_EXTRAS, i as i32),
                    ]
                };
                Click::Result(TO_PAGE)
            }
            (2, None) => Click::Result(LEAVE),
            (2, Some(_)) => {
                self.girl = None;
                Click::None
            }
            (0, None) if i < self.cards() => {
                self.girl = Some(i);
                self.page = 0;
                Click::None
            }
            (0, Some(girl)) if self.slot_open(girl, i) => {
                self.set = vec![
                    (globals::RETURNING, 0),
                    (globals::GIRL, girl as i32 + 1),
                    (globals::SCENE, (self.page * 9 + i) as i32 + 1),
                ];
                Click::Result(0)
            }
            (4, Some(girl)) if self.pages(girl) != 1 => match i {
                0 if self.page > 0 => {
                    self.page -= 1;
                    Click::None
                }
                1 if self.page + 1 < self.pages(girl) => {
                    self.page += 1;
                    Click::None
                }
                _ => Click::None,
            },
            (5, Some(girl))
                if !self.computer && cleared(&self.clear, girl) && !self.all_open(girl) =>
            {
                Click::Unlock
            }
            _ => Click::None,
        }
    }

    /// The secondary click: from a girl's scenes back to the grid
    /// (`None`), from the grid it leaves like back.
    pub fn cancel(&mut self) -> Option<i32> {
        match self.girl.take() {
            Some(_) => None,
            None => Some(LEAVE),
        }
    }

    /// How much of the reel is open, in percent as the header shows it.
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
        if !self.computer {
            canvas.draw_part(&parts, (0, 68, 1920, 68), 0, 0, false, 255);
        }
        let side = |canvas: &mut Canvas| {
            if !self.computer {
                for i in 0..4u32 {
                    let s = if i == 1 {
                        2
                    } else {
                        state(TAB | i as i32).min(2)
                    };
                    canvas.draw_part(
                        &parts,
                        (i * 237, s * 163 + 272, 237, 163),
                        0,
                        i as i32 * 170 + 132,
                        false,
                        255,
                    );
                }
            }
            let s = state(BACK).min(2);
            canvas.draw_part(&parts, (948, s * 123 + 272, 218, 123), 10, 953, false, 255);
        };
        let Some(girl) = self.girl else {
            draw_percent(canvas, &parts, self.percent());
            draw_cards(canvas, assets, language, self.cards(), &state)?;
            side(canvas);
            return Ok(());
        };
        if !self.computer {
            draw_percent(canvas, &parts, self.percent());
        }
        let empty = assets.localized_image("thumb_emp.akb", language)?;
        let name = format!(
            "sc_{}{}.akb",
            SHEETS[girl],
            if self.r18 { "_r18" } else { "" }
        );
        let sheet = assets.image(&name)?;
        let captions = assets.localized_image(
            if self.r18 {
                "scene_ch_r18.akb"
            } else {
                "scene_ch.akb"
            },
            language,
        )?;
        let g = girl as u32;
        let (tw, th) = THUMB;
        let (cw, ch) = CAPTION;
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
                let line = k as u32 + g / 4 * 9;
                canvas.draw_part(
                    &captions,
                    (g % 4 * cw, line * ch, cw, ch),
                    x,
                    (row * 300 + 319) as i32,
                    false,
                    255,
                );
                let s = state(slot as i32).min(2) as usize;
                let (fw, fh) = FRAME;
                canvas.draw_part(&parts, (FRAME_X, FRAME_Y[0], fw, fh), x, y - 23, false, 255);
                if s > 0 {
                    canvas.draw_part(&parts, (FRAME_X, FRAME_Y[s], fw, fh), x, y - 23, false, 255);
                }
            }
        }
        side(canvas);
        let pages = self.pages(girl);
        if pages != 1 {
            let s = if self.page == 0 {
                3
            } else {
                state(PREVIOUS).min(2)
            };
            canvas.draw_part(&parts, (0, s * 78 + 924, 140, 78), 1494, 999, false, 255);
            let s = if self.page + 1 >= pages {
                3
            } else {
                state(NEXT).min(2)
            };
            canvas.draw_part(&parts, (140, s * 78 + 924, 140, 78), 1694, 999, false, 255);
        }
        if !self.computer && cleared(&self.clear, girl) {
            let sy = if self.all_open(girl) {
                853
            } else {
                state(UNLOCK).min(2) * 83 + 604
            };
            canvas.draw_part(&parts, (1670, sy, 280, 83), 285, 995, false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reel(r18: bool, computer: bool) -> Reel {
        let mut r = Reel {
            r18,
            computer,
            girl: None,
            page: 0,
            open: Vec::new(),
            clear: [false; 14],
            grid: Arc::new(Vec::new()),
            view: Arc::new(Vec::new()),
            set: Vec::new(),
        };
        r.refresh(&[]);
        r
    }

    #[test]
    fn tables() {
        // The pages hold every scene, and the last page is not empty.
        for (list, pages) in [(&SCENES_R18, &PAGES_R18), (&SCENES, &PAGES)] {
            for g in 0..GIRLS {
                assert_eq!(list[g].len().div_ceil(9), pages[g], "girl {g}");
            }
        }
        // The ending flags are scenes of their girls.
        for g in 0..14 {
            assert!(SCENES[g].contains(&album::CLEAR[g]), "girl {g}");
            assert!(SCENES_R18[g].contains(&album::CLEAR[g]), "girl {g}");
        }
    }

    #[test]
    fn clicks() {
        let mut r = reel(true, false);
        assert_eq!(r.percent(), 0);
        assert!(matches!(r.click(3), Click::None));
        assert_eq!(r.girl, Some(3));
        // Closed scenes do nothing.
        assert!(matches!(r.click(0), Click::None));
        let mut bytes = vec![0u8; 2000];
        bytes[1791] = 2;
        r.refresh(&bytes);
        assert!(matches!(r.click(1), Click::Result(0)));
        let set = r.take_globals();
        assert!(set.contains(&(globals::GIRL, 4)) && set.contains(&(globals::SCENE, 2)));
        assert!(set.contains(&(globals::RETURNING, 0)));
        assert_eq!(r.percent(), 1);
        // One page: the arrows do nothing.
        assert!(matches!(r.click(NEXT), Click::None));
        assert_eq!(r.page, 0);
        assert!(matches!(r.click(UNLOCK), Click::None));
        assert!(matches!(r.click(TAB | 1), Click::None));
        assert!(matches!(r.click(TAB | 3), Click::Result(TO_PAGE)));
        assert_eq!(r.take_globals(), vec![(globals::PAGE_OF_EXTRAS, 3)]);
        assert!(matches!(r.click(TAB | 2), Click::Result(TO_PAGE)));
        let set = r.take_globals();
        assert!(set.contains(&(globals::PAGE_OF_EXTRAS, 2)) && set.contains(&(globals::PAGE, 0)));
        assert!(matches!(r.click(TAB), Click::Result(TO_PAGE)));
        assert!(r.take_globals().contains(&(globals::PAGE_OF_EXTRAS, 0)));
        // The 15th girl has two pages.
        assert_eq!(r.cancel(), None);
        assert!(matches!(r.click(14), Click::None));
        assert!(matches!(r.click(NEXT), Click::None));
        assert_eq!(r.page, 1);
        assert_eq!(r.cancel(), None);
        assert_eq!(r.cancel(), Some(LEAVE));
        // The ending opens the unlock button.
        let mut r = reel(true, false);
        r.click(3);
        bytes[1796] = 1;
        r.refresh(&bytes);
        assert!(matches!(r.click(UNLOCK), Click::Unlock));
        assert!(matches!(r.click(BACK), Click::None));
        assert_eq!(r.girl, None);
        assert!(matches!(r.click(BACK), Click::Result(LEAVE)));
        // Without the update archive the 15th card is not there.
        let mut r = reel(false, false);
        assert!(matches!(r.click(14), Click::None));
        assert_eq!(r.girl, None);
    }

    #[test]
    fn computer() {
        let mut r = reel(true, true);
        for id in [TAB, TAB | 2, TAB | 3] {
            assert!(matches!(r.click(id), Click::None));
        }
        assert!(r.take_globals().is_empty());
        let mut bytes = vec![0u8; 2000];
        bytes[1796] = 1;
        r.refresh(&bytes);
        r.click(3);
        assert!(matches!(r.click(UNLOCK), Click::None));
        assert!(matches!(r.click(BACK), Click::None));
        assert!(matches!(r.click(BACK), Click::Result(LEAVE)));
    }
}
