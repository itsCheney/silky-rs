//! The settings screen (system and sound pages) and the extras screens of
//! the title menu (gallery, scene replay, music room).
//!
//! Like the save screen these are drawn by the engine: a background, a
//! sprite sheet and an `.area` file each (`config00.area`,
//! `config01.area`, `cgmode.area`, `scmode.area`, `soundmode.area`). The
//! settings pages are long pictures (`system_cover`, `sound_cover`) that
//! scroll under the controls. Help lines come from `confighelp.map` and
//! `soundconfighelp.map`.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;
use silky_format::map::MapFile;

use crate::assets::{localized_name, Assets, Language};
use crate::game_tables::{entry_images, CG_ENTRIES, CG_IMAGES, MUSIC, SCENES};
use crate::render::Canvas;
use crate::screens::{area, area_file, draw_scroll_thumb, Cells, INK};
use crate::system::{RightClick, Settings, SystemData, VOICE_GROUPS};
use crate::text::TextRenderer;
use crate::ui::{Button, Ui};
use crate::{Extras, Result};

/// Ids shared by both settings pages.
pub mod ids {
    pub const UP: i32 = 3328;
    pub const DOWN: i32 = 3329;
    pub const SCROLL: i32 = 3330;
    pub const SYSTEM_TAB: i32 = 3584;
    pub const SOUND_TAB: i32 = 3585;
    pub const RESET: i32 = 3840;
    pub const BACK: i32 = 4096;
}

/// Ids of the extras screens.
pub mod extras_ids {
    /// Thumbnails or music rows: `ITEM + i`.
    pub const ITEM: i32 = 0;
    pub const PREVIOUS: i32 = 256;
    pub const NEXT: i32 = 257;
    /// Music room scroll bar.
    pub const SCROLL: i32 = 258;
    /// Gallery page numbers: `PAGE + n`.
    pub const PAGE: i32 = 512;
    pub const BACK: i32 = 768;
    /// Gallery, scenes, music: `MODE + n`.
    pub const MODE: i32 = 2560;
}

/// Controls that stay put while the page scrolls.
fn is_fixed(id: i32) -> bool {
    use ids::*;
    matches!(
        id,
        UP | DOWN | SCROLL | SYSTEM_TAB | SOUND_TAB | RESET | BACK
    )
}
/// System page toggle for auto mode itself (not a stored setting).
pub const AUTO_MODE: i32 = 1536;

// Settings sheets (`system_p`, `sound_p`).
const TAB_SYSTEM: Cells = Cells::new(0, 0, 186, 117, 117);
const TAB_SOUND: Cells = Cells::new(186, 0, 186, 117, 117);
const ARROW: Cells = Cells::new(373, 0, 52, 53, 54);
const RESET: Cells = Cells::new(427, 0, 71, 73, 74);
const BACK: Cells = Cells::new(565, 0, 88, 91, 91);
/// System page: on rows 0..3, off rows 3..6.
const SYS_TOGGLE: Cells = Cells::new(0, 468, 42, 35, 35);
const SYS_RADIO: Cells = Cells::new(42, 468, 42, 35, 35);
const SYS_KNOB: Cells = Cells::new(498, 309, 36, 42, 43);
const LANGUAGE: [Cells; 3] = [
    Cells::new(84, 468, 152, 35, 35),
    Cells::new(236, 468, 152, 35, 35),
    Cells::new(84, 573, 152, 35, 35),
];
/// Right click options: two columns, four state rows per option.
const RIGHT_CLICK: [u32; 2] = [653, 1131];
const RIGHT_CLICK_CELL: (u32, u32, u32) = (476, 43, 45);
/// Sound page: on rows 0..3, off rows 3..6.
const SND_TOGGLE_X: u32 = 1019;
const SND_TOGGLE_ROWS: [u32; 6] = [0, 46, 90, 134, 178, 222];
const SND_TOGGLE: (u32, u32) = (54, 42);
/// Volume bars: (x, width, fill row) for the big and the small ones.
const SND_BAR_BIG: (u32, u32, u32) = (659, 352, 39);
const SND_BAR_SMALL: (u32, u32, u32) = (668, 224, 151);
const SND_BAR_H: u32 = 29;

/// Where the settings page picture sits relative to the area file, the
/// part of the screen it shows, and the scroll bar track.
const SYSTEM_COVER_Y: i32 = 30;
const SOUND_COVER_Y: i32 = 10;
const VIEW: (i32, i32) = (25, 660);
pub const SETTINGS_TRACK: (i32, i32, i32, i32) = (1008, 115, 26, 488);
pub const SCROLL_STEP: i32 = 60;
/// Colour sample of the message window on the system page (cover pixels).
const WINDOW_SAMPLE: (i32, i32, u32, u32) = (31, 391, 468, 56);
const HELP_POS: (f32, f32) = (96.0, 694.0);
const HELP_SIZE: f32 = 17.0;

