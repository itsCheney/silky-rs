//! The settings screens of 同級生 Remake: two pages of system settings
//! (`system2_bg_<lang>.akb` with `config00.area`, then `system1_bg_<lang>.akb`
//! with `config01.area`).
//!
//! Read from the executable and compared under Wine. The parts come from
//! `system_p_<lang>.akb`: ON and OFF switches (86 x 50, rows ON normal,
//! hover, pressed, then OFF the same), slider knobs (36 x 60, rows normal,
//! hover, pressed), the game mode buttons, the language buttons, the
//! right-click choices and the page tabs; the side column, the bottom strip
//! and the hint bar are the system menu's ([`super::system`]). A slider's
//! knob runs from the left end of its area to 36 pixels short of the right
//! end; the speed sliders stop at eleven places, the three-place choices
//! (screen and map speed) put a knob on the chosen place. The window
//! colour box shows the game screen behind the menu through the message
//! window's tint, with a sample line on it.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;

use super::sound;
use super::system::{side, System};
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::screens::question;
use crate::system::{RightClick, Settings};
use crate::Result;

/// Ids of `config00.area` (page 1) and `config01.area` (page 2), and the
/// side column's from [`SIDE`].
pub mod ids {
    /// Classic and easy mode.
    pub const MODE: i32 = 0;
    /// Screen effect speed: normal, fast, off.
    pub const SCREEN_SPEED: i32 = 256;
    pub const TYPEWRITER: i32 = 512;
    pub const TEXT_SPEED: i32 = 513;
    pub const PUNCTUATION: i32 = 768;
    /// Skip mode on or off, and its speed.
    pub const SKIP: i32 = 1024;
    pub const SKIP_SPEED: i32 = 1025;
    /// Auto mode on or off, and its speed.
    pub const AUTO: i32 = 1280;
    pub const AUTO_SPEED: i32 = 1281;
    /// Walking speed on the maps: slow, normal, fast.
    pub const MAP_SPEED: i32 = 1536;
    pub const KEEP_AUTO: i32 = 1792;
    pub const KEEP_SKIP: i32 = 2048;
    pub const AUTO_SAVE: i32 = 2304;
    pub const FIXED_WINDOW: i32 = 2560;
    pub const TOOLTIPS: i32 = 2816;
    /// Window red, green, blue, then transparency.
    pub const COLOR: i32 = 3072;
    pub const ALPHA: i32 = 3075;
    pub const PREVIOUS: i32 = 3328;
    pub const NEXT: i32 = 3329;
    pub const TAB: i32 = 3584;
    /// Page 2: the ten confirmations and the six right-click choices.
    pub const CONFIRM: i32 = 4096;
    pub const RIGHT_CLICK: i32 = 4352;
    pub const RESET: i32 = 4608;
    pub const LANGUAGE: i32 = 5120;
    /// The side column.
    pub const SIDE: i32 = 0x10000;
}

/// A page of the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    System1,
    System2,
    /// The sound page ([`super::sound`]).
    Sound,
}

/// The settings screen's state.
pub struct ConfigScreen {
    pub page: Page,
    /// Slider held down with the pointer.
    pub dragging: Option<i32>,
    /// Clock when the screen opened (the sample line types out from it).
    pub opened: u64,
    /// The text, skip and auto speed sliders are up (their switches are
    /// on); the knob is left out and the slider does nothing otherwise.
    pub speeds: [bool; 3],
    /// The sound page's page of characters.
    pub voices: usize,
}

impl ConfigScreen {
    pub fn new(page: Page, opened: u64) -> Self {
        Self {
            page,
            dragging: None,
            opened,
            speeds: [true; 3],
            voices: 0,
        }
    }
}

