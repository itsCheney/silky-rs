//! The sound page of 同級生 Remake's settings (`config02.area`).
//!
//! Read from the executable and compared under Wine. The page is
//! `sound2_bg_<lang>.akb` with the panel of each character slot in use
//! copied from `sound1_bg_<lang>.akb`; the parts come from
//! `sound_p_<lang>.akb`: faces (150 x 150, four to a row), names (230 x 46
//! a row), digits and the per cent sign, the character page tabs, ON and
//! OFF switches, knobs and the slider bars. A slider is a dark bar with a
//! pink fill up to the knob (red while the pointer is on it); a slider
//! whose switch is off shows only the dark bar. Fourteen characters and
//! everyone else take three pages of six.

use std::sync::Arc;

use silky_format::akb::Image;
use silky_format::area::Area;

use super::config::ConfigScreen;
use super::system::System;
use crate::assets::Assets;
use crate::render::Canvas;
use crate::system::{Settings, VOICE_GROUP_SLOTS};

/// Ids of `config02.area`.
pub mod ids {
    pub const MASTER_ON: i32 = 0;
    pub const MASTER: i32 = 1;
    pub const MUSIC_ON: i32 = 256;
    pub const MUSIC: i32 = 257;
    pub const EFFECTS_ON: i32 = 512;
    pub const EFFECTS: i32 = 513;
    pub const VOICE_ON: i32 = 768;
    pub const VOICE: i32 = 769;
    /// "Click to stop voices".
    pub const CUT_VOICE: i32 = 1024;
    /// The character pages: previous, next, then the three tabs.
    pub const PREVIOUS: i32 = 1280;
    pub const NEXT: i32 = 1281;
    pub const TAB: i32 = 1536;
    /// A character slot's switch and slider (six slots).
    pub const CHAR_ON: i32 = 1792;
    pub const CHAR: i32 = 2048;
    pub const RESET: i32 = 2304;
}

/// Character slots on a page, and pages.
pub const SLOTS: usize = 6;
pub const PAGES: usize = VOICE_GROUP_SLOTS.div_ceil(SLOTS);

/// The sample each slider plays when it is let go (listed in the
/// executable): effects, voice, then the characters in page order. Music
/// has none.
const EFFECTS_SAMPLE: &str = "pinpon.wav";
const VOICE_SAMPLE: &str = "v0029.ogg";
const CHAR_SAMPLES: [&str; VOICE_GROUP_SLOTS] = [
    "v9998.ogg",
    "l9998.ogg",
    "p9998.ogg",
    "m9998.ogg",
    "q9998.ogg",
    "t9998.ogg",
    "g9998.ogg",
    "n9998.ogg",
    "e9998.ogg",
    "u9998.ogg",
    "o9998.ogg",
    "j9998.ogg",
    "c9998.ogg",
    "d9998.ogg",
    "i9998.ogg",
];

/// The parts of `sound_p_<lang>.akb`.
const FACE: u32 = 150;
const NAME: (u32, u32, u32) = (600, 230, 46);
const DIGIT: (u32, u32, u32) = (832, 20, 26);
const PERCENT: (u32, u32, u32, u32) = (1032, 0, 26, 26);
/// Digits stand 25 pixels apart, the per cent sign 28 after the last.
const DIGIT_STEP: i32 = 25;
const PERCENT_STEP: i32 = 28;
/// The page tabs: previous, 1, 2, 3, next, 56 x 56 from (832, 26); rows
/// normal, hover, current, gray.
const TABS: (u32, u32, u32) = (832, 26, 56);
const SWITCH: (u32, u32, u32, u32) = (1112, 0, 86, 50);
/// Knobs: rows normal, hover, pressed.
const KNOB: (u32, u32, u32, u32) = (1200, 0, 36, 60);
/// The dark bar and the fills (normal, hover), and where they go in a
/// slider's area.
const BAR: (u32, u32, u32, u32) = (1236, 0, 602, 42);
const BAR_AT: (i32, i32) = (14, 10);
const FILL: (u32, u32, u32) = (1240, 592, 30);
const FILL_ROWS: [u32; 2] = [48, 90];
const FILL_AT: (i32, i32) = (18, 16);
/// The effects and voice sliders sit 4 pixels lower in their areas than
/// the others (measured under Wine).
const LOWER: i32 = 4;
/// How far the knob travels.
const SPAN: f32 = 594.0;
/// The per cent sign of the four main sliders.
const MAIN_PERCENT: [(i32, i32); 4] = [(709, 86), (1507, 86), (709, 234), (1507, 234)];
/// The per cent sign of a character, from its slider's area.
const CHAR_PERCENT: (i32, i32) = (413, -49);
/// A character slot: its panel (copied from `sound1_bg`), face and name.
const PANEL: (i32, i32, u32, u32) = (280, 469, 784, 152);
const SLOT_STEP: (i32, i32) = (792, 160);
const FACE_AT: (i32, i32) = (1, 1);
const NAME_AT: (i32, i32) = (228, 9);

