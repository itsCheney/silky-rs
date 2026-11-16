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
    /// The HP gauges (1015).
    pub battle: Option<Battle>,
    /// The monitor of the DNA PC over its screen (1014).
    pub monitor: bool,
    /// The Apeiria system window on that screen (1014).
    pub system: bool,
    /// The list of conclusions drawn into this background (992): cells
    /// of [`CONCLUSIONS_SHEET`], top first.
    pub conclusions: Option<(String, Vec<u8>)>,
    /// A mail header and its text drawn into this background (992 sub
    /// 1): background, cell, text, page.
    pub note: Option<(String, i32, String, i32)>,
}

/// The mail drawn into the background (992 sub 1): a 303 x 88 header
/// cell of [`CONCLUSIONS_SHEET`] (from x = 500, eight a column) at
/// (655, 73), and the text in a 300 x 272 window at (641, 161), pages of
/// 272 pixels. The text is black 14-pixel characters (7 for a narrow
/// one) from (20, 20), lines 16 apart, wrapped at x = 296.
pub const NOTE_CELL: (u32, u32, u32) = (500, 303, 88);
pub const NOTE_CELL_AT: (i32, i32) = (655, 73);
pub const NOTE_AT: (i32, i32) = (641, 161);
pub const NOTE_VIEW: (i32, i32) = (300, 272);

/// The conclusions list (992): 250 x 51 cells of this sheet, column
/// `v / 100` and row `v % 100` for a value `v` of bytes 1300 to 1306
/// (as many as byte 1320 says, the last first), drawn from (356, 103)
/// down. A value under 100 gets 100 added once drawn, so the next list
/// shows it in the second column.
pub const CONCLUSIONS_SHEET: &str = "dnapcA_mailparts.akb";
pub const CONCLUSIONS_AT: (i32, i32) = (356, 103);
pub const CONCLUSION_CELL: (u32, u32) = (250, 51);
pub const CONCLUSIONS_BYTES: usize = 1300;
pub const CONCLUSIONS_COUNT: usize = 1320;

/// The DNA PC's screen, drawn into the background layer, and the monitor
/// laid over it.
pub const MONITOR_SCREEN: &str = "dnapc_alphaB_bg.akb";
pub const MONITOR: &str = "dnapc_alphaA_bg.akb";
/// The system window: pieces of `apeirasystem.akb` (source left, top,
/// width, height, then the place in the window), the window at (305, 37).
pub const SYSTEM_SHEET: &str = "apeirasystem.akb";
pub const SYSTEM_AT: (i32, i32) = (305, 37);
pub const SYSTEM_PARTS: [((u32, u32, u32, u32), (i32, i32)); 3] = [
    ((0, 430, 670, 430), (0, 0)),
    ((1180, 430, 75, 87), (301, 281)),
    ((670, 760, 360, 16), (268, 368)),
];

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

/// Top left of each copy by count (tables of 256 bytes
/// for each of 1, 2, 4, 8, 16 and 32 copies; any other count takes the
/// table of 32).
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

/// The HP gauges of the battles (`Game.1015`):
/// Reiichi's at the top, then Apeiria, Miu, Mashiro and Kuon, cut from
/// `hp.akb`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Battle {
    /// HP of each gauge, out of 100.
    pub hp: [i32; 5],
    /// Which gauges have slid in.
    pub shown: [bool; 5],
    /// Steps queued with sub 4 for sub 9: (kind, gauge, value); kind 0
    /// slides a gauge in, 1 out, 2 moves its HP to the value, 3 waits.
    pub queue: Vec<(i32, i32, i32)>,
    /// HP changing over time (sub 6), until done or stopped (sub 7).
    pub drains: Vec<Drain>,
}

/// One HP change over time (`Game.1015` sub 6), read from the
/// executable: every `interval` milliseconds the gauge's HP moves by
/// `delta` (kept between 1 and 100), `count` times; it ends early once
/// the HP is 1.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Drain {
    pub id: i32,
    pub gauge: usize,
    pub delta: i32,
    pub count: i32,
    pub interval: u64,
    pub done: i32,
    /// Clock time of the next step; set at the first frame.
    pub next: Option<u64>,
}