// Extras sheets (`cgmode_p`, `scenemode_p`, `musicmode_p`).
const MODE_BUTTON: [Cells; 3] = [
    Cells::new(0, 0, 180, 123, 123),
    Cells::new(180, 0, 180, 123, 123),
    Cells::new(360, 0, 180, 123, 123),
];
const EX_BACK_ROWS: [u32; 4] = [493, 586, 675, 766];
const EX_BACK: (u32, u32) = (88, 87);
/// Thumbnail frames: normal, hover, pressed, locked.
const FRAME: Cells = Cells::new(540, 0, 320, 180, 180);
const THUMB: (u32, u32) = (306, 166);
const THUMB_INSET: i32 = 7;
/// Previous rows 0..4, next rows 4..8.
const PAGE_ARROW: Cells = Cells::new(862, 0, 75, 75, 76);
const PAGE_DIGIT: (u32, u32, u32, u32) = (935, 0, 46, 49);
const PAGE_DIGIT_PITCH: u32 = 46;
pub const CG_PAGES: usize = 9;
/// Completion, in the box at the bottom right.
const COMPLETION_POS: (f32, f32) = (1110.0, 680.0);
/// Music room: rows of `musiclist`, 908 x 64, four state rows per track,
/// two tracks side by side.
const MUSIC_ROW: (u32, u32, u32) = (908, 62, 64);
const MUSIC_ROWS: usize = 8;
const MUSIC_ARROW: Cells = Cells::new(541, 0, 52, 53, 54);
const MUSIC_TRACK: (i32, i32, i32, i32) = (1209, 121, 26, 412);
const MUSIC_THUMB_COLOR: [u8; 4] = [196, 64, 108, 255];
const MUSIC_BAR: (u32, u32, u32, u32) = (668, 39, 452, 30);
const MUSIC_BAR_POS: (i32, i32) = (476, 653);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    System,
    Sound,
}

pub struct SettingsScreen {
    pub page: Page,
    pub scroll: i32,
    /// Slider held down with the pointer.
    pub dragging: Option<i32>,
}

/// An extras screen.
impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            page: Page::System,
            scroll: 0,
            dragging: None,
        }
    }
}

impl Default for SettingsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtrasScreen {
    pub fn new(kind: Extras) -> Self {
        Self {
            kind,
            page: 0,
            scroll: 0,
            view: None,
            playing: None,
        }
    }

    /// Gallery: the next picture of `entry` after `from` that was seen.
    pub fn next_picture(data: &SystemData, entry: usize, from: Option<usize>) -> Option<usize> {
        let start = from.map_or(0, |p| p + 1);
        entry_images(entry)
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, &i)| data.seen_all(CG_IMAGES[i]))
            .map(|(p, _)| p)
    }
}

pub struct ExtrasScreen {
    pub kind: Extras,
    /// Gallery page.
    pub page: usize,
    /// Music room: first row shown.
    pub scroll: usize,
    /// Gallery: entry being viewed and the picture of it.
    pub view: Option<(usize, usize)>,
    pub playing: Option<usize>,
}

pub struct Menus {
    language: Language,
    system: Vec<Area>,
    sound: Vec<Area>,
    gallery: Vec<Area>,
    scenes: Vec<Area>,
    music: Vec<Area>,
    system_help: Option<MapFile>,
    sound_help: Option<MapFile>,
}

fn map(assets: &Assets, name: &str) -> Option<MapFile> {
    assets.data(name).ok().and_then(|d| MapFile::parse(&d).ok())
}

fn find(areas: &[Area], x: i32, y: i32) -> Option<&Area> {
    areas.iter().find(|a| a.contains(x, y))
}

/// Value of a slider for a pointer at `x`.
pub fn slider_value(a: &Area, x: i32) -> f32 {
    ((x - a.left - 10) as f32 / (a.width() as f32 - 20.0)).clamp(0.0, 1.0)
}

impl Menus {
    pub fn load(assets: &Assets, language: Language) -> Self {
        Self {
            language,
            system: area_file(assets, "config00.area"),
            sound: area_file(assets, "config01.area"),
            gallery: area_file(assets, "cgmode.area"),
            scenes: area_file(assets, "scmode.area"),
            music: area_file(assets, "soundmode.area"),
            system_help: map(assets, "confighelp.map"),
            sound_help: map(assets, "soundconfighelp.map"),
        }
    }