/// The character in slot `slot` of page `page`.
pub fn character(page: usize, slot: usize) -> Option<usize> {
    Some(page * SLOTS + slot).filter(|&k| k < VOICE_GROUP_SLOTS)
}

pub fn is_slider(id: i32) -> bool {
    matches!(id, ids::MASTER | ids::MUSIC | ids::EFFECTS | ids::VOICE)
        || (ids::CHAR..ids::CHAR + SLOTS as i32).contains(&id)
}

/// The value a slider takes for pointer x.
pub fn slider_value(a: &Area, x: i32) -> f32 {
    ((x - a.left - KNOB.2 as i32 / 2) as f32 / SPAN).clamp(0.0, 1.0)
}

/// The switch and value of a slider, or of a switch.
fn control(st: &Settings, page: usize, id: i32) -> Option<(bool, f32)> {
    let slot = |base: i32| character(page, (id - base) as usize);
    Some(match id {
        ids::MASTER_ON | ids::MASTER => (st.master_on, st.master),
        ids::MUSIC_ON | ids::MUSIC => (st.music_on, st.music),
        ids::EFFECTS_ON | ids::EFFECTS => (st.effects_on, st.effects),
        ids::VOICE_ON | ids::VOICE => (st.voice_on, st.voice),
        _ if (ids::CHAR_ON..ids::CHAR_ON + SLOTS as i32).contains(&id) => {
            let k = slot(ids::CHAR_ON)?;
            (st.voice_group_on[k], st.voice_group[k])
        }
        _ if (ids::CHAR..ids::CHAR + SLOTS as i32).contains(&id) => {
            let k = slot(ids::CHAR)?;
            (st.voice_group_on[k], st.voice_group[k])
        }
        _ => return None,
    })
}

fn is_switch(id: i32) -> bool {
    matches!(
        id,
        ids::MASTER_ON | ids::MUSIC_ON | ids::EFFECTS_ON | ids::VOICE_ON | ids::CUT_VOICE
    ) || (ids::CHAR_ON..ids::CHAR_ON + SLOTS as i32).contains(&id)
}

/// Whether an element of the page does something now: a slider only
/// while its switch is on, the page arrows while there is a page that
/// way, a character slot only when it holds a character.
pub fn enabled(st: &Settings, page: usize, id: i32) -> bool {
    match id {
        ids::PREVIOUS => page > 0,
        ids::NEXT => page + 1 < PAGES,
        _ if is_slider(id) => control(st, page, id).is_some_and(|(on, _)| on),
        _ => control(st, page, id).is_some() || !is_switch(id) || id == ids::CUT_VOICE,
    }
}

/// Changes a setting from a click on `id`; `value` is the slider
/// position. Returns false if `id` is not a setting.
pub fn apply(st: &mut Settings, page: usize, id: i32, value: f32) -> bool {
    let char_at = |base: i32| character(page, (id - base) as usize);
    match id {
        ids::MASTER_ON => st.master_on ^= true,
        ids::MUSIC_ON => st.music_on ^= true,
        ids::EFFECTS_ON => st.effects_on ^= true,
        ids::VOICE_ON => st.voice_on ^= true,
        ids::CUT_VOICE => st.cut_voice ^= true,
        ids::MASTER => st.master = value,
        ids::MUSIC => st.music = value,
        ids::EFFECTS => st.effects = value,
        ids::VOICE => st.voice = value,
        _ if (ids::CHAR_ON..ids::CHAR_ON + SLOTS as i32).contains(&id) => {
            match char_at(ids::CHAR_ON) {
                Some(k) => st.voice_group_on[k] ^= true,
                None => return false,
            }
        }
        _ if (ids::CHAR..ids::CHAR + SLOTS as i32).contains(&id) => match char_at(ids::CHAR) {
            Some(k) => st.voice_group[k] = value,
            None => return false,
        },
        _ => return false,
    }
    true
}