impl Default for Battle {
    fn default() -> Self {
        Battle {
            hp: [100; 5],
            shown: [false; 5],
            queue: Vec::new(),
            drains: Vec::new(),
        }
    }
}

/// A gauge's place in `hp.akb` and on screen.
pub struct Gauge {
    /// Frame: source top left and size; the screen top left once slid in.
    pub frame: (u32, u32, u32, u32),
    pub at: (i32, i32),
    /// Bar: source top left (the empty bar lies under it) and size, and
    /// its top left in the frame.
    pub bar: (u32, u32, u32, u32),
    pub bar_at: (i32, i32),
    /// Where the skull goes in the frame at 0 HP.
    pub skull_at: (i32, i32),
}

/// The skull at 0 HP: 33 x 36 at (254, 53) of `hp.akb`.
pub const SKULL: (u32, u32, u32, u32) = (254, 53, 33, 36);

impl Battle {
    /// Runs the HP changes due by clock time `now`; true when an HP
    /// changed.
    pub fn advance(&mut self, now: u64) -> bool {
        let mut changed = false;
        let hp = &mut self.hp;
        self.drains.retain_mut(|d| {
            let next = *d.next.get_or_insert(now + d.interval);
            if now < next {
                return true;
            }
            let h = &mut hp[d.gauge.min(4)];
            *h = (*h + d.delta).clamp(1, 100);
            changed = true;
            if *h != 1 && d.done < d.count {
                d.done += 1;
                d.next = Some(now + d.interval);
                true
            } else {
                false
            }
        });
        changed
    }

    /// Gauge `i` (0 to 4).
    pub fn gauge(i: usize) -> Gauge {
        if i == 0 {
            Gauge {
                frame: (0, 0, 416, 53),
                at: (8, 8),
                bar: (0, 217, 297, 34),
                bar_at: (88, 8),
                skull_at: (220, 7),
            }
        } else {
            let i = i as u32;
            Gauge {
                frame: (0, 41 * i + 11, 254, 41),
                at: (8, 41 * i as i32 + 20),
                bar: (0, 285, 160, 22),
                bar_at: (88, 10),
                skull_at: (152, 3),
            }
        }
    }

    /// The colour set's column in `hp.akb` for `hp`: green from
    /// half, orange from a tenth, red below.
    pub fn column(hp: i32) -> u32 {
        let f = hp as f32 / 100.0;
        if f >= 0.5 {
            0
        } else if f >= 0.1 {
            416
        } else {
            832
        }
    }

    /// Width of the coloured bar out of `width` for `hp`; at least 1 while
    /// any is left.
    pub fn filled(width: u32, hp: i32) -> u32 {
        let f = hp.max(0) as f32 / 100.0;
        let w = (((width as f32 - 7.0) * f * 10.0 + 5.0) / 10.0) as i32;
        if hp == 0 {
            0
        } else {
            w.max(1) as u32
        }
    }

    /// Carries out the queued steps (sub 9).
    pub fn run(&mut self) {
        for (kind, i, v) in std::mem::take(&mut self.queue) {
            let Some(i) = usize::try_from(i).ok().filter(|i| *i < 5) else {
                continue;
            };
            match kind {
                0 => self.shown[i] = true,
                1 => self.shown[i] = false,
                2 => self.hp[i] = v,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod drain_tests {
    use super::*;

    #[test]
    fn drains_until_one() {
        let mut b = Battle::default();
        b.hp[0] = 3;
        b.drains.push(Drain {
            gauge: 0,
            delta: -1,
            count: 99,
            interval: 500,
            ..Drain::default()
        });
        assert!(!b.advance(0));
        assert!(b.advance(500));
        assert_eq!(b.hp[0], 2);
        assert!(!b.advance(900));
        assert!(b.advance(1000));
        assert_eq!(b.hp[0], 1);
        assert!(b.drains.is_empty());
    }
}
