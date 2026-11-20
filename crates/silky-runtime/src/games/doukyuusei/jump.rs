//! The map jump page of 同級生 Remake: opened by the menu's map button on
//! the walking maps; picking a place ends the walk with `0xff00` and the
//! place's number, and the caller's sub script then walks the hero there.
//!
//! Read from the executable. Byte 7089 ([`MODE_AT`]) picks the picture: the
//! town (0, `sakima_*`), Yabuki (1, `yabuki_*`) or the school (2 to 5,
//! `school_*`, one picture of all four floors). DEN (6) has no page. Each
//! picture has a background, a sheet of parts per place, a sheet of name
//! bubbles and a click map (`*.CAREA`, a byte per pixel: the place, 255 none,
//! 100 back). A place is offered when the story has found it (a byte
//! variable, or always), and the school hides four stairs on the 2F.
//! Hovering a place draws its lit part and its name bubble, pressing it the
//! pressed row. Back and the secondary click return to the walk.
//!
//! With the game mode setting on, the girl who will be at a place shows as
//! a face on it (see [`super::who`]). The tables and their sources are in
//! `docs/games/doukyuusei.md`.

use std::sync::Arc;

use super::room::Click;
use super::who::{face_at, Map};
use crate::assets::{Assets, Language};
use crate::render::Canvas;
use crate::{Error, Result};
use silky_vm::Vm;

/// The byte that tells which map the page is for.
pub const MODE_AT: usize = 7089;
/// Id of the back button.
pub const BACK: i32 = 100;
/// High byte of the result of a place chosen.
pub const RESULT: i32 = 0xff00;

const W: usize = 1920;
const H: usize = 1080;
/// Back button: source size, and where it goes.
const BACK_SIZE: u32 = 136;
const BACK_AT: (i32, i32) = (1776, 937);

/// Where a place's byte flag is, or always offered.
#[derive(Clone, Copy, Debug)]
enum Flag {
    Always,
    Byte(usize),
    Never,
    /// Offered unless the page is the school's 2F (mode 3).
    Hidden3,
}

/// A place: its part on the sheet (x, y), where it goes, its size, and
/// whether it is offered.
#[derive(Debug)]
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

/// Id 19 is Reiko's house, whose art depends on a variant (town only).
const REIKO: usize = 19;
const REIKO_X: [u32; 2] = [1002, 1327];

