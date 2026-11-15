//! Pictures 景の海のアペイリア lays over the scene with its own commands,
//! read from the executable: copies of one picture at fixed spots
//! (`Game.995`), the deduction board (994), the notes and explanation
//! screens (996) and the item panel (997). Their fades and slides are not
//! done; they appear and go at once.

use std::sync::Arc;

use silky_format::akb::Image;

/// Kept with the scene so saves keep it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Panels {
    /// Copies of one picture (995).
    pub swarm: Option<Swarm>,
    /// Picture on the deduction board (994).
    pub board: Option<String>,
    /// Page of the notes panel, `k01_0N.akb` (996 subs 0 to 3).
    pub notes: Option<i32>,
    /// Picture of the explanation screen, over `kbg.akb` (996 subs 10 to
    /// 19).
    pub explain: Option<String>,
    /// Picture of the item panel (997).
    pub item: Option<String>,
}

/// Copies of a 512 x 512 picture, each at its spot for the count, scaled
/// about its centre.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Swarm {
    pub picture: String,
    /// Size in percent.
    pub size: i32,
    /// Opacity of each copy once shown.
    pub alpha: u8,
    /// Which copies are shown.
    pub shown: Vec<bool>,
    /// Sub 4 turns the first copy into `ch08_02_03.akb`.
    pub first: Option<String>,
}

/// The deduction board: 1016 x 424 at (132, 15).
pub const BOARD_AT: (i32, i32) = (132, 15);
/// The notes panel: 958 x 538 at (145, 0).
pub const NOTES_AT: (i32, i32) = (145, 0);
/// The explanation screen's frame, drawn under its picture.
pub const EXPLAIN_FRAME: &str = "kbg.akb";
/// The item panel's bottom edge; it sits at the left edge.
pub const ITEM_BOTTOM: i32 = 495;

/// Top left of each copy by count (tables at 0x574c60, one of 256 bytes
/// for each of 1, 2, 4, 8, 16 and 32 copies; any other count takes the
/// table of 32, by the byte map at 0x4891b0).
const SPOTS_1: [(i32, i32); 1] = [(384, 104)];
const SPOTS_2: [(i32, i32); 2] = [(128, 104), (640, 104)];
const SPOTS_4: [(i32, i32); 4] = [(551, 104), (219, 104), (0, -66), (768, -66)];
const SPOTS_8: [(i32, i32); 8] = [
    (551, 104),
    (219, 104),
    (0, -66),
    (768, -66),
    (531, -116),
    (239, -66),
    (0, 124),
    (768, 124),
];
const SPOTS_16: [(i32, i32); 16] = [
    (521, 104),
    (249, 104),
    (40, -116),
    (728, -136),
    (501, -116),
    (269, -116),
    (0, 124),
    (768, 124),
    (581, 284),
    (149, 284),
    (-170, -116),
    (938, -126),
    (640, -6),
    (389, 4),
    (-150, 254),
    (928, 254),
];
const SPOTS_32: [(i32, i32); 32] = [
    (552, -37),
    (113, 2),
    (40, -196),
    (728, -216),
    (501, -196),
    (269, -196),
    (-50, -6),
    (747, -43),
    (390, -38),
    (-33, 153),
    (-170, -196),
    (938, -206),
    (946, -23),
    (-211, -13),
    (-220, 174),
    (974, 151),
    (825, 102),
    (277, 78),
    (341, 257),
    (531, 119),
    (701, 104),
    (640, 332),
    (529, 436),
    (157, 157),
    (-210, -126),
    (978, 343),
    (833, 275),
    (814, 436),
    (94, 311),
    (-40, 384),
    (372, 417),
    (183, 446),
];

/// The spots for `count` copies.
pub fn spots(count: usize) -> &'static [(i32, i32)] {
    match count {
        1 => &SPOTS_1,
        2 => &SPOTS_2,
        4 => &SPOTS_4,
        8 => &SPOTS_8,
        16 => &SPOTS_16,
        _ => &SPOTS_32,
    }
}

impl Swarm {
    /// `count` hidden copies of `picture`.
    pub fn new(picture: String, count: i32, size: i32, alpha: i32) -> Self {
        Swarm {
            picture,
            size,
            alpha: alpha.clamp(0, 255) as u8,
            shown: vec![false; count.clamp(0, 32) as usize],
            first: None,
        }
    }

    /// Shows or hides copies `from` to `to` (exclusive); -1 means from the
    /// first or to the last.
    pub fn set(&mut self, from: i32, to: i32, shown: bool) {
        let n = self.shown.len() as i32;
        let from = if from < 0 { 0 } else { from.min(n) };
        let to = if to < 0 { n } else { to.min(n) };
        for s in &mut self.shown[from as usize..to.max(from) as usize] {
            *s = shown;
        }
    }

    /// Size as a factor.
    pub fn factor(&self) -> f32 {
        self.size.max(0) as f32 / 100.0
    }
}

/// `img` with its alpha scaled by `alpha`.
pub fn faded(img: &Image, alpha: u8) -> Arc<Image> {
    let mut rgba = img.rgba.clone();
    for px in rgba.chunks_exact_mut(4) {
        px[3] = (px[3] as u32 * alpha as u32 / 255) as u8;
    }
    Arc::new(Image {
        width: img.width,
        height: img.height,
        rgba,
    })
}