    /// Whether the game has the extras screens this runtime knows (the
    /// tables in [`crate::game_tables`] are for one game).
    pub fn has_extras(&self) -> bool {
        !self.gallery.is_empty()
    }

    fn sheet(&self, assets: &mut Assets, name: &str) -> Result<Arc<Image>> {
        assets.image(&localized_name(name, self.language))
    }

    // Settings.

    fn settings_areas(&self, page: Page) -> &[Area] {
        match page {
            Page::System => &self.system,
            Page::Sound => &self.sound,
        }
    }

    fn cover_y(page: Page) -> i32 {
        match page {
            Page::System => SYSTEM_COVER_Y,
            Page::Sound => SOUND_COVER_Y,
        }
    }

    pub fn max_scroll(&self, assets: &mut Assets, page: Page) -> i32 {
        let name = match page {
            Page::System => "system_cover.akb",
            Page::Sound => "sound_cover.akb",
        };
        let h = self.sheet(assets, name).map_or(0, |i| i.height as i32);
        (h + Self::cover_y(page) - VIEW.1).max(0)
    }

    /// Area of a control on screen (page controls move with the scroll).
    fn on_screen(a: &Area, scroll: i32) -> Option<Area> {
        if is_fixed(a.id) {
            return Some(*a);
        }
        let (top, bottom) = (a.top - scroll, a.bottom - scroll);
        (top >= VIEW.0 && bottom <= VIEW.1).then_some(Area { top, bottom, ..*a })
    }

