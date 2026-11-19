//! The map jump page of 同級生 Remake: opened by the menu's map button on
//! the walking maps; picking a place ends the walk with `0xff00` and the
//! place's number, and SUB1.MES then walks the hero there.
//!
//! Read from the executable. The page is the town map picture
//! (`sakima_bg`) with one part per place from `sakima_p`; a place is
//! offered when the story has found it (a byte variable, or always).
//! Hovering a place draws its lit part and its name bubble from
//! `sakimaplate_p`, pressing it the pressed row. `SAKIMA_SUB.CAREA` gives
//! the place under a pixel (255 none, 100 the back button). Back and the
//! secondary click return to the walk.
//!
//! With the game mode setting on, the girl who will be at a place shows
//! as a face on it (see [`super::who`]). Only the town (mode 0) is done;
//! the yabuki and school pages are not. See
//! `docs/games/doukyuusei.md`.

use std::sync::Arc;

use super::plate::GAME_MODE;
use super::room::Click;
use super::who::{face_at, Map};
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::Result;
use silky_vm::Vm;

/// The byte that tells which map the page is for (0 the town).
pub const MODE_AT: usize = 7089;
/// Id of the back button.
pub const BACK: i32 = 100;
/// High byte of the result of a place chosen.
pub const RESULT: i32 = 0xff00;

const CAREA: &str = "SAKIMA_SUB.CAREA";
const W: usize = 1920;
const H: usize = 1080;
const PLACES: usize = 37;
/// Back button: source, size and place.
const BACK_PART: (u32, u32, u32) = (2377, 136, 136);
const BACK_AT: (i32, i32) = (1776, 937);

/// Where a place's byte flag is, or always offered.
#[derive(Clone, Copy)]
enum Flag {
    Always,
    Byte(usize),
    Never,
}

/// A place: its part on the sheet (x, y), where it goes, its size, and
/// whether it is offered.
struct Place {
    src: (u32, u32),
    at: (i32, i32),
    size: (u32, u32),
    flag: Flag,
}

const fn p(sx: u32, sy: u32, dx: i32, dy: i32, w: u32, h: u32, flag: Flag) -> Place {
    Place {
        src: (sx, sy),
        at: (dx, dy),
        size: (w, h),
        flag,
    }
}

use Flag::{Always as A, Byte as B, Never as N};

/// Id 19 is Reiko's house, whose art depends on a variant.
const REIKO: usize = 19;
const REIKO_X: [u32; 2] = [1002, 1327];

/// The town's places by id (the destination number is id + 1).
const TOWN: [Place; PLACES] = [
    p(984, 0, 1457, 27, 349, 164, A),
    p(1973, 0, 43, 15, 404, 209, A),
    p(320, 3950, 938, 573, 515, 283, A),
    p(2240, 2900, 1521, 632, 320, 182, A),
    p(835, 3950, 525, 598, 324, 218, A),
    p(1920, 2900, 1489, 536, 320, 255, A),
    p(2155, 3951, 1142, 707, 383, 338, A),
    p(640, 0, 1372, 22, 344, 174, A),
    p(639, 840, 888, 129, 363, 229, A),
    p(1652, 840, 1181, 194, 398, 216, A),
    p(319, 840, 563, 75, 320, 214, A),
    p(1333, 0, 945, 5, 320, 181, A),
    p(640, 1760, 1005, 351, 320, 179, A),
    p(320, 2900, 801, 354, 320, 191, A),
    p(960, 1760, 699, 188, 320, 181, A),
    p(1280, 2900, 1169, 453, 320, 199, A),
    p(1280, 1760, 466, 195, 338, 216, A),
    p(0, 840, 227, 154, 319, 165, A),
    p(0, 3950, 1485, 591, 320, 208, B(1701)),
    p(1327, 840, 1235, 56, 325, 197, N),
    p(0, 1760, 1524, 311, 320, 173, B(1703)),
    p(0, 0, 1522, 22, 320, 199, B(1704)),
    p(320, 0, 1442, 22, 320, 199, B(1705)),
    p(640, 2900, 467, 457, 320, 172, B(1706)),
    p(2538, 3950, 1530, 827, 320, 214, B(1707)),
    p(1618, 1760, 123, 272, 499, 278, A),
    p(2117, 1760, 123, 272, 499, 278, A),
    p(320, 1760, 1231, 311, 320, 186, B(1708)),
    p(1653, 0, 674, 28, 320, 194, B(1709)),
    p(2050, 840, 1482, 180, 320, 179, B(1710)),
    p(1600, 2900, 1560, 428, 320, 185, B(1711)),
    p(960, 2900, 812, 495, 320, 185, B(1712)),
    p(1835, 3950, 816, 814, 320, 185, B(1713)),
    p(0, 0, 0, 0, 0, 0, N),
    p(0, 2900, 517, 357, 320, 194, B(1697)),
    p(1515, 3950, 496, 831, 320, 205, B(1699)),
    p(1159, 3950, 150, 831, 356, 215, B(1698)),
];