/// The town's places by id (the destination number is id + 1).
const TOWN_PLACES: &[Place] = &[
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

/// The yabuki's places by id.
const YABUKI_PLACES: &[Place] = &[
    p(351, 1240, 248, 608, 1065, 255, A),
    p(739, 0, 894, 22, 624, 147, A),
    p(1747, 1240, 670, 832, 577, 201, A),
    p(2324, 1240, 670, 832, 576, 201, A),
    p(0, 0, 43, 22, 739, 147, A),
    p(0, 1240, 413, 321, 351, 305, B(1715)),
    p(1416, 1240, 569, 788, 331, 257, B(1716)),
    p(2148, 0, 1133, 398, 354, 276, B(1717)),
    p(1363, 0, 1129, 47, 388, 279, B(1718)),
    p(1751, 0, 1187, 226, 397, 306, B(1719)),
];

/// The school's places by id, the four floors on one picture.
const SCHOOL_PLACES: &[Place] = &[
    p(1851, 0, 1202, 864, 320, 167, A),
    p(2171, 0, 1178, 620, 320, 158, Flag::Hidden3),
    p(1115, 0, 1511, 757, 395, 190, A),
    p(1510, 0, 967, 757, 341, 190, A),
    p(715, 0, 968, 521, 400, 222, A),
    p(0, 0, 1511, 521, 395, 222, A),
    p(395, 0, 1296, 521, 320, 222, A),
    p(1830, 900, 1259, 224, 320, 158, A),
    p(2150, 900, 1179, 113, 320, 158, A),
    p(1507, 900, 968, 248, 323, 190, A),
    p(1115, 900, 1511, 248, 392, 190, A),
    p(715, 900, 968, 13, 400, 222, A),
    p(0, 900, 1511, 13, 395, 222, A),
    p(395, 900, 1296, 13, 320, 222, A),
    p(2491, 1800, 299, 224, 320, 158, Flag::Hidden3),
    p(2171, 1800, 218, 113, 320, 158, Flag::Hidden3),
    p(1519, 1800, 132, 248, 320, 190, A),
    p(1839, 1800, 17, 248, 332, 309, A),
    p(1115, 1800, 550, 248, 404, 190, A),
    p(720, 1800, 15, 13, 395, 222, A),
    p(0, 1800, 552, 13, 400, 222, A),
    p(400, 1800, 340, 13, 320, 222, A),
    p(2276, 3040, 299, 732, 320, 158, Flag::Hidden3),
    p(1956, 3040, 218, 620, 320, 158, A),
    p(722, 3040, 14, 521, 395, 222, A),
    p(0, 3040, 552, 521, 402, 222, A),
    p(1521, 3040, 17, 757, 435, 190, A),
    p(1117, 3040, 550, 757, 404, 190, A),
    p(402, 3040, 339, 521, 320, 222, A),
];

/// The pin (place number for the schedule) of each town element, -1 none.
const TOWN_PIN_OF: &[i8] = &[
    0, 1, 5, 13, 12, 11, 14, 8, 7, 9, -1, -1, -1, -1, -1, -1, -1, -1, 15, 6, 10, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
/// Where a face goes (top-left) by pin; the town's pin 1 uses [`SLOT_AT`].
const TOWN_PIN_AT: &[(i32, i32)] = &[
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

/// Yabuki: the pin of each place, and where each pin's face goes.
const YABUKI_PIN_OF: &[i8] = &[0, 1, 6, -1, 2, 5, 7, -1, 3, 4];
const YABUKI_PIN_AT: &[(i32, i32)] = &[
    (939, 621),
    (960, 16),
    (464, 23),
    (1221, 56),
    (1260, 253),
    (328, 378),
    (991, 876),
    (593, 840),
];

/// School: the pin of each place, and where each pin's face goes.
const SCHOOL_PIN_OF: &[i8] = &[
    -1, -1, 0, 1, -1, -1, -1, 2, -1, 3, -1, -1, -1, -1, -1, -1, -1, 4, -1, -1, -1, -1, -1, 5, -1,
    6, -1, -1, -1,
];
const SCHOOL_PIN_AT: &[(i32, i32)] = &[
    (1654, 768),
    (964, 698),
    (1370, 122),
    (935, 207),
    (134, 206),
    (304, 616),
    (735, 520),
];

/// What differs between the pictures of the page.
#[derive(Debug)]
struct Spec {
    /// The click map.
    carea: &'static str,
    /// The file stem: `{stem}_bg.akb`, `{stem}_p.akb`, `{stem}plate_p.akb`.
    stem: &'static str,
    places: &'static [Place],
    pin_of: &'static [i8],
    pin_at: &'static [(i32, i32)],
    /// Source x of the back button's sheet cells.
    back_x: u32,
    /// Pin 1 is the academy's four slots (town only).
    academy_pin: bool,
}

static TOWN: Spec = Spec {
    carea: "SAKIMA_SUB.CAREA",
    stem: "sakima",
    places: TOWN_PLACES,
    pin_of: TOWN_PIN_OF,
    pin_at: TOWN_PIN_AT,
    back_x: 2377,
    academy_pin: true,
};

static YABUKI: Spec = Spec {
    carea: "YABUKI_SUB.CAREA",
    stem: "yabuki",
    places: YABUKI_PLACES,
    pin_of: YABUKI_PIN_OF,
    pin_at: YABUKI_PIN_AT,
    back_x: 2502,
    academy_pin: false,
};

static SCHOOL: Spec = Spec {
    carea: "SCHOOL_SUB.CAREA",
    stem: "school",
    places: SCHOOL_PLACES,
    pin_of: SCHOOL_PIN_OF,
    pin_at: SCHOOL_PIN_AT,
    back_x: 2491,
    academy_pin: false,
};

/// The picture for a byte 7089 value, or none (DEN and other values).
fn spec(mode: u8) -> Option<&'static Spec> {
    match mode {
        0 => Some(&TOWN),
        1 => Some(&YABUKI),
        2..=5 => Some(&SCHOOL),
        _ => None,
    }
}

/// The map the faces are read for. The school's script passes 2 on the
/// 1F and 3 on the other floors (the executable does so, see `who.rs`).
fn face_map(mode: u8) -> Map {
    match mode {
        0 => Map::Town,
        1 => Map::Yabuki,
        _ => Map::School(if mode == 2 { 2 } else { 3 }),
    }
}

fn mode_of(bytes: &[u8]) -> u8 {
    bytes.get(MODE_AT).copied().unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct Jump {
    spec: &'static Spec,
    map: Arc<Vec<u8>>,
    open: Vec<bool>,
    /// Faces of the places offered: (cell, x, y).
    faces: Vec<(u8, i32, i32)>,
    /// The easy mode is set: the faces show.
    easy: bool,
    /// Reiko's house art (town only): 0 or 1, none if not found.
    reiko: Option<usize>,
}

impl Jump {
    /// The page for the picture byte 7089 names; faces show in the easy
    /// mode (`easy`).
    pub fn load(assets: &Assets, vm: &Vm, easy: bool) -> Result<Self> {
        let mode = mode_of(&vm.bytes);
        let Some(spec) = spec(mode) else {
            return Err(Error::Invalid(format!("no map jump page for mode {mode}")));
        };
        let mut map = assets.data(spec.carea)?;
        map.resize(W * H, 255);
        let mut j = Self::new(spec, map, &vm.bytes, mode);
        let who_map = face_map(mode);
        j.faces = j.pins(|place| face_at(vm, who_map, place));
        j.easy = easy;
        Ok(j)
    }

    /// The game mode changed while the page is up (from its settings
    /// button); returns whether the page needs drawing again.
    pub fn set_easy(&mut self, easy: bool) -> bool {
        let changed = self.easy != easy && !self.faces.is_empty();
        self.easy = easy;
        changed
    }

    /// Whether the faces show.
    pub fn faces_shown(&self) -> bool {
        self.easy && !self.faces.is_empty()
    }

    /// The faces of the places offered; `who` gives the face at a pin.
    fn pins(&self, who: impl Fn(u8) -> Option<u8>) -> Vec<(u8, i32, i32)> {
        let mut faces = Vec::new();
        for (id, &pin) in self.spec.pin_of.iter().enumerate() {
            if pin < 0 || !self.offered(id) {
                continue;
            }
            let found = if self.spec.academy_pin && pin == 1 {
                (0..4).find_map(|k: u8| who(k + 1).map(|f| (f, SLOT_AT[k as usize])))
            } else {
                who(pin as u8).map(|f| (f, self.spec.pin_at[pin as usize]))
            };
            faces.extend(found.map(|(f, (x, y))| (f, x, y)));
        }
        faces
    }

    fn new(spec: &'static Spec, map: Vec<u8>, bytes: &[u8], mode: u8) -> Self {
        let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
        let mut open: Vec<bool> = spec
            .places
            .iter()
            .map(|place| match place.flag {
                Flag::Always => true,
                Flag::Byte(i) => byte(i) != 0,
                Flag::Never => false,
                Flag::Hidden3 => mode != 3,
            })
            .collect();
        // Reiko's house: the first art once found, the second later.
        let reiko = match (mode, byte(1702), byte(1692)) {
            (0, 1, _) => Some(1),
            (0, 0, 1) => Some(0),
            _ => None,
        };
        if mode == 0 {
            open[REIKO] = reiko.is_some();
        }
        Self {
            spec,
            map: Arc::new(map),
            open,
            faces: Vec::new(),
            easy: false,
            reiko,
        }
    }

    /// Whether the page is done for the map the script is on: the
    /// pictures of byte 7089 0 to 5; DEN (6) has none.
    pub fn available(bytes: &[u8]) -> bool {
        spec(mode_of(bytes)).is_some()
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
        (i == BACK || self.offered(i as usize)).then_some(i)
    }

    pub fn click(&mut self, id: i32) -> Click {
        match id {
            BACK => Click::Resume,
            _ if id >= 0 && self.offered(id as usize) => Click::Result(RESULT | (id + 1)),
            _ => Click::None,
        }
    }

    fn place(&self, id: usize) -> (u32, u32, i32, i32, u32, u32) {
        let t = &self.spec.places[id];
        let sx = match self.reiko {
            Some(art) if id == REIKO => REIKO_X[art],
            _ => t.src.0,
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
        let stem = self.spec.stem;
        let bg = assets.localized_image(&format!("{stem}_bg.akb"), language)?;
        let parts = assets.localized_image(&format!("{stem}_p.akb"), language)?;
        let plates = assets.localized_image(&format!("{stem}plate_p.akb"), language)?;
        canvas.draw_part(&bg, (0, 0, W as u32, H as u32), 0, 0, false, 255);
        let mut label = None;
        for id in (0..self.spec.places.len()).filter(|&i| self.offered(i)) {
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
        if self.easy && !self.faces.is_empty() {
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
        canvas.draw_part(
            &parts,
            (self.spec.back_x, BACK_SIZE * row, BACK_SIZE, BACK_SIZE),
            BACK_AT.0,
            BACK_AT.1,
            false,
            255,
        );
        Ok(())
    }
}

/// The face cell sheet: 144-pixel cells, four to a row.
pub(super) const FACES: &str = "map_p.akb";
pub(super) const FACE: u32 = 144;

#[cfg(test)]
mod tests {
    use super::*;

    fn jump(bytes: &[u8], mode: u8) -> Jump {
        let mut map = vec![255u8; W * H];
        map[0] = 2;
        map[1] = 21;
        map[2] = 100;
        Jump::new(spec(mode).unwrap(), map, bytes, mode)
    }

    fn with_mode(mode: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; 8000];
        bytes[MODE_AT] = mode;
        bytes
    }

    #[test]
    fn offered_places() {
        let mut bytes = vec![0u8; 2000];
        let j = jump(&bytes, 0);
        assert!(j.offered(0) && j.offered(25) && j.offered(26));
        assert!(!j.offered(18) && !j.offered(19) && !j.offered(33));
        bytes[1701] = 1;
        assert!(jump(&bytes, 0).offered(18));
        bytes[1692] = 1;
        let j = jump(&bytes, 0);
        assert!(j.offered(19) && j.reiko == Some(0));
        bytes[1702] = 1;
        assert_eq!(jump(&bytes, 0).reiko, Some(1));
    }

    #[test]
    fn offered_by_mode() {
        let none = vec![0u8; 2000];
        // Yabuki: five places always, the rest by their byte.
        let y = jump(&none, 1);
        assert_eq!((0..10).filter(|&i| y.offered(i)).count(), 5);
        let mut bytes = none.clone();
        bytes[1715] = 1;
        bytes[1719] = 1;
        let y = jump(&bytes, 1);
        assert!(y.offered(5) && !y.offered(6) && y.offered(9));
        // The school: all 29 on the 1F, 2F-only hides the four stairs.
        for mode in [2, 4, 5] {
            let j = jump(&none, mode);
            assert!((0..29).all(|i| j.offered(i)), "mode {mode}");
        }
        let j = jump(&none, 3);
        let hidden: Vec<usize> = (0..29).filter(|&i| !j.offered(i)).collect();
        assert_eq!(hidden, vec![1, 14, 15, 22]);
        // Reiko's house is the school's class 3-A: not a town art there.
        assert!(jump(&none, 4).offered(REIKO) && jump(&none, 4).reiko.is_none());
    }

    #[test]
    fn availability() {
        for mode in 0..=5 {
            assert!(Jump::available(&with_mode(mode)), "mode {mode}");
        }
        assert!(!Jump::available(&with_mode(6)));
        assert!(!Jump::available(&with_mode(7)));
    }

    #[test]
    fn results() {
        let none = vec![0u8; 2000];
        let mut y = jump(&none, 1);
        assert!(matches!(y.click(0), Click::Result(0xff01)));
        assert!(matches!(y.click(4), Click::Result(0xff05)));
        // Cat's Eye is not found yet.
        assert!(matches!(y.click(5), Click::None));
        assert!(matches!(y.click(BACK), Click::Resume));
        let mut s = jump(&none, 4);
        assert!(matches!(s.click(28), Click::Result(0xff1d)));
        assert!(matches!(s.click(-1), Click::None));
        assert!(matches!(jump(&none, 3).click(1), Click::None));
        assert!(matches!(jump(&none, 3).click(2), Click::Result(0xff03)));
    }

    #[test]
    fn hits() {
        let none = vec![0u8; 2000];
        // The map is the same for every picture: pixel 0 is id 2, pixel 1
        // id 21, pixel 2 the back button.
        let y = jump(&none, 1);
        assert_eq!(y.hit(0, 0), Some(2));
        assert_eq!(y.hit(1, 0), None);
        assert_eq!(y.hit(2, 0), Some(BACK));
        assert_eq!(y.hit(3, 0), None);
    }

    #[test]
    fn table_rows() {
        let none = vec![0u8; 2000];
        assert_eq!(jump(&none, 4).place(17), (1839, 1800, 17, 248, 332, 309));
        assert_eq!(jump(&none, 1).place(9), (1751, 0, 1187, 226, 397, 306));
        assert_eq!(jump(&none, 2).place(0), (1851, 0, 1202, 864, 320, 167));
        assert_eq!(jump(&none, 1).spec.back_x, 2502);
        assert_eq!(jump(&none, 5).spec.back_x, 2491);
        assert_eq!(jump(&none, 0).spec.back_x, 2377);
    }

    #[test]
    fn pins() {
        let mut bytes = vec![0u8; 2000];
        bytes[1701] = 1;
        let j = jump(&bytes, 0);
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
    fn yabuki_pins() {
        let mut bytes = vec![0u8; 2000];
        bytes[1715] = 1;
        let j = jump(&bytes, 1);
        // Places 0, 1, 2, 4 and 5 (Cat's Eye) have faces; 6 to 9 are not found.
        let faces = j.pins(|p| Some(p));
        assert_eq!(faces.len(), 5);
        assert!(faces.contains(&(0, 939, 621)));
        assert!(faces.contains(&(6, 991, 876)));
        assert!(faces.contains(&(5, 328, 378)));
    }

    #[test]
    fn school_pins() {
        let j = jump(&vec![0u8; 2000], 4);
        let faces = j.pins(|p| Some(p));
        // Pins 0..=6 of the seven places (2, 3, 7, 9, 17, 23 and 25).
        assert_eq!(faces.len(), 7);
        assert!(faces.contains(&(0, 1654, 768)));
        assert!(faces.contains(&(6, 735, 520)));
    }

    #[test]
    fn faces_follow_the_game_mode() {
        let mut j = jump(&vec![0u8; 2000], 4);
        j.faces = j.pins(|p| Some(p));
        assert!(!j.faces.is_empty());
        // The page of a classic game shows none, and takes them on when the
        // mode is changed from its settings button.
        assert!(!j.faces_shown());
        assert!(j.set_easy(true));
        assert!(j.faces_shown());
        assert!(!j.set_easy(true));
        assert!(j.set_easy(false));
        assert!(!j.faces_shown());
        // Without faces to show nothing needs drawing again.
        let mut empty = jump(&vec![0u8; 2000], 4);
        assert!(!empty.set_easy(true));
    }

    #[test]
    fn face_maps() {
        assert_eq!(face_map(0), Map::Town);
        assert_eq!(face_map(1), Map::Yabuki);
        assert_eq!(face_map(2), Map::School(2));
        assert_eq!(face_map(3), Map::School(3));
        assert_eq!(face_map(4), Map::School(3));
        assert_eq!(face_map(5), Map::School(3));
    }
}
