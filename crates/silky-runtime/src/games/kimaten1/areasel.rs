//! Investigation over the background (`Game.41`).
//!
//! The call names a `.carea` file next to the background: one byte per
//! pixel of the 1280x720 screen, 255 for nothing and otherwise the number of
//! the hotspot there (1 to 7). The hotspot under the pointer is tinted pale
//! yellow and the help line reads "look here". A press on a hotspot answers
//! its number; the back button, right click and Escape answer 100. The
//! number that comes first in the call is not used by the screen.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use silky_format::akb::Image;
use silky_format::area::Area;

use super::chrome::{self, BAR_LEN};
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// Id of the back button.
pub const BACK: i32 = 0x100;
const BACK_ANSWER: i32 = 100;
const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
/// The tint of a hotspot, RGBA.
const TINT: [u8; 4] = [0xff, 0xff, 0xe0, 0x40];
const HOTSPOT_HELP: &str = "ここを調べる";
const BACK_HELP: &str = "行動選択に戻る";
const CLICK: &str = "ses_001.wav";

/// How often the headless run pressed a hotspot: (picture of the screen,
/// hotspot). A hotspot can say something new each time it is looked at.
static TRIED: Mutex<Option<HashMap<(u64, i32), u8>>> = Mutex::new(None);
const LOOKS: u8 = 3;

#[derive(Debug, Clone)]
pub struct AreaSelect {
    areas: Vec<Area>,
    map: Arc<Vec<u8>>,
    off: [bool; BAR_LEN],
    sound: Option<&'static str>,
    /// Identifies the picture of hotspots, for the headless run.
    scene: u64,
}

impl AreaSelect {
    pub fn open(areas: Vec<Area>, map: Vec<u8>, notebook: bool) -> Self {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        map.hash(&mut h);
        let scene = h.finish();
        let mut off = [false; BAR_LEN];
        off[7] = !notebook;
        Self {
            areas,
            map: Arc::new(map),
            off,
            sound: None,
            scene,
        }
    }

    /// The numbers of the hotspots in the picture, ascending.
    fn hotspots(&self) -> Vec<i32> {
        let mut seen = [false; 8];
        for &b in self.map.iter() {
            if (1..=7).contains(&b) {
                seen[b as usize] = true;
            }
        }
        (1..=7).filter(|&i| seen[i as usize]).collect()
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if let Some(a) = chrome::hit_area(&self.areas, x, y, |id| {
            id == BACK || chrome::bar_index(id).is_some()
        }) {
            return Some(a.id);
        }
        if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
            return None;
        }
        match self.map.get((y * WIDTH + x) as usize) {
            Some(&b) if (1..=7).contains(&b) => Some(b as i32),
            _ => None,
        }
    }

    pub fn enabled(&self, id: i32) -> bool {
        chrome::bar_index(id).map_or(true, |i| !self.off[i])
    }

    pub fn click(&mut self, id: i32) -> Click {
        if let Some(c) = chrome::bar_click(id) {
            return c;
        }
        if id == BACK {
            self.sound = Some(CLICK);
            return Click::Result(BACK_ANSWER);
        }
        if (1..=7).contains(&id) {
            *TRIED
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .entry((self.scene, id))
                .or_default() += 1;
            return Click::Result(id);
        }
        Click::None
    }

    pub fn cancel(&mut self) -> Click {
        self.sound = Some(CLICK);
        Click::Result(BACK_ANSWER)
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        // The tint goes under the interface.
        if let Some(id) = look.hover.filter(|id| (1..=7).contains(id)) {
            let mut rgba = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
            for (i, &b) in self.map.iter().enumerate().take((WIDTH * HEIGHT) as usize) {
                if b as i32 == id {
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&TINT);
                }
            }
            let img = Arc::new(Image {
                width: WIDTH as u32,
                height: HEIGHT as u32,
                rgba,
            });
            canvas.draw(&img, 0, 0, false);
        }
        let sheet = chrome::draw_common(canvas, assets, look, &self.off)?;
        canvas.draw_part(&sheet, (1143, 654, 419, 100), 860, 29, false, 255);
        if let Some(a) = self.areas.iter().find(|a| a.id == BACK) {
            let row = chrome::state(look, BACK);
            canvas.draw_part(
                &sheet,
                (1143, 854 + 57 * row, 78, 57),
                a.left,
                a.top,
                false,
                255,
            );
        }
        let help = match look.hover {
            Some(BACK) => Some(BACK_HELP),
            Some(id) if (1..=7).contains(&id) => Some(HOTSPOT_HELP),
            _ => chrome::bar_help(look),
        };
        chrome::draw_help(canvas, look, help);
        Ok(())
    }

    /// Headless run: the hotspot of the picture looked at least so far, the
    /// first of them, until each was looked at a few times; then back.
    pub fn auto_pick(&self) -> Option<i32> {
        let tried = TRIED.lock().unwrap();
        let count = |s: i32| {
            tried
                .as_ref()
                .and_then(|t| t.get(&(self.scene, s)))
                .copied()
                .unwrap_or(0)
        };
        let fresh = self
            .hotspots()
            .into_iter()
            .min_by_key(|&s| (count(s), s))
            .filter(|&s| count(s) < LOOKS);
        Some(fresh.unwrap_or(BACK))
    }
}