/// The parts of `system_p_<lang>.akb`.
const SWITCH: (u32, u32, u32, u32) = (508, 768, 86, 50);
const KNOB: (u32, u32, u32, u32) = (594, 768, 36, 60);
/// The knob of a three-place choice (the pressed look).
const CHOSEN_KNOB: u32 = 2;
/// Classic and easy mode: 413 x 80, rows normal, hover, current; placed
/// as measured under Wine.
const MODE_PART: [(u32, u32); 2] = [(630, 768), (1057, 768)];
const MODE_SIZE: (u32, u32) = (413, 80);
const MODE_POS: [(i32, i32); 2] = [(660, 45), (1087, 45)];
/// Language buttons: 146 x 64 from x 1452, rows normal, hover, current.
const LANGUAGE_PART: (u32, u32, u32) = (1452, 146, 64);
/// Right-click choices: 726 x 64, two columns, rows normal, hover,
/// current, gray; a pair of choices every 256 rows.
const RIGHT_PART: (u32, u32, u32) = (726, 64, 256);
/// The page tabs: previous, SYSTEM 1, SYSTEM 2, next; rows of 56 from
/// 768 (normal, hover, current, gray).
const TABS_PART_X: [u32; 4] = [0, 56, 254, 452];
const TABS_PART_W: [u32; 4] = [56, 198, 198, 56];
const TABS_PART_Y: u32 = 768;
const TABS_H: u32 = 56;
/// The reset button of `menu_p.akb`: 128 x 78, rows by state.
const RESET_PART: (u32, u32, u32, u32) = (1788, 0, 128, 78);
/// The window colour box and its sample line.
const SAMPLE: (i32, i32, u32, u32) = (298, 748, 792, 90);
const SAMPLE_TEXT: (i32, i32) = (498, 773);

/// Places of a speed slider.
const STEPS: f32 = 10.0;

fn knob_x(a: &Area, v: f32) -> i32 {
    a.left + (v.clamp(0.0, 1.0) * (a.width() as f32 - KNOB.2 as f32)).round() as i32
}

/// The value a slider takes for pointer x.
pub fn slider_value(a: &Area, id: i32, x: i32) -> f32 {
    let span = (a.width() as f32 - KNOB.2 as f32).max(1.0);
    let v = ((x - a.left) as f32 - KNOB.2 as f32 / 2.0) / span;
    let v = v.clamp(0.0, 1.0);
    if is_speed(id) {
        (v * STEPS).round() / STEPS
    } else {
        v
    }
}

/// The speed sliders in order: text, skip, auto.
fn speed_index(id: i32) -> usize {
    match id {
        ids::TEXT_SPEED => 0,
        ids::SKIP_SPEED => 1,
        _ => 2,
    }
}

fn is_speed(id: i32) -> bool {
    matches!(id, ids::TEXT_SPEED | ids::SKIP_SPEED | ids::AUTO_SPEED)
}

/// Whether control `id` is a slider (it follows a held pointer).
pub fn is_slider(page: Page, id: i32) -> bool {
    match page {
        Page::System1 => is_speed(id) || (ids::COLOR..=ids::ALPHA).contains(&id),
        Page::System2 => false,
        Page::Sound => sound::is_slider(id),
    }
}

fn slider(st: &Settings, id: i32) -> Option<f32> {
    Some(match id {
        ids::TEXT_SPEED => st.text_speed,
        ids::SKIP_SPEED => st.skip_speed,
        ids::AUTO_SPEED => st.auto_speed,
        ids::ALPHA => 1.0 - st.window_opacity,
        _ if (ids::COLOR..ids::ALPHA).contains(&id) => st.window_color[(id - ids::COLOR) as usize],
        _ => return None,
    })
}

/// The confirmations of page 2 in area order, as dialog questions; `None`
/// is the jump to events.
const CONFIRMS: [Option<usize>; 10] = [
    Some(question::SAVE),
    Some(question::OVERWRITE),
    Some(question::LOAD),
    Some(question::DELETE_ALL),
    Some(question::QUIT),
    Some(question::DELETE),
    Some(question::TITLE),
    Some(question::SCENE_SELECT),
    Some(question::JUMP),
    None,
];

