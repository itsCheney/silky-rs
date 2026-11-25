//! The game's own scene mode (`Game.31`, the 回想 screen): 78 scenes on six
//! pages of 15 cards, a filmstrip that plays while the pointer is on a
//! card, page arrows, a completion counter and the frame the CG and music
//! screens share (see [`super::mode`]).
//!
//! The script draws the background before it calls this; the screen draws
//! the frame and its own parts over it. A card is open when the byte
//! 3100 + scene is 1; a closed card shows only the film frame. Picking an
//! open card answers the scene number (0 to 77); the script then runs
//! `nanairo_scene%03d.mes` and opens the screen again on the same page.

use std::cell::Cell;
use std::sync::Arc;

use silky_format::akb::Image;

use super::mode::{self, Frame, Mode, Phase, BACK, FRAME_MS, QUIT, SLIDE_OUT_FRAMES};
use crate::assets::Assets;
use crate::games::kimaten1::{Click, Look};
use crate::render::Canvas;
use crate::Result;

pub const SHEET: &str = "SCENEmode.akb";
/// The scene titles, 222 x 25 each, in the same cells as the thumbnails.
const TITLES: &str = "SCENEmode2.akb";
/// The thumbnails, 222 x 125 each, five to a row.
const THUMBS: &str = "scene_thumb.akb";

pub const SCENES: usize = 78;
pub const PER_PAGE: usize = 15;
pub const PAGES: usize = 6;
/// The first byte flag: scene `i` is open when byte `FLAG_FIRST + i` is 1.
pub const FLAG_FIRST: usize = 3100;

/// Frames of each scene's filmstrip (`scene_thumbNNN.akb` holds that many
/// 222 x 125 pictures side by side).
pub const FRAMES: [u8; SCENES] = [
    3, 1, 3, 2, 2, 2, 1, 2, 1, 1, 2, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1, 1, 1, 2, 1, 2, 2, 1, 2, 1, 1, 1,
    3, 3, 1, 1, 2, 1, 1, 2, 2, 2, 1, 2, 1, 1, 2, 3, 2, 2, 1, 2, 2, 1, 1, 2, 1, 1, 1, 2, 1, 1, 2, 1,
    1, 1, 1, 3, 1, 2, 1, 1, 1, 1, 2, 2, 1, 1,
];

/// Area ids: the cards are 0..15.
const PREV: i32 = 0x100;
const NEXT: i32 = 0x101;

/// The area of card `slot` as `SCMODE.AREA` has it: 225 x 127.
fn card_rect(slot: usize) -> (i32, i32, i32, i32) {
    let (x, y) = (28 + 250 * (slot % 5) as i32, 140 + 156 * (slot / 5) as i32);
    (x, y, x + 225, y + 127)
}

/// The areas (left, top, right, bottom).
fn rect(id: i32) -> Option<(i32, i32, i32, i32)> {
    Some(match id {
        0..=14 => card_rect(id as usize),
        PREV => (551, 609, 583, 641),
        NEXT => (697, 609, 729, 641),
        0xf00 => (25, 6, 118, 117),
        0xf01 => (166, 6, 259, 117),
        0xf02 => (305, 6, 398, 117),
        BACK => (1077, 605, 1170, 716),
        QUIT => (1178, 605, 1271, 716),
        _ => return None,
    })
}

/// How long a filmstrip picture stays, and how long the change to the
/// next one takes, in ms. The first change starts 500 ms after the
/// pointer came onto the card.
const HOLD_MS: u64 = 1000;
const FADE_MS: u64 = 1000;
const HEAD_START_MS: u64 = 500;

/// What the filmstrip shows `ms` after the pointer came onto a card of
/// `frames` pictures: the picture, the one fading in over it and how far
/// (0 to 255) it has come.
pub fn filmstrip(ms: u64, frames: usize) -> (usize, usize, u8) {
    let frames = frames.max(1);
    let t = ms + HEAD_START_MS;
    let period = HOLD_MS + FADE_MS;
    let cur = (t / period) as usize % frames;
    let local = t % period;
    if frames == 1 || local <= HOLD_MS {
        return (cur, cur, 0);
    }
    let blend = ((local - HOLD_MS) * 255 / FADE_MS).min(255) as u8;
    (cur, (cur + 1) % frames, blend)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    No,
    /// Asked to leave; the slide out starts when the clock is next seen.
    Pending(i32),
    At(u64, i32),
}