/// The sample a slider plays when it is let go.
pub fn sample(page: usize, id: i32) -> Option<&'static str> {
    match id {
        ids::EFFECTS => Some(EFFECTS_SAMPLE),
        ids::VOICE => Some(VOICE_SAMPLE),
        _ if (ids::CHAR..ids::CHAR + SLOTS as i32).contains(&id) => {
            character(page, (id - ids::CHAR) as usize).map(|k| CHAR_SAMPLES[k])
        }
        _ => None,
    }
}

/// Puts back the defaults of the sound page.
pub fn reset(st: &mut Settings, d: &Settings) {
    st.master_on = d.master_on;
    st.master = d.master;
    st.music_on = d.music_on;
    st.music = d.music;
    st.effects_on = d.effects_on;
    st.effects = d.effects;
    st.voice_on = d.voice_on;
    st.voice = d.voice;
    st.voice_group_on = d.voice_group_on;
    st.voice_group = d.voice_group;
    st.cut_voice = d.cut_voice;
}

/// The line of `SYSTEM_<lang>.MAP` that is the hint for `id`.
pub fn hint_line(page: usize, id: i32) -> Option<u32> {
    Some(match id {
        ids::MASTER_ON => 87,
        ids::MASTER => 88,
        ids::MUSIC_ON => 89,
        ids::MUSIC => 90,
        ids::EFFECTS_ON => 91,
        ids::EFFECTS => 92,
        ids::VOICE_ON => 93,
        ids::VOICE => 94,
        _ if (ids::CHAR_ON..ids::CHAR_ON + SLOTS as i32).contains(&id) => {
            95 + character(page, (id - ids::CHAR_ON) as usize)? as u32
        }
        _ if (ids::CHAR..ids::CHAR + SLOTS as i32).contains(&id) => {
            110 + character(page, (id - ids::CHAR) as usize)? as u32
        }
        ids::CUT_VOICE => 125,
        ids::PREVIOUS => 126,
        ids::NEXT => 127,
        _ if (ids::TAB..ids::TAB + PAGES as i32).contains(&id) => 128 + (id - ids::TAB) as u32,
        ids::RESET => 131,
        _ => return None,
    })
}

/// Draws a percentage with its sign at `x` (right aligned).
fn draw_percent(canvas: &mut Canvas, sheet: &Arc<Image>, value: f32, x: i32, y: i32) {
    let (px, py, w, h) = PERCENT;
    canvas.draw_part(sheet, (px, py, w, h), x, y, false, 255);
    let n = (value.clamp(0.0, 1.0) * 100.0).round() as u32;
    let digits = n.to_string();
    let count = digits.len() as i32;
    for (i, d) in digits.bytes().enumerate() {
        let (dx, dw, dh) = DIGIT;
        let part = (dx + (d - b'0') as u32 * dw, 0, dw, dh);
        let at = x - PERCENT_STEP - (count - 1 - i as i32) * DIGIT_STEP;
        canvas.draw_part(sheet, part, at, y, false, 255);
    }
}

/// Draws a slider: the dark bar, then (while on) the fill and the knob.
fn draw_slider(canvas: &mut Canvas, sheet: &Arc<Image>, a: &Area, on: bool, v: f32, row: u32) {
    let top = a.top
        + if matches!(a.id, ids::EFFECTS | ids::VOICE) {
            LOWER
        } else {
            0
        };
    let (bx, by, bw, bh) = BAR;
    canvas.draw_part(
        sheet,
        (bx, by, bw, bh),
        a.left + BAR_AT.0,
        top + BAR_AT.1,
        false,
        255,
    );
    if !on {
        return;
    }
    let travel = (v.clamp(0.0, 1.0) * SPAN).round() as i32;
    let (fx, fw, fh) = FILL;
    let width = travel.min(fw as i32) as u32;
    if width > 0 {
        let part = (fx, FILL_ROWS[(row > 0) as usize], width, fh);
        canvas.draw_part(sheet, part, a.left + FILL_AT.0, top + FILL_AT.1, false, 255);
    }
    let (kx, ky, kw, kh) = KNOB;
    let part = (kx, ky + row.min(2) * kh, kw, kh);
    canvas.draw_part(sheet, part, a.left + travel, top, false, 255);
}