/// The right-click choices in area order.
const RIGHT_CLICKS: [RightClick; 6] = [
    RightClick::HideWindow,
    RightClick::Auto,
    RightClick::Settings,
    RightClick::Skip,
    RightClick::Load,
    RightClick::Save,
];

const LANGUAGES: [Language; 3] = [
    Language::English,
    Language::SimplifiedChinese,
    Language::TraditionalChinese,
];

/// A switch's state (skip and auto mode are the game's, not settings).
fn switch(st: &Settings, id: i32, skip: bool, auto: bool) -> Option<bool> {
    Some(match id {
        ids::TYPEWRITER => st.typewriter,
        ids::PUNCTUATION => st.punctuation_pause,
        ids::SKIP => skip,
        ids::AUTO => auto,
        ids::KEEP_AUTO => st.keep_auto_after_choices,
        ids::KEEP_SKIP => st.keep_skip_after_choices,
        ids::AUTO_SAVE => st.auto_save_at_choices,
        ids::FIXED_WINDOW => st.fixed_window,
        ids::TOOLTIPS => st.tooltips,
        _ if (ids::CONFIRM..ids::CONFIRM + 10).contains(&id) => {
            match CONFIRMS[(id - ids::CONFIRM) as usize] {
                Some(q) => st.confirm[q],
                None => st.confirm_event,
            }
        }
        _ => return None,
    })
}

/// Changes a setting from a click on control `id`; `value` is the slider
/// position. Returns false if `id` is not a setting.
pub fn apply(st: &mut Settings, id: i32, value: f32) -> bool {
    match id {
        ids::MODE | 1 => st.classic_mode = id == ids::MODE,
        _ if (ids::SCREEN_SPEED..ids::SCREEN_SPEED + 3).contains(&id) => {
            st.effect_speed = (id - ids::SCREEN_SPEED) as u8
        }
        _ if (ids::MAP_SPEED..ids::MAP_SPEED + 3).contains(&id) => {
            st.map_speed = (id - ids::MAP_SPEED) as u8
        }
        ids::TYPEWRITER => st.typewriter ^= true,
        ids::PUNCTUATION => st.punctuation_pause ^= true,
        ids::KEEP_AUTO => st.keep_auto_after_choices ^= true,
        ids::KEEP_SKIP => st.keep_skip_after_choices ^= true,
        ids::AUTO_SAVE => st.auto_save_at_choices ^= true,
        ids::FIXED_WINDOW => st.fixed_window ^= true,
        ids::TOOLTIPS => st.tooltips ^= true,
        ids::TEXT_SPEED => st.text_speed = value,
        ids::SKIP_SPEED => st.skip_speed = value,
        ids::AUTO_SPEED => st.auto_speed = value,
        ids::ALPHA => st.window_opacity = 1.0 - value,
        _ if (ids::COLOR..ids::ALPHA).contains(&id) => {
            st.window_color[(id - ids::COLOR) as usize] = value
        }
        _ if (ids::CONFIRM..ids::CONFIRM + 10).contains(&id) => {
            match CONFIRMS[(id - ids::CONFIRM) as usize] {
                Some(q) => st.confirm[q] ^= true,
                None => st.confirm_event ^= true,
            }
        }
        _ if (ids::RIGHT_CLICK..ids::RIGHT_CLICK + 6).contains(&id) => {
            st.right_click = RIGHT_CLICKS[(id - ids::RIGHT_CLICK) as usize]
        }
        _ if (ids::LANGUAGE..ids::LANGUAGE + 3).contains(&id) => {
            st.language = Some(LANGUAGES[(id - ids::LANGUAGE) as usize])
        }
        _ => return false,
    }
    true
}