/// The pin (place number for the schedule) of each town element, -1 none.
const PIN_OF: [i8; PLACES] = [
    0, 1, 5, 13, 12, 11, 14, 8, 7, 9, -1, -1, -1, -1, -1, -1, -1, -1, 15, 6, 10, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
/// Where a face goes (top-left) by pin; pin 1 uses [`SLOT_AT`].
const PIN_AT: [(i32, i32); 16] = [
    (1473, 59),
    (159, 19),
    (159, 19),
    (48, 57),
    (159, 10),
    (1134, 630),
    (1203, 100),
    (961, 194),
    (1347, 21),
    (1305, 245),
    (1664, 367),
    (1755, 557),
    (500, 648),
    (1688, 712),
    (1221, 839),
    (1476, 660),
];
/// The four places of Sakimake Academy share one pin; these are their spots.
const SLOT_AT: [(i32, i32); 4] = [(159, 19), (159, 19), (48, 57), (159, 10)];
/// The face sheet: 144-pixel cells, four to a row.
const FACES: &str = "map_p.akb";
const FACE: u32 = 144;

#[derive(Debug, Clone)]
pub struct Jump {
    map: Arc<Vec<u8>>,
    open: [bool; PLACES],
    /// Faces to draw: (cell, x, y).
    faces: Vec<(u8, i32, i32)>,
    /// Reiko's house art: 0 or 1.
    reiko: usize,
}

impl Jump {
    /// The page is for the town, see [`Jump::available`]; faces show
    /// with the game mode setting on.
    pub fn load(assets: &Assets, vm: &Vm) -> Result<Self> {
        let mut map = assets.data(CAREA)?;
        map.resize(W * H, 255);
        let mut j = Self::new(map, &vm.bytes);
        if GAME_MODE == 1 {
            j.faces = j.pins(|place| face_at(vm, Map::Town, place));
        }
        Ok(j)
    }

    /// The faces of the places offered; `who` gives the face at a place.
    fn pins(&self, who: impl Fn(u8) -> Option<u8>) -> Vec<(u8, i32, i32)> {
        let mut faces = Vec::new();
        for (id, &pin) in PIN_OF.iter().enumerate() {
            if pin < 0 || !self.offered(id) {
                continue;
            }
            let found = if pin == 1 {
                (0..4).find_map(|k| who(k + 1).map(|f| (f, SLOT_AT[k as usize])))
            } else {
                who(pin as u8).map(|f| (f, PIN_AT[pin as usize]))
            };
            faces.extend(found.map(|(f, (x, y))| (f, x, y)));
        }
        faces
    }

    fn new(map: Vec<u8>, bytes: &[u8]) -> Self {
        let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
        let mut open = [false; PLACES];
        for (o, place) in open.iter_mut().zip(TOWN.iter()) {
            *o = match place.flag {
                Flag::Always => true,
                Flag::Byte(i) => byte(i) != 0,
                Flag::Never => false,
            };
        }
        // Reiko's house: the first art once found, the second later.
        let reiko = match (byte(1702), byte(1692)) {
            (1, _) => Some(1),
            (0, 1) => Some(0),
            _ => None,
        };
        open[REIKO] = reiko.is_some();
        Self {
            map: Arc::new(map),
            open,
            faces: Vec::new(),
            reiko: reiko.unwrap_or(0),
        }
    }

    /// Whether the page is done for the map the script is on.
    pub fn available(bytes: &[u8]) -> bool {
        bytes.get(MODE_AT).copied().unwrap_or(0) == 0
    }

    /// Whether place `id` is offered.
    pub fn offered(&self, id: usize) -> bool {
        self.open.get(id).copied().unwrap_or(false)
    }

    /// The place (or [`BACK`]) under (x, y); none for a place not offered.
    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 {
            return None;
        }
        let i = self.map[y as usize * W + x as usize] as i32;
        (i == BACK || (i < PLACES as i32 && self.offered(i as usize))).then_some(i)
    }

    pub fn click(&mut self, id: i32) -> Click {
        match id {
            BACK => Click::Resume,
            0..=36 if self.offered(id as usize) => Click::Result(RESULT | (id + 1)),
            _ => Click::None,
        }
    }

    fn place(&self, id: usize) -> (u32, u32, i32, i32, u32, u32) {
        let t = &TOWN[id];
        let sx = if id == REIKO {
            REIKO_X[self.reiko]
        } else {
            t.src.0
        };
        (sx, t.src.1, t.at.0, t.at.1, t.size.0, t.size.1)
    }

    /// `state` gives the row (0 normal, 1 hover, 2 pressed) of an id.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        language: Language,
        state: impl Fn(i32) -> u32,
    ) -> Result<()> {
        let bg = assets.localized_image("sakima_bg.akb", language)?;
        let parts = assets.localized_image("sakima_p.akb", language)?;
        let plates = assets.localized_image("sakimaplate_p.akb", language)?;
        canvas.draw_part(&bg, (0, 0, W as u32, H as u32), 0, 0, false, 255);
        let mut label = None;
        for id in (0..PLACES).filter(|&i| self.offered(i)) {
            let row = state(id as i32).min(2);
            let (sx, sy, dx, dy, w, h) = self.place(id);
            canvas.draw_part(&parts, (sx, sy + h * row, w, h), dx, dy, false, 255);
            if row > 0 {
                label = Some((id, row));
            }
        }
        // The name bubble of the place lit, over the parts.
        if let Some((id, row)) = label {
            let (sx, sy, dx, dy, w, h) = self.place(id);
            canvas.draw_part(&plates, (sx, sy + h * row, w, h), dx, dy, false, 255);
        }
        if !self.faces.is_empty() {
            let sheet = assets.localized_image(FACES, language)?;
            for &(f, x, y) in &self.faces {
                let src = (f as u32 & 3) * FACE;
                canvas.draw_part(
                    &sheet,
                    (src, (f as u32 >> 2) * FACE, FACE, FACE),
                    x,
                    y,
                    false,
                    255,
                );
            }
        }
        let row = state(BACK).min(2);
        let (sx, w, h) = BACK_PART;
        canvas.draw_part(
            &parts,
            (sx, h * row, w, h),
            BACK_AT.0,
            BACK_AT.1,
            false,
            255,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jump(bytes: &[u8]) -> Jump {
        let mut map = vec![255u8; W * H];
        map[0] = 2;
        map[1] = 21;
        map[2] = 100;
        Jump::new(map, bytes)
    }

    #[test]
    fn offered_places() {
        let mut bytes = vec![0u8; 2000];
        let j = jump(&bytes);
        assert!(j.offered(0) && j.offered(25) && j.offered(26));
        assert!(!j.offered(18) && !j.offered(19) && !j.offered(33));
        bytes[1701] = 1;
        assert!(jump(&bytes).offered(18));
        bytes[1692] = 1;
        let j = jump(&bytes);
        assert!(j.offered(19) && j.reiko == 0);
        bytes[1702] = 1;
        assert_eq!(jump(&bytes).reiko, 1);
    }

    #[test]
    fn pins() {
        let mut bytes = vec![0u8; 2000];
        bytes[1701] = 1;
        let j = jump(&bytes);
        // Everyone at pin 3 and, for the academy, only on slot 2 (pin 3).
        let faces = j.pins(|p| (p == 3 || p == 15 || p == 12).then_some(p));
        // Place 3 is the third slot; the coffee house (pin 13) is empty, the
        // boutique (15) found.
        assert!(faces.contains(&(3, 48, 57)));
        assert!(faces.contains(&(15, 1476, 660)));
        // Station (pin 5), found by default, has nobody; pin 12 (the
        // drugstore, id 4) is offered.
        assert!(faces.contains(&(12, 500, 648)));
        assert_eq!(faces.len(), 3);
        // A place not found (Reiko's house) shows nobody.
        assert!(j.pins(|p| (p == 6).then_some(1)).is_empty());
    }

    #[test]
    fn clicks() {
        let mut j = jump(&[]);
        assert_eq!(j.hit(0, 0), Some(2));
        // Not found yet: no hover, no click.
        assert_eq!(j.hit(1, 0), None);
        assert_eq!(j.hit(2, 0), Some(BACK));
        assert_eq!(j.hit(3, 0), None);
        assert!(matches!(j.click(2), Click::Result(0xff03)));
        assert!(matches!(j.click(21), Click::None));
        assert!(matches!(j.click(BACK), Click::Resume));
        assert!(Jump::available(&[0u8; 8000]));
    }
}