    pub fn hit_settings(&self, s: &SettingsScreen, x: i32, y: i32) -> Option<(Button, Area)> {
        self.settings_areas(s.page)
            .iter()
            .filter_map(|a| Self::on_screen(a, s.scroll))
            .find(|a| a.contains(x, y))
            .map(|a| (Button::Screen(a.id), a))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_settings(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        s: &SettingsScreen,
        data: &SystemData,
        max_scroll: i32,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        let st = &data.settings;
        let (bg, cover, sheet) = match s.page {
            Page::System => ("system_bg.akb", "system_cover.akb", "system_p.akb"),
            Page::Sound => ("sound_bg.akb", "sound_cover.akb", "sound_p.akb"),
        };
        canvas.draw(&*self.sheet(assets, bg)?, 0, 0, false);
        let cover = self.sheet(assets, cover)?;
        // The visible band of the page picture.
        let cy = Self::cover_y(s.page);
        let src_top = (VIEW.0 - cy + s.scroll).max(0) as u32;
        let dst_top = src_top as i32 + cy - s.scroll;
        let h = (VIEW.1 - dst_top).max(0) as u32;
        canvas.draw_part(&cover, (0, src_top, cover.width, h), 0, dst_top, false, 255);
        if s.page == Page::System {
            let (x, y, w, h) = WINDOW_SAMPLE;
            let y = y + cy - s.scroll;
            if y >= VIEW.0 && y + h as i32 <= VIEW.1 {
                let c = st.window_color;
                let rgba = [
                    (c[0] * 255.0) as u8,
                    (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8,
                    (st.window_opacity * 255.0) as u8,
                ];
                canvas.fill(x, y, w, h, rgba);
            }
        }
        let sheet = self.sheet(assets, sheet)?;
        for a in self.settings_areas(s.page) {
            let Some(a) = Self::on_screen(a, s.scroll) else {
                continue;
            };
            let b = Button::Screen(a.id);
            let st_row = |active: bool| ui.state(b, true, active) as u32;
            match a.id {
                ids::SYSTEM_TAB => {
                    TAB_SYSTEM.draw(canvas, &sheet, st_row(s.page == Page::System), &a)
                }
                ids::SOUND_TAB => TAB_SOUND.draw(canvas, &sheet, st_row(s.page == Page::Sound), &a),
                ids::UP => ARROW.draw(
                    canvas,
                    &sheet,
                    if s.scroll > 0 { st_row(false) } else { 3 },
                    &a,
                ),
                ids::DOWN => ARROW.draw(
                    canvas,
                    &sheet,
                    4 + if s.scroll < max_scroll {
                        st_row(false)
                    } else {
                        3
                    },
                    &a,
                ),
                ids::RESET => RESET.draw(canvas, &sheet, st_row(false), &a),
                ids::BACK => BACK.draw(canvas, &sheet, st_row(false), &a),
                ids::SCROLL => {}
                id => match s.page {
                    Page::System => self.draw_system_control(canvas, &sheet, ui, id, &a, st),
                    Page::Sound => self.draw_sound_control(canvas, &sheet, ui, id, &a, st),
                },
            }
        }
        draw_scroll_thumb(
            canvas,
            SETTINGS_TRACK,
            s.scroll.max(0) as usize,
            (max_scroll + 100) as usize,
            100,
        );
        if let (true, Some(tr), Some(Button::Screen(id))) = (st.tooltips, text, ui.hover) {
            if let Some(help) = self.settings_help(s.page, id) {
                tr.draw_plain(canvas, help, HELP_POS.0, HELP_POS.1, HELP_SIZE, INK);
            }
        }
        Ok(())
    }

    fn draw_toggle(canvas: &mut Canvas, sheet: &Image, ui: &Ui, a: &Area, on: bool) {
        let st = (ui.state(Button::Screen(a.id), true, false) as u32).min(2);
        SYS_TOGGLE.draw(canvas, sheet, if on { st } else { 3 + st }, a);
    }

    fn draw_knob(canvas: &mut Canvas, sheet: &Image, ui: &Ui, a: &Area, v: f32) {
        let st = (ui.state(Button::Screen(a.id), true, false) as u32).min(2);
        let x = a.left + 10 + (v.clamp(0.0, 1.0) * (a.width() as f32 - 20.0)) as i32;
        let y = a.top + (a.height() as i32 - SYS_KNOB.h as i32) / 2;
        canvas.draw_part(
            sheet,
            SYS_KNOB.part(st),
            x - SYS_KNOB.w as i32 / 2,
            y,
            false,
            255,
        );
    }

    fn draw_system_control(
        &self,
        canvas: &mut Canvas,
        sheet: &Image,
        ui: &Ui,
        id: i32,
        a: &Area,
        st: &Settings,
    ) {
        if id == AUTO_MODE {
            Self::draw_toggle(canvas, sheet, ui, a, ui.auto);
        } else if let Some(on) = system_toggle(st, id) {
            Self::draw_toggle(canvas, sheet, ui, a, on);
        } else if let Some(v) = system_slider(st, id) {
            Self::draw_knob(canvas, sheet, ui, a, v);
        } else if (2560..=2562).contains(&id) {
            let hover = ui.state(Button::Screen(id), true, false) as u32;
            if st.effect_speed as i32 == id - 2560 {
                SYS_RADIO.draw(canvas, sheet, 2, a);
            } else if hover > 0 {
                SYS_RADIO.draw(canvas, sheet, 0, a);
            }
        } else if (3072..=3077).contains(&id) {
            let k = (id - 3072) as u32;
            let (w, h, pitch) = RIGHT_CLICK_CELL;
            let active = right_click_option(k as usize) == st.right_click;
            let row = (k / 2) * 4 + ui.state(Button::Screen(id), true, active) as u32;
            canvas.draw_part(
                sheet,
                (RIGHT_CLICK[(k % 2) as usize], pitch * row, w, h),
                a.left + (a.width() as i32 - w as i32) / 2,
                a.top + (a.height() as i32 - h as i32) / 2,
                false,
                255,
            );
        } else if (4352..=4354).contains(&id) {
            let lang = language_option((id - 4352) as usize);
            let active = st.language.unwrap_or(self.language) == lang;
            let row = (ui.state(Button::Screen(id), true, active) as u32).min(2);
            LANGUAGE[(id - 4352) as usize].draw(canvas, sheet, row, a);
        }
    }

    fn draw_sound_control(
        &self,
        canvas: &mut Canvas,
        sheet: &Image,
        ui: &Ui,
        id: i32,
        a: &Area,
        st: &Settings,
    ) {
        if let Some(on) = sound_toggle(st, id) {
            let s = (ui.state(Button::Screen(id), true, false) as usize).min(2);
            let row = SND_TOGGLE_ROWS[if on { s } else { 3 + s }];
            let (w, h) = SND_TOGGLE;
            canvas.draw_part(
                sheet,
                (SND_TOGGLE_X, row, w, h),
                a.left + (a.width() as i32 - w as i32) / 2,
                a.top + (a.height() as i32 - h as i32) / 2,
                false,
                255,
            );
        } else if let Some(v) = sound_slider(st, id) {
            let (x, w, fill) = if id >= 1280 {
                SND_BAR_SMALL
            } else {
                SND_BAR_BIG
            };
            let left = a.left + (a.width() as i32 - w as i32) / 2;
            let top = a.top + (a.height() as i32 - SND_BAR_H as i32) / 2;
            let filled = (v.clamp(0.0, 1.0) * w as f32) as u32;
            canvas.draw_part(sheet, (x, fill, filled, SND_BAR_H), left, top, false, 255);
        }
    }

    fn settings_help(&self, page: Page, id: i32) -> Option<&str> {
        let (help, index) = match page {
            Page::System => (
                &self.system_help,
                match id {
                    0 => 0,
                    1 => 1,
                    256 => 2,
                    1024 => 3,
                    1025 => 4,
                    1536 => 5,
                    1537 => 6,
                    1792 => 7,
                    1280 => 8,
                    512 => 9,
                    768 => 10,
                    2304 => 11,
                    2048..=2051 => 12,
                    2560..=2562 => 13,
                    2816..=2824 => 14,
                    3072..=3077 => 15,
                    ids::UP => 16,
                    ids::DOWN => 17,
                    ids::SCROLL => 18,
                    ids::SYSTEM_TAB => 19,
                    ids::SOUND_TAB => 20,
                    ids::RESET => 21,
                    ids::BACK => 22,
                    _ => return None,
                },
            ),
            Page::Sound => (
                &self.sound_help,
                match id {
                    0 => 0,
                    1 => 1,
                    256 => 2,
                    257 => 3,
                    512 => 4,
                    513 => 5,
                    768 => 6,
                    769 => 7,
                    1024..=1033 => 8 + (id - 1024) as usize,
                    1280..=1289 => 18 + (id - 1280) as usize,
                    1792 => 28,
                    ids::UP => 29,
                    ids::DOWN => 30,
                    ids::SCROLL => 31,
                    ids::SYSTEM_TAB => 32,
                    ids::SOUND_TAB => 33,
                    ids::RESET => 34,
                    ids::BACK => 35,
                    _ => return None,
                },
            ),
        };
        help.as_ref()?.line(self.language.map_section(), index)
    }

    /// A control of the settings screen where it is on screen now.
    pub fn settings_area(&self, s: &SettingsScreen, id: i32) -> Option<Area> {
        self.settings_areas(s.page)
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| Self::on_screen(a, s.scroll))
    }

    pub fn settings_track(&self, x: i32, y: i32) -> Option<f32> {
        let (tx, ty, tw, th) = SETTINGS_TRACK;
        (x >= tx - 20 && x < tx + tw + 20 && y >= ty && y < ty + th)
            .then(|| (y - ty) as f32 / th as f32)
    }

    // Extras.

    pub fn areas(&self, kind: Extras) -> &[Area] {
        self.extras_areas(kind)
    }

    fn extras_areas(&self, kind: Extras) -> &[Area] {
        match kind {
            Extras::Gallery => &self.gallery,
            Extras::Scenes => &self.scenes,
            Extras::Music => &self.music,
        }
    }

    /// Whether a gallery entry has any picture seen.
    pub fn entry_unlocked(data: &SystemData, entry: usize) -> bool {
        entry_images(entry)
            .into_iter()
            .any(|i| data.seen_all(CG_IMAGES[i]))
    }

    pub fn scene_unlocked(data: &SystemData, scene: usize) -> bool {
        SCENES.get(scene).is_some_and(|s| data.seen_all(&[s]))
    }

    pub fn track_unlocked(data: &SystemData, track: usize) -> bool {
        MUSIC
            .get(track)
            .is_some_and(|m| data.music.contains(&m.to_ascii_lowercase()))
    }

    /// Item (entry, scene, track) behind a thumbnail or row.
    pub fn item(s: &ExtrasScreen, i: usize) -> Option<usize> {
        let (k, n) = match s.kind {
            Extras::Gallery => (s.page * 9 + i, CG_ENTRIES.len()),
            Extras::Scenes => (i, SCENES.len()),
            Extras::Music => (s.scroll * 2 + i, MUSIC.len()),
        };
        (k < n).then_some(k)
    }

    pub fn unlocked(s: &ExtrasScreen, data: &SystemData, item: usize) -> bool {
        match s.kind {
            Extras::Gallery => Self::entry_unlocked(data, item),
            Extras::Scenes => Self::scene_unlocked(data, item),
            Extras::Music => Self::track_unlocked(data, item),
        }
    }

    pub fn music_max_scroll() -> usize {
        MUSIC.len().div_ceil(2).saturating_sub(MUSIC_ROWS / 2)
    }

    pub fn hit_extras(
        &self,
        s: &ExtrasScreen,
        data: &SystemData,
        x: i32,
        y: i32,
    ) -> Option<Button> {
        if s.view.is_some() {
            return None;
        }
        let a = find(self.extras_areas(s.kind), x, y)?;
        let id = a.id;
        if (extras_ids::ITEM..extras_ids::PREVIOUS).contains(&id) {
            let item = Self::item(s, (id - extras_ids::ITEM) as usize)?;
            if !Self::unlocked(s, data, item) {
                return None;
            }
        }
        Some(Button::Screen(id))
    }

    pub fn music_track_fraction(&self, x: i32, y: i32) -> Option<f32> {
        let a = area(&self.music, extras_ids::SCROLL).filter(|a| a.contains(x, y))?;
        let _ = a;
        let (_, ty, _, th) = MUSIC_TRACK;
        Some(((y - ty) as f32 / th as f32).clamp(0.0, 1.0))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_extras(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        ui: &Ui,
        s: &ExtrasScreen,
        data: &SystemData,
        music_pos: Option<(f64, f64)>,
        text: Option<&TextRenderer>,
    ) -> Result<()> {
        if let Some((entry, pos)) = s.view {
            return self.draw_picture(canvas, assets, entry, pos);
        }
        let (bg, sheet) = match s.kind {
            Extras::Gallery => ("cgmode_bg.akb", "cgmode_p.akb"),
            Extras::Scenes => ("scenemode_bg.akb", "scenemode_p.akb"),
            Extras::Music => ("musicmode_bg.akb", "musicmode_p.akb"),
        };
        canvas.draw(&*self.sheet(assets, bg)?, 0, 0, false);
        let sheet = self.sheet(assets, sheet)?;
        let state = |id: i32, enabled: bool, active: bool| {
            ui.state(Button::Screen(id), enabled, active) as u32
        };
        let thumbs = match s.kind {
            Extras::Gallery => Some(assets.image(&format!("cgmode_thm{:02}.akb", s.page))?),
            Extras::Scenes => Some(assets.image("scene_thumb.akb")?),
            Extras::Music => None,
        };
        let list = match s.kind {
            Extras::Music => Some(self.sheet(assets, "musiclist.akb")?),
            _ => None,
        };
        for a in self.extras_areas(s.kind) {
            let id = a.id;
            match id {
                _ if (extras_ids::MODE..extras_ids::MODE + 3).contains(&id) => {
                    let k = (id - extras_ids::MODE) as usize;
                    let active = k == s.kind as usize;
                    MODE_BUTTON[k].draw(canvas, &sheet, state(id, true, active), a);
                }
                extras_ids::BACK => {
                    let (w, h) = EX_BACK;
                    let row = EX_BACK_ROWS[state(id, true, false) as usize];
                    canvas.draw_part(
                        &sheet,
                        (0, row, w, h),
                        a.left + (a.width() as i32 - w as i32) / 2,
                        a.top + (a.height() as i32 - h as i32) / 2,
                        false,
                        255,
                    );
                }
                extras_ids::PREVIOUS | extras_ids::NEXT if s.kind == Extras::Music => {
                    let enabled = if id == extras_ids::PREVIOUS {
                        s.scroll > 0
                    } else {
                        s.scroll < Self::music_max_scroll()
                    };
                    let base = if id == extras_ids::PREVIOUS { 0 } else { 4 };
                    MUSIC_ARROW.draw(canvas, &sheet, base + state(id, enabled, false), a);
                }
                extras_ids::PREVIOUS | extras_ids::NEXT => {
                    let enabled = if id == extras_ids::PREVIOUS {
                        s.page > 0
                    } else {
                        s.page + 1 < CG_PAGES
                    };
                    let base = if id == extras_ids::PREVIOUS { 0 } else { 4 };
                    PAGE_ARROW.draw(canvas, &sheet, base + state(id, enabled, false), a);
                }
                _ if (extras_ids::PAGE..extras_ids::PAGE + CG_PAGES as i32).contains(&id) => {
                    let n = (id - extras_ids::PAGE) as u32;
                    let (x, y, w, h) = PAGE_DIGIT;
                    let row = state(id, true, n as usize == s.page);
                    canvas.draw_part(
                        &sheet,
                        (x + PAGE_DIGIT_PITCH * (n + 1), y + h * row, w, h),
                        a.left + (a.width() as i32 - w as i32) / 2,
                        a.top + (a.height() as i32 - h as i32) / 2,
                        false,
                        255,
                    );
                }
                _ if (extras_ids::ITEM..extras_ids::PREVIOUS).contains(&id) => {
                    let i = (id - extras_ids::ITEM) as usize;
                    let Some(item) = Self::item(s, i) else {
                        continue;
                    };
                    let open = Self::unlocked(s, data, item);
                    if let Some(list) = &list {
                        let (w, h, pitch) = MUSIC_ROW;
                        let row = if !open {
                            3
                        } else if s.playing == Some(item) {
                            2
                        } else {
                            state(id, true, false).min(2)
                        };
                        let y = pitch * ((item as u32 / 2) * 4 + row);
                        canvas.draw_part(
                            list,
                            ((item as u32 % 2) * w, y, w, h),
                            a.left,
                            a.top,
                            false,
                            255,
                        );
                        continue;
                    }
                    if let (true, Some(th)) = (open, &thumbs) {
                        let (w, h) = THUMB;
                        let cell = (i as u32 % 3 * w, i as u32 / 3 * h, w, h);
                        canvas.draw_part(
                            th,
                            cell,
                            a.left + THUMB_INSET,
                            a.top + THUMB_INSET,
                            false,
                            255,
                        );
                    }
                    let row = if open {
                        state(id, true, false).min(2)
                    } else {
                        3
                    };
                    FRAME.draw(canvas, &sheet, row, a);
                }
                _ => {}
            }
        }
        match s.kind {
            Extras::Gallery => {
                if let Some(tr) = text {
                    let total = CG_IMAGES.len();
                    let seen = CG_IMAGES.iter().filter(|l| data.seen_all(l)).count();
                    let pct = format!("{}%", seen * 100 / total);
                    tr.draw_plain(canvas, &pct, COMPLETION_POS.0, COMPLETION_POS.1, 28.0, INK);
                }
            }
            Extras::Music => {
                draw_scroll_thumb_color(
                    canvas,
                    MUSIC_TRACK,
                    s.scroll,
                    Self::music_max_scroll() + 4,
                    4,
                );
                if let Some((pos, len)) = music_pos.filter(|p| p.1 > 0.0) {
                    let (x, y, w, h) = MUSIC_BAR;
                    let filled = ((pos / len).clamp(0.0, 1.0) * w as f64) as u32;
                    canvas.draw_part(
                        &sheet,
                        (x, y, filled, h),
                        MUSIC_BAR_POS.0,
                        MUSIC_BAR_POS.1,
                        false,
                        255,
                    );
                }
            }
            Extras::Scenes => {}
        }
        Ok(())
    }

    /// A gallery picture over the whole screen.
    fn draw_picture(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        entry: usize,
        pos: usize,
    ) -> Result<()> {
        let Some(&index) = entry_images(entry).get(pos) else {
            return Ok(());
        };
        for (k, layer) in CG_IMAGES[index].iter().enumerate() {
            let img = assets.image(&format!("{layer}.akb"))?;
            // Difference pictures use pure green as transparency.
            canvas.draw(&img, 0, 0, k > 0);
        }
        Ok(())
    }
}

fn draw_scroll_thumb_color(
    canvas: &mut Canvas,
    track: (i32, i32, i32, i32),
    pos: usize,
    len: usize,
    shown: usize,
) {
    let (x, y, w, h) = track;
    let size = (h as usize * shown / len.max(1)).max(40) as i32;
    let max = len.saturating_sub(shown).max(1);
    let top = y + ((h - size) as usize * pos.min(max) / max) as i32;
    canvas.fill(x, top, w as u32, size as u32, MUSIC_THUMB_COLOR);
}

/// The system page's toggles.
pub fn system_toggle(st: &Settings, id: i32) -> Option<bool> {
    Some(match id {
        0 => st.typewriter,
        256 => st.punctuation_pause,
        512 => st.auto_save_at_choices,
        768 => st.tooltips,
        1024 => st.skip_read_only,
        1280 => st.keep_skip_after_choices,
        AUTO_MODE => return None,
        1792 => st.keep_auto_after_choices,
        2304 => st.fixed_window,
        2816..=2824 => st.confirm[confirm_question(id)],
        _ => return None,
    })
}

/// Dialog question asked for a confirmation toggle.
pub fn confirm_question(id: i32) -> usize {
    use crate::screens::question::*;
    match id {
        2816 => TITLE,
        2817 => QUIT,
        2818 => SCENE_SELECT,
        2819 => JUMP,
        2820 => SAVE,
        2821 => LOAD,
        2822 => OVERWRITE,
        2823 => DELETE,
        _ => DELETE_ALL,
    }
}

pub fn system_slider(st: &Settings, id: i32) -> Option<f32> {
    Some(match id {
        1 => st.text_speed,
        1025 => st.skip_speed,
        1537 => st.auto_speed,
        2048..=2050 => st.window_color[(id - 2048) as usize],
        2051 => st.window_opacity,
        _ => return None,
    })
}

pub fn sound_toggle(st: &Settings, id: i32) -> Option<bool> {
    Some(match id {
        0 => st.master_on,
        256 => st.music_on,
        512 => st.effects_on,
        768 => st.voice_on,
        1024..=1033 => st.voice_group_on[(id - 1024) as usize],
        1792 => st.cut_voice,
        _ => return None,
    })
}

pub fn sound_slider(st: &Settings, id: i32) -> Option<f32> {
    Some(match id {
        1 => st.master,
        257 => st.music,
        513 => st.effects,
        769 => st.voice,
        1280..=1289 => st.voice_group[(id - 1280) as usize],
        _ => return None,
    })
}

/// Changes a setting from a click on control `id`; `value` is the slider
/// position. Returns false if `id` is not a setting.
pub fn apply(st: &mut Settings, page: Page, id: i32, value: f32) -> bool {
    match page {
        Page::System => match id {
            0 => st.typewriter ^= true,
            256 => st.punctuation_pause ^= true,
            512 => st.auto_save_at_choices ^= true,
            768 => st.tooltips ^= true,
            1024 => st.skip_read_only ^= true,
            1280 => st.keep_skip_after_choices ^= true,
            1792 => st.keep_auto_after_choices ^= true,
            2304 => st.fixed_window ^= true,
            2816..=2824 => st.confirm[confirm_question(id)] ^= true,
            1 => st.text_speed = value,
            1025 => st.skip_speed = value,
            1537 => st.auto_speed = value,
            2048..=2050 => st.window_color[(id - 2048) as usize] = value,
            2051 => st.window_opacity = value,
            2560..=2562 => st.effect_speed = (id - 2560) as u8,
            3072..=3077 => st.right_click = right_click_option((id - 3072) as usize),
            4352..=4354 => st.language = Some(language_option((id - 4352) as usize)),
            _ => return false,
        },
        Page::Sound => match id {
            0 => st.master_on ^= true,
            256 => st.music_on ^= true,
            512 => st.effects_on ^= true,
            768 => st.voice_on ^= true,
            1024..=1033 => st.voice_group_on[(id - 1024) as usize] ^= true,
            1792 => st.cut_voice ^= true,
            1 => st.master = value,
            257 => st.music = value,
            513 => st.effects = value,
            769 => st.voice = value,
            1280..=1289 => st.voice_group[(id - 1280) as usize] = value,
            _ => return false,
        },
    }
    true
}

/// Whether control `id` is a slider (it follows a held pointer).
pub fn is_slider(page: Page, id: i32) -> bool {
    match page {
        Page::System => matches!(id, 1 | 1025 | 1537 | 2048..=2051),
        Page::Sound => matches!(id, 1 | 257 | 513 | 769 | 1280..=1289),
    }
}

/// Puts back the defaults of one page.
pub fn reset(st: &mut Settings, page: Page) {
    let d = Settings::default();
    match page {
        Page::System => {
            let sound = (
                st.master_on,
                st.master,
                st.music_on,
                st.music,
                st.effects_on,
                st.effects,
                st.voice_on,
                st.voice,
                st.voice_group_on,
                st.voice_group,
                st.cut_voice,
            );
            let language = st.language;
            *st = d;
            st.language = language;
            (
                st.master_on,
                st.master,
                st.music_on,
                st.music,
                st.effects_on,
                st.effects,
                st.voice_on,
                st.voice,
                st.voice_group_on,
                st.voice_group,
                st.cut_voice,
            ) = sound;
        }
        Page::Sound => {
            st.master_on = d.master_on;
            st.master = d.master;
            st.music_on = d.music_on;
            st.music = d.music;
            st.effects_on = d.effects_on;
            st.effects = d.effects;
            st.voice_on = d.voice_on;
            st.voice = d.voice;
            st.voice_group_on = [true; VOICE_GROUPS];
            st.voice_group = [1.0; VOICE_GROUPS];
            st.cut_voice = d.cut_voice;
        }
    }
}

fn right_click_option(k: usize) -> RightClick {
    match k {
        0 => RightClick::HideWindow,
        1 => RightClick::Settings,
        2 => RightClick::Skip,
        3 => RightClick::Auto,
        4 => RightClick::Load,
        _ => RightClick::Save,
    }
}

fn language_option(k: usize) -> Language {
    match k {
        0 => Language::English,
        1 => Language::SimplifiedChinese,
        _ => Language::TraditionalChinese,
    }
}