/// Puts back the defaults of the system pages (sound and language stay).
pub fn reset(st: &mut Settings, defaults: &Settings) {
    let keep = st.clone();
    *st = defaults.clone();
    st.language = keep.language;
    st.master_on = keep.master_on;
    st.master = keep.master;
    st.music_on = keep.music_on;
    st.music = keep.music;
    st.effects_on = keep.effects_on;
    st.effects = keep.effects;
    st.voice_on = keep.voice_on;
    st.voice = keep.voice;
    st.voice_group_on = keep.voice_group_on;
    st.voice_group = keep.voice_group;
    st.cut_voice = keep.cut_voice;
}

impl System {
    fn config_areas(&self, page: Page) -> &[Area] {
        match page {
            Page::System1 => &self.config[0],
            Page::System2 => &self.config[1],
            Page::Sound => &self.config[2],
        }
    }

    pub fn config_area(&self, page: Page, id: i32) -> Option<&Area> {
        self.config_areas(page).iter().find(|a| a.id == id)
    }

    /// Whether an element of the settings does something now.
    pub fn config_enabled(
        s: &ConfigScreen,
        st: &Settings,
        id: i32,
        can_save: bool,
        has_log: bool,
    ) -> bool {
        if s.page == Page::Sound && id < ids::SIDE {
            return sound::enabled(st, s.voices, id);
        }
        match id {
            ids::TEXT_SPEED | ids::SKIP_SPEED | ids::AUTO_SPEED => s.speeds[speed_index(id)],
            ids::PREVIOUS => s.page == Page::System2,
            ids::NEXT => s.page == Page::System1,
            _ if id >= ids::SIDE => match (id - ids::SIDE) as usize {
                side::SAVE => can_save,
                side::LOG => has_log,
                _ => true,
            },
            _ => true,
        }
    }

    /// The element of the settings under (x, y).
    pub fn hit_config(&self, s: &ConfigScreen, x: i32, y: i32) -> Option<i32> {
        if let Some(id) = self.hit_side(x, y) {
            return Some(id - super::system::ids::SIDE + ids::SIDE);
        }
        let a = self
            .config_areas(s.page)
            .iter()
            .find(|a| a.contains(x, y))?;
        Some(a.id)
    }

    /// The hint for an element of the settings (`SYSTEM_<lang>.MAP`).
    fn config_hint(&self, s: &ConfigScreen, id: i32) -> Option<String> {
        use ids::*;
        let n = match id {
            _ if id >= SIDE => 79 + (id - SIDE) as u32,
            _ if s.page == Page::Sound => sound::hint_line(s.voices, id)?,
            MODE | 1 => 70,
            _ if (SCREEN_SPEED..SCREEN_SPEED + 3).contains(&id) => 71,
            TYPEWRITER => 57,
            TEXT_SPEED => 58,
            PUNCTUATION => 59,
            ids::SKIP => 60,
            SKIP_SPEED => 61,
            ids::AUTO => 62,
            AUTO_SPEED => 63,
            KEEP_AUTO => 64,
            KEEP_SKIP => 65,
            AUTO_SAVE => 66,
            TOOLTIPS => 67,
            FIXED_WINDOW => 68,
            _ if (COLOR..=ALPHA).contains(&id) => 69,
            PREVIOUS => 72,
            NEXT => 73,
            TAB => 74,
            3585 => 75,
            _ if (CONFIRM..CONFIRM + 10).contains(&id) => 76,
            _ if (RIGHT_CLICK..RIGHT_CLICK + 6).contains(&id) => 77,
            RESET => 78,
            _ => return None,
        };
        Some(self.system.as_ref()?.get(0, n)?.to_string())
    }