impl System {
    /// Draws the sound page's own parts over the system menu's.
    pub(super) fn draw_sound(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        s: &ConfigScreen,
        st: &Settings,
        state: &dyn Fn(i32) -> u32,
    ) -> crate::Result<()> {
        let bg = assets.localized_image("sound2_bg.akb", self.language)?;
        canvas.draw(&bg, 0, 0, false);
        let panels = assets.localized_image("sound1_bg.akb", self.language)?;
        let sheet = assets.localized_image("sound_p.akb", self.language)?;
        let page = s.voices;
        for slot in 0..SLOTS {
            let Some(k) = character(page, slot) else {
                continue;
            };
            let (x, y) = (
                PANEL.0 + (slot % 2) as i32 * SLOT_STEP.0,
                PANEL.1 + (slot / 2) as i32 * SLOT_STEP.1,
            );
            let part = (x as u32, y as u32, PANEL.2, PANEL.3);
            canvas.draw_part(&panels, part, x, y, false, 255);
            let face = ((k as u32 % 4) * FACE, (k as u32 / 4) * FACE, FACE, FACE);
            canvas.draw_part(&sheet, face, x + FACE_AT.0, y + FACE_AT.1, false, 255);
            let (nx, nw, nh) = NAME;
            let name = (nx, k as u32 * nh, nw, nh);
            canvas.draw_part(&sheet, name, x + NAME_AT.0, y + NAME_AT.1, false, 255);
        }
        let main = [ids::MASTER, ids::MUSIC, ids::EFFECTS, ids::VOICE];
        for a in self.config[2].iter() {
            let id = a.id;
            if id == ids::CUT_VOICE || (is_switch(id) && control(st, page, id).is_some()) {
                let on = match id {
                    ids::CUT_VOICE => st.cut_voice,
                    _ => control(st, page, id).is_some_and(|(on, _)| on),
                };
                let row = state(id).min(2) + if on { 0 } else { 3 };
                let (px, py, w, h) = SWITCH;
                canvas.draw_part(&sheet, (px, py + row * h, w, h), a.left, a.top, false, 255);
            } else if is_slider(id) {
                let Some((on, v)) = control(st, page, id) else {
                    continue;
                };
                draw_slider(canvas, &sheet, a, on, v, state(id));
                if on {
                    let (x, y) = match main.iter().position(|&m| m == id) {
                        Some(i) => MAIN_PERCENT[i],
                        None => (a.left + CHAR_PERCENT.0, a.top + CHAR_PERCENT.1),
                    };
                    draw_percent(canvas, &sheet, v, x, y);
                }
            }
        }
        // The character page tabs.
        let tabs = [
            ids::PREVIOUS,
            ids::TAB,
            ids::TAB + 1,
            ids::TAB + 2,
            ids::NEXT,
        ];
        for (k, id) in tabs.into_iter().enumerate() {
            let Some(a) = self.config[2].iter().find(|a| a.id == id) else {
                continue;
            };
            let row = if id == ids::TAB + page as i32 {
                2
            } else if !enabled(st, page, id) {
                3
            } else {
                state(id).min(1)
            };
            let (tx, ty, size) = TABS;
            let part = (tx + k as u32 * size, ty + row * size, size, size);
            canvas.draw_part(&sheet, part, a.left, a.top, false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifteen_characters_on_three_pages() {
        assert_eq!(PAGES, 3);
        assert_eq!(character(2, 2), Some(14));
        assert_eq!(character(2, 3), None);
        assert_eq!(hint_line(2, ids::CHAR + 2), Some(124));
    }
}
