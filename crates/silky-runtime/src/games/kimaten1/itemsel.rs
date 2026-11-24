//! Choosing a piece of evidence (`Game.46`).
//!
//! Five pieces of evidence can be held, each switched on by one byte of
//! the script (224, 231, 212, 213 and 138). Those held are packed into the
//! first positions of `itemselect.area`; a press answers the piece's
//! number from 1. There is no way out except the system buttons.

use silky_format::area::Area;

use super::chrome::{self, BAR_LEN};
use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// The bytes that hold each piece.
pub const FLAGS: [usize; 5] = [224, 231, 212, 213, 138];
const SHEET: &str = "pursue_p.akb";
const CELL: u32 = 208;
const NAMES: [&str; 5] = [
    "スケッチブック",
    "録音データ",
    "取引現場の証拠写真",
    "パープルメス",
    "注射器",
];
const CLICK: &str = "ses_001.wav";

#[derive(Debug, Clone)]
pub struct ItemSelect {
    areas: Vec<Area>,
    /// The pieces held, in order.
    items: Vec<u8>,
    off: [bool; BAR_LEN],
    sound: Option<&'static str>,
}

impl ItemSelect {
    pub fn open(areas: Vec<Area>, flags: [u8; 5], notebook: bool) -> Self {
        let mut off = [false; BAR_LEN];
        off[7] = !notebook;
        Self {
            areas,
            items: (0..5u8).filter(|&i| flags[i as usize] != 0).collect(),
            off,
            sound: None,
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        chrome::hit_area(&self.areas, x, y, |id| {
            chrome::bar_index(id).is_some() || (0..self.items.len() as i32).contains(&id)
        })
        .map(|a| a.id)
    }

    pub fn enabled(&self, id: i32) -> bool {
        chrome::bar_index(id).map_or(true, |i| !self.off[i])
    }

    pub fn click(&mut self, id: i32) -> Click {
        if let Some(c) = chrome::bar_click(id) {
            return c;
        }
        match usize::try_from(id).ok().and_then(|k| self.items.get(k)) {
            Some(&item) => {
                self.sound = Some(CLICK);
                Click::Result(item as i32 + 1)
            }
            None => Click::None,
        }
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let pieces = assets.image(SHEET)?;
        let sheet = chrome::draw_common(canvas, assets, look, &self.off)?;
        canvas.draw_part(&sheet, (1221, 854, 340, 80), 940, 29, false, 255);
        let mut help = chrome::bar_help(look);
        for (k, &item) in self.items.iter().enumerate() {
            let Some(a) = self.areas.iter().find(|a| a.id == k as i32) else {
                continue;
            };
            let row = chrome::state(look, k as i32);
            canvas.draw_part(
                &pieces,
                (CELL * item as u32, CELL * row, CELL, CELL),
                a.left,
                a.top,
                false,
                255,
            );
            if look.hover == Some(k as i32) {
                help = Some(NAMES[item as usize]);
            }
        }
        chrome::draw_help(canvas, look, help);
        Ok(())
    }

    /// Headless run: the first piece.
    pub fn auto_pick(&self) -> Option<i32> {
        (!self.items.is_empty()).then_some(0)
    }
}