    /// Draws the settings. `state` gives the row (0 normal, 1 hover, 2
    /// pressed) of an element; `hover` is the element under the pointer;
    /// `skip` and `auto` are the game's modes; `under` is the game screen
    /// behind the menu and `clock` the time now.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_config(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        s: &ConfigScreen,
        st: &Settings,
        state: impl Fn(i32) -> u32,
        hover: Option<i32>,
        (skip, auto, can_save, has_log): (bool, bool, bool, bool),
        (under, clock): (Option<&Arc<Image>>, u64),
    ) -> Result<()> {
        let enabled = |id| Self::config_enabled(s, st, id, can_save, has_log);
        let side_state = |i: usize| {
            let id = ids::SIDE + i as i32;
            let current = match s.page {
                Page::Sound => side::SOUND,
                _ => side::SETTINGS,
            };
            if i == current {
                2
            } else if !enabled(id) {
                3
            } else {
                state(id).min(1)
            }
        };
        let hint = hover.and_then(|id| self.config_hint(s, id));
        if s.page == Page::Sound {
            self.draw_sound(canvas, assets, s, st, &state)?;
            self.draw_base(canvas, assets, hint.as_deref())?;
            self.draw_reset(
                canvas,
                assets,
                self.config_area(s.page, sound::ids::RESET),
                &state,
            )?;
            return self.draw_side(canvas, assets, side_state);
        }
        let bg = match s.page {
            Page::System1 => "system2_bg.akb",
            _ => "system1_bg.akb",
        };
        let bg = assets.localized_image(bg, self.language)?;
        canvas.draw(&bg, 0, 0, false);
        self.draw_base(canvas, assets, hint.as_deref())?;
        let sheet = assets.localized_image("system_p.akb", self.language)?;
        let areas = self.config_areas(s.page);
        let area = |id: i32| areas.iter().find(|a| a.id == id);
        // The game mode, on both pages.
        for (k, &(x, y)) in MODE_POS.iter().enumerate() {
            let id = ids::MODE + k as i32;
            let current = st.classic_mode == (k == 0);
            let row = if current { 2 } else { state(id).min(1) };
            let (px, py) = MODE_PART[k];
            let (w, h) = MODE_SIZE;
            canvas.draw_part(&sheet, (px, py + row * h, w, h), x, y, false, 255);
        }
        for a in areas {
            let id = a.id;
            if let Some(on) = switch(st, id, skip, auto) {
                let row = state(id).min(2) + if on { 0 } else { 3 };
                let (px, py, w, h) = SWITCH;
                canvas.draw_part(&sheet, (px, py + row * h, w, h), a.left, a.top, false, 255);
            } else if let Some(v) =
                slider(st, id).filter(|_| !is_speed(id) || s.speeds[speed_index(id)])
            {
                let (px, py, w, h) = KNOB;
                let row = state(id).min(2);
                let x = knob_x(a, v);
                canvas.draw_part(&sheet, (px, py + row * h, w, h), x, a.top, false, 255);
            }
        }
        if s.page == Page::System1 {
            for (base, chosen) in [
                (ids::SCREEN_SPEED, st.effect_speed.min(2) as i32),
                (ids::MAP_SPEED, st.map_speed.min(2) as i32),
            ] {
                if let Some(a) = area(base + chosen) {
                    let (px, py, w, h) = KNOB;
                    let x = a.left + (a.width() as i32 - w as i32) / 2;
                    let part = (px, py + CHOSEN_KNOB * h, w, h);
                    canvas.draw_part(&sheet, part, x, a.top, false, 255);
                }
            }
            for (k, lang) in LANGUAGES.iter().enumerate() {
                let id = ids::LANGUAGE + k as i32;
                let Some(a) = area(id) else { continue };
                let current = st.language.unwrap_or(self.language) == *lang;
                let row = if current { 2 } else { state(id).min(1) };
                let (px, w, h) = LANGUAGE_PART;
                let part = (px + k as u32 * w, row * h, w, h);
                canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
            }
            self.draw_sample(canvas, assets, st, under, clock.saturating_sub(s.opened));
        } else {
            for (k, action) in RIGHT_CLICKS.iter().enumerate() {
                let id = ids::RIGHT_CLICK + k as i32;
                let Some(a) = area(id) else { continue };
                let current = st.right_click == *action;
                let row = if current { 2 } else { state(id).min(1) };
                let (w, h, group) = RIGHT_PART;
                let part = ((k as u32 % 2) * w, (k as u32 / 2) * group + row * h, w, h);
                canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
            }
        }
        // The page tabs and reset.
        let current_tab = match s.page {
            Page::System1 => ids::TAB,
            _ => ids::TAB + 1,
        };
        for (k, id) in [ids::PREVIOUS, ids::TAB, ids::TAB + 1, ids::NEXT]
            .into_iter()
            .enumerate()
        {
            let Some(a) = area(id) else { continue };
            let row = if id == current_tab {
                2
            } else if !enabled(id) {
                3
            } else {
                state(id).min(1)
            };
            let part = (
                TABS_PART_X[k],
                TABS_PART_Y + row * TABS_H,
                TABS_PART_W[k],
                TABS_H,
            );
            canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
        }
        self.draw_reset(canvas, assets, area(ids::RESET), &state)?;
        self.draw_side(canvas, assets, side_state)
    }

    /// The reset button of `menu_p.akb`.
    fn draw_reset(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        a: Option<&Area>,
        state: &dyn Fn(i32) -> u32,
    ) -> Result<()> {
        if let Some(a) = a {
            let menu = assets.image("menu_p.akb")?;
            let (px, py, w, h) = RESET_PART;
            let part = (px, py + state(a.id).min(2) * h, w, h);
            canvas.draw_part(&menu, part, a.left, a.top, false, 255);
        }
        Ok(())
    }

    /// The window colour box: the game screen behind the menu, the
    /// window's tint over it and the sample line, typing out at the text
    /// speed `ms` after the screen opened, then starting again.
    fn draw_sample(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        st: &Settings,
        under: Option<&Arc<Image>>,
        ms: u64,
    ) {
        let (x, y, w, h) = SAMPLE;
        match under {
            Some(b) => canvas.draw_part(b, (x as u32, y as u32, w, h), x, y, false, 255),
            None => canvas.fill(x, y, w, h, [0, 0, 0, 255]),
        }
        let c = st
            .window_color
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        let a = (st.window_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        canvas.fill(x, y, w, h, [c[0], c[1], c[2], a]);
        let text = match self.language {
            Language::SimplifiedChinese => "留言样本",
            Language::TraditionalChinese => "留言樣本",
            _ => "Message Sample",
        };
        // Types out, then holds a second before it starts again (the hold
        // is a guess).
        let n = text.chars().count() as u64;
        let per = st.char_ms().max(1);
        let shown = (ms % (n * per + 1000) / per).min(n) as usize;
        let line: String = text.chars().take(shown).collect();
        if let Some(font) = self.font(assets) {
            font.draw_line(canvas, &line, SAMPLE_TEXT.0, SAMPLE_TEXT.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use silky_format::game::Title;

    #[test]
    fn game_mode_buttons_set_the_mode() {
        let mut st = Settings::for_title(Title::Doukyuusei);
        // A new install is in the easy mode.
        assert!(!st.classic_mode);
        assert_eq!(super::super::plate::game_mode(st.classic_mode), 1);
        assert!(apply(&mut st, ids::MODE, 0.0));
        assert!(st.classic_mode);
        assert_eq!(super::super::plate::game_mode(st.classic_mode), 0);
        assert!(apply(&mut st, 1, 0.0));
        assert!(!st.classic_mode);
    }

    #[test]
    fn reset_goes_back_to_the_easy_mode() {
        let defaults = Settings::for_title(Title::Doukyuusei);
        let mut st = defaults.clone();
        apply(&mut st, ids::MODE, 0.0);
        reset(&mut st, &defaults);
        assert!(!st.classic_mode);
    }

    #[test]
    fn game_mode_is_kept_with_the_system_data() {
        let mut st = Settings::for_title(Title::Doukyuusei);
        apply(&mut st, ids::MODE, 0.0);
        let json = serde_json::to_vec(&st).unwrap();
        let back: Settings = serde_json::from_slice(&json).unwrap();
        assert!(back.classic_mode);
        // Data written before the setting existed reads as easy.
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(!old.classic_mode);
    }
}