#[derive(Debug, Clone)]
pub struct SceneMode {
    open: [bool; SCENES],
    /// Whether each tab (CG, music, scene) is open.
    tabs: [u8; 3],
    page: usize,
    leave: Leave,
    sound: Option<&'static str>,
    /// The card the pointer is on and the time it came onto it.
    hover: Cell<Option<(i32, u64)>>,
}

impl SceneMode {
    /// `flags` are the bytes 3100.. (1 when open), `tabs` the tab bytes in
    /// the order of the strip (CG, music, scene), `page` the page to open
    /// on (the page of the last scene picked).
    pub fn open(flags: [u8; SCENES], tabs: [u8; 3], page: usize) -> Self {
        SceneMode {
            open: flags.map(|f| f == 1),
            tabs,
            page: if page < PAGES { page } else { 0 },
            leave: Leave::No,
            sound: None,
            hover: Cell::new(None),
        }
    }

    pub fn page(&self) -> usize {
        self.page
    }

    /// How many scenes are open.
    pub fn open_count(&self) -> usize {
        self.open.iter().filter(|&&o| o).count()
    }

    /// The completion shown: the share of open scenes in per cent, cut
    /// down; 0 with none open.
    pub fn percent(&self) -> u32 {
        match self.open_count() {
            0 => 0,
            n => mode::percent(n, SCENES),
        }
    }

    fn scene_of(&self, slot: usize) -> Option<usize> {
        let i = self.page * PER_PAGE + slot;
        (i < SCENES).then_some(i)
    }

    fn unlocked(&self, slot: usize) -> bool {
        self.scene_of(slot).is_some_and(|i| self.open[i])
    }

    fn tab_open(&self, i: usize) -> bool {
        self.tabs[i] != 0
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        let inside =
            |id: i32| rect(id).is_some_and(|(l, t, r, b)| x >= l && x < r && y >= t && y < b);
        (0..PER_PAGE as i32)
            .filter(|&s| self.scene_of(s as usize).is_some())
            .chain([PREV, NEXT, 0xf00, 0xf01, 0xf02, BACK, QUIT])
            .find(|&id| inside(id))
    }

    /// Where an arrow key puts the pointer: the button it is on is `at`.
    pub fn arrow(&self, at: Option<i32>, key: super::keys::Arrow) -> Option<i32> {
        if self.leave != Leave::No {
            return None;
        }
        super::keys::grid(at, key, self.page, &|k| self.scene_of(k).is_some())
    }

    /// The middle of a button, where the pointer goes.
    pub fn centre(&self, id: i32) -> Option<(i32, i32)> {
        rect(id).map(super::keys::middle)
    }

    pub fn enabled(&self, id: i32) -> bool {
        match id {
            0..=14 => self.unlocked(id as usize),
            PREV => self.page > 0,
            NEXT => self.page + 1 < PAGES,
            0xf00..=0xf02 => {
                let i = (id - mode::TAB_FIRST) as usize;
                i != Mode::Scene.tab() && self.tab_open(i)
            }
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.leave != Leave::No {
            return Click::None;
        }
        match id {
            0..=14 => {
                if let (true, Some(i)) = (self.unlocked(id as usize), self.scene_of(id as usize)) {
                    self.sound = Some(mode::OK_SOUND);
                    self.leave = Leave::Pending(i as i32);
                }
                Click::None
            }
            PREV | NEXT => {
                if self.enabled(id) {
                    self.sound = Some(mode::OK_SOUND);
                    self.page = if id == PREV {
                        self.page - 1
                    } else {
                        self.page + 1
                    };
                    self.hover.set(None);
                }
                Click::None
            }
            0xf00 | 0xf01 => {
                let i = (id - mode::TAB_FIRST) as usize;
                if self.tab_open(i) {
                    self.sound = Some(mode::OK_SOUND);
                    let to = if i == Mode::Cg.tab() {
                        Mode::Cg
                    } else {
                        Mode::Music
                    };
                    self.leave = Leave::Pending(to.code());
                }
                Click::None
            }
            BACK => {
                self.sound = Some(mode::OK_SOUND);
                self.leave = Leave::Pending(mode::LEAVE);
                Click::None
            }
            QUIT => {
                self.sound = Some(mode::OK_SOUND);
                Click::Bar(crate::ui::bar::QUIT)
            }
            _ => Click::None,
        }
    }

    /// A right click or Escape: leaves with the cancel sound.
    pub fn cancel(&mut self) -> Click {
        if self.leave == Leave::No {
            self.sound = Some(mode::CANCEL_SOUND);
            self.leave = Leave::Pending(mode::LEAVE);
        }
        Click::None
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    /// Headless run: the first thing to press.
    pub fn auto_pick(&self) -> Option<i32> {
        Some(BACK)
    }

    /// Whether the screen changes by itself at `ms` since it opened: the
    /// frame sliding in or out, or a filmstrip of several pictures playing.
    pub fn animating(&self, ms: u64) -> bool {
        self.leave != Leave::No
            || ms < mode::SLIDE_IN_FRAMES * FRAME_MS
            || self.hover.get().is_some_and(|(id, _)| {
                self.scene_of(id as usize)
                    .is_some_and(|i| FRAMES[i] > 1 && self.open[i])
            })
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

    fn arrow_state(&self, look: &Look, id: i32) -> u32 {
        if !self.enabled(id) {
            3
        } else if look.pressed == Some(id) {
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
        Frame {
            sheet: &sheet,
            mode: Mode::Scene,
            open: [self.tab_open(0), self.tab_open(1), true],
            hover: look.hover,
            pressed: look.pressed,
            phase,
        }
        .draw(canvas);
        let titles = assets.image(TITLES)?;
        let thumbs = assets.image(THUMBS)?;
        self.draw_page(canvas, &sheet, &titles, &thumbs, look, alpha);
        self.draw_hover(canvas, assets, &sheet, &titles, look, alpha);
        Ok(())
    }

    fn draw_page(
        &self,
        canvas: &mut Canvas,
        sheet: &Arc<Image>,
        titles: &Arc<Image>,
        thumbs: &Arc<Image>,
        look: &Look,
        alpha: u8,
    ) {
        // The line under the frame's top bar.
        canvas.draw_part(sheet, (0, 90, 830, 1), 450, 116, false, alpha);
        for slot in 0..PER_PAGE {
            let Some(i) = self.scene_of(slot) else {
                continue;
            };
            let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
            let (x, y) = (24 + 250 * col, 129 + 156 * row);
            canvas.draw_part(sheet, (465, 301, 232, 149), x, y, false, alpha);
            if col != 4 {
                canvas.draw_part(sheet, (697, 301, 18, 149), x + 232, y, false, alpha);
            }
            if row != 2 {
                canvas.draw_part(sheet, (715, 428, 232, 7), x, y + 149, false, alpha);
                if col != 4 {
                    canvas.draw_part(sheet, (715, 435, 18, 7), x + 232, y + 149, false, alpha);
                }
            }
            if self.open[i] {
                let (cx, ry) = (222 * (i % 5) as u32, (i / 5) as u32);
                canvas.draw_part(
                    thumbs,
                    (cx, 125 * ry, 222, 125),
                    x + 5,
                    y + 12,
                    false,
                    alpha,
                );
                canvas.draw_part(titles, (cx, 25 * ry, 222, 25), x + 5, y + 108, false, alpha);
            }
        }
        for (id, at, sx) in [(PREV, (551, 609), 465), (NEXT, (697, 609), 497)] {
            let st = self.arrow_state(look, id);
            canvas.draw_part(sheet, (sx, 173 + 32 * st, 32, 32), at.0, at.1, false, alpha);
        }
        let digit = |canvas: &mut Canvas, n: usize, at: (i32, i32)| {
            canvas.draw_part(
                sheet,
                (826 + 32 * n as u32, 135, 32, 36),
                at.0,
                at.1,
                false,
                alpha,
            );
        };
        digit(canvas, self.page + 1, (596, 607));
        canvas.draw_part(sheet, (903, 91, 23, 41), 629, 604, false, alpha);
        digit(canvas, PAGES, (652, 607));
        mode::draw_percent(canvas, sheet, self.percent(), alpha);
    }

    /// The card under the pointer: the filmstrip picture over its
    /// thumbnail, the title bar again, and the hover frame.
    fn draw_hover(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        sheet: &Arc<Image>,
        titles: &Arc<Image>,
        look: &Look,
        alpha: u8,
    ) {
        let slot = match look.hover {
            Some(id @ 0..=14) if self.leave == Leave::No && self.enabled(id) => id,
            _ => {
                self.hover.set(None);
                return;
            }
        };
        let Some(i) = self.scene_of(slot as usize) else {
            return;
        };
        let since = match self.hover.get() {
            Some((id, t)) if id == slot => t,
            _ => {
                self.hover.set(Some((slot, look.ms)));
                look.ms
            }
        };
        let (x, y) = card_rect(slot as usize).into_xy();
        let strip = assets.image(&format!("scene_thumb{i:03}.akb")).ok();
        if let Some(strip) = strip {
            let (cur, next, blend) = filmstrip(look.ms.saturating_sub(since), FRAMES[i] as usize);
            let part = |f: usize| (222 * f as u32, 0, 222, 125);
            canvas.draw_part(&strip, part(cur), x + 1, y + 1, false, alpha);
            if blend > 0 {
                let a = (blend as u32 * alpha as u32 / 255) as u8;
                canvas.draw_part(&strip, part(next), x + 1, y + 1, false, a);
            }
        }
        let (cx, ry) = (222 * (i % 5) as u32, (i / 5) as u32);
        canvas.draw_part(titles, (cx, 25 * ry, 222, 25), x + 1, y + 97, false, alpha);
        canvas.draw_part(sheet, (715, 301, 224, 127), x, y, false, alpha);
    }
}

trait IntoXy {
    fn into_xy(self) -> (i32, i32);
}

impl IntoXy for (i32, i32, i32, i32) {
    fn into_xy(self) -> (i32, i32) {
        (self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> SceneMode {
        SceneMode::open([1; SCENES], [1; 3], 0)
    }

    #[test]
    fn frame_counts() {
        assert_eq!(FRAMES.len(), 78);
        assert_eq!(&FRAMES[..8], &[3, 1, 3, 2, 2, 2, 1, 2]);
        assert_eq!(FRAMES[77], 1);
        assert!(FRAMES.iter().all(|&f| (1..=3).contains(&f)));
    }

    #[test]
    fn cards_and_hit_areas() {
        let m = all();
        assert_eq!(m.hit(30, 142), Some(0));
        assert_eq!(m.hit(1030, 142), Some(4));
        assert_eq!(m.hit(280, 300), Some(6));
        assert_eq!(m.hit(1250, 576), Some(14));
        // Between the cards and under the title bars there is no card.
        assert_eq!(m.hit(26, 142), None);
        assert_eq!(m.hit(30, 268), None);
        assert_eq!(m.hit(560, 620), Some(PREV));
        assert_eq!(m.hit(700, 620), Some(NEXT));
        assert_eq!(m.hit(30, 10), Some(0xf00));
        assert_eq!(m.hit(200, 10), Some(0xf01));
        assert_eq!(m.hit(350, 10), Some(0xf02));
        assert_eq!(m.hit(1100, 650), Some(BACK));
        assert_eq!(m.hit(1200, 650), Some(QUIT));
    }

    #[test]
    fn the_last_page_has_three_cards() {
        let mut m = all();
        for _ in 0..5 {
            m.click(NEXT);
        }
        assert_eq!(m.page(), 5);
        assert!(!m.enabled(NEXT) && m.enabled(PREV));
        assert_eq!(m.hit(30, 142), Some(0));
        assert_eq!(m.hit(530, 142), Some(2));
        assert_eq!(m.hit(780, 142), None);
        assert_eq!(m.hit(30, 300), None);
        m.click(2);
        assert_eq!(m.settle(), Some(Click::Result(77)));
    }

    #[test]
    fn pages_turn_with_the_arrows() {
        let mut m = all();
        assert!(!m.enabled(PREV) && m.enabled(NEXT));
        m.click(PREV);
        assert_eq!((m.page(), m.take_sound()), (0, None));
        m.click(NEXT);
        assert_eq!(m.page(), 1);
        assert_eq!(m.take_sound().as_deref(), Some(mode::OK_SOUND));
        m.click(PREV);
        assert_eq!(m.page(), 0);
        // The page a scene was picked on comes back.
        assert_eq!(SceneMode::open([1; SCENES], [1; 3], 3).page(), 3);
        assert_eq!(SceneMode::open([1; SCENES], [1; 3], 9).page(), 0);
    }

    #[test]
    fn a_card_answers_its_scene() {
        let mut m = all();
        m.click(NEXT);
        m.click(NEXT);
        m.take_sound();
        // Page 2, card 7: scene 37.
        assert_eq!(m.click(7), Click::None);
        assert_eq!(m.take_sound().as_deref(), Some(mode::OK_SOUND));
        // The answer waits for the slide out.
        assert_eq!(m.tick(1000), None);
        assert_eq!(m.tick(1000 + 11 * 16), None);
        assert_eq!(m.tick(1000 + 12 * 16), Some(Click::Result(37)));
        // Nothing else is taken while it leaves.
        m.click(0);
        assert_eq!(m.settle(), Some(Click::Result(37)));
    }

    #[test]
    fn closed_cards_do_nothing() {
        let mut flags = [0; SCENES];
        flags[1] = 1;
        let mut m = SceneMode::open(flags, [1; 3], 0);
        assert!(!m.enabled(0) && m.enabled(1));
        m.click(0);
        assert_eq!((m.settle(), m.take_sound()), (None, None));
        assert_eq!(m.open_count(), 1);
        m.click(1);
        assert_eq!(m.settle(), Some(Click::Result(1)));
    }

    #[test]
    fn completion_counter() {
        assert_eq!(SceneMode::open([0; SCENES], [1; 3], 0).percent(), 0);
        let mut flags = [0; SCENES];
        flags[..39].fill(1);
        assert_eq!(SceneMode::open(flags, [1; 3], 0).percent(), 50);
        flags[0] = 0;
        flags[1] = 0;
        flags[39..40].fill(1);
        assert_eq!(SceneMode::open(flags, [1; 3], 0).open_count(), 38);
        assert_eq!(all().percent(), 100);
        let mut one = [0; SCENES];
        one[5] = 1;
        assert_eq!(SceneMode::open(one, [1; 3], 0).percent(), 1);
    }

    #[test]
    fn tabs_leave_with_the_mode_codes() {
        let mut m = all();
        // The scene tab itself does nothing.
        assert!(!m.enabled(0xf02));
        m.click(0xf02);
        assert_eq!((m.settle(), m.take_sound()), (None, None));
        m.click(0xf00);
        assert_eq!(m.take_sound().as_deref(), Some(mode::OK_SOUND));
        assert_eq!(m.settle(), Some(Click::Result(0x100)));
        let mut m = all();
        m.click(0xf01);
        assert_eq!(m.settle(), Some(Click::Result(0x101)));
        assert_eq!(mode::next(Mode::Scene, 0x100), mode::Next::Go(Mode::Cg));
        assert_eq!(mode::next(Mode::Scene, 0x101), mode::Next::Go(Mode::Music));
        assert_eq!(mode::next(Mode::Scene, 37), mode::Next::Leave(37));
        // A closed tab is grey and does nothing.
        let mut m = SceneMode::open([1; SCENES], [0, 1, 1], 0);
        assert!(!m.enabled(0xf00) && m.enabled(0xf01));
        m.click(0xf00);
        assert_eq!(m.settle(), None);
    }

    #[test]
    fn back_quit_and_cancel() {
        let mut m = all();
        m.click(BACK);
        assert_eq!(m.settle(), Some(Click::Result(-1)));
        let mut m = all();
        assert_eq!(m.click(QUIT), Click::Bar(crate::ui::bar::QUIT));
        assert_eq!(m.take_sound().as_deref(), Some(mode::OK_SOUND));
        assert_eq!(m.settle(), None);
        let mut m = all();
        m.cancel();
        assert_eq!(m.take_sound().as_deref(), Some(mode::CANCEL_SOUND));
        assert_eq!(m.settle(), Some(Click::Result(-1)));
    }

    #[test]
    fn the_filmstrip_holds_then_fades_to_the_next() {
        // One picture never changes.
        assert_eq!(filmstrip(0, 1), (0, 0, 0));
        assert_eq!(filmstrip(5000, 1), (0, 0, 0));
        // The pointer has been on the card for 500 ms when the first
        // fade starts, and it lasts a second.
        assert_eq!(filmstrip(0, 3), (0, 0, 0));
        assert_eq!(filmstrip(500, 3), (0, 0, 0));
        assert_eq!(filmstrip(501, 3).0, 0);
        assert_eq!(filmstrip(1000, 3), (0, 1, 127));
        let (cur, next, blend) = filmstrip(1499, 3);
        assert_eq!((cur, next), (0, 1));
        assert!(blend >= 254);
        // Then the next picture holds a second and fades on.
        assert_eq!(filmstrip(1500, 3), (1, 1, 0));
        assert_eq!(filmstrip(2500, 3), (1, 1, 0));
        assert_eq!(filmstrip(3000, 3).0, 1);
        assert_eq!(filmstrip(3000, 3).1, 2);
        // After the last it goes round to the first.
        assert_eq!(filmstrip(5500, 3), (0, 0, 0));
        assert_eq!(filmstrip(3000, 2), (1, 0, 127));
    }

    #[test]
    fn slide_in_runs_on_the_clock() {
        let m = all();
        assert!(m.animating(0));
        assert!(m.animating(13 * 16));
        assert!(!m.animating(14 * 16));
        // A filmstrip of several pictures keeps the screen running.
        m.hover.set(Some((0, 0)));
        assert!(m.animating(5000));
        m.hover.set(Some((1, 0)));
        assert!(!m.animating(5000));
    }
}
