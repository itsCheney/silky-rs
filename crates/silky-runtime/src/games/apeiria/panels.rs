//! Pictures 景の海のアペイリア lays over the scene with its own commands,
//! read from the executable: copies of one picture at fixed spots
//! (`Game.995`), the deduction board (994), the notes and explanation
//! screens (996), the item panel (997), the DNA PC (1014) and the HP
//! gauges (1015). The copies, the explanation screen, the item panel,
//! the DNA PC's window and the gauges move as the executable has them;
//! the board and the notes appear and go at once.

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
    /// Its opacity while it fades out (sub 17).
    pub explain_fade: Option<Tween>,
    /// The next picture laid over it as it comes (subs 13, 14) or blinks
    /// (15, 19).
    pub explain_over: Option<(String, Tween)>,
    /// The screen drawn into a layer (sub 18), kept until the background
    /// changes.
    pub explain_kept: Option<String>,
    /// Picture of the item panel (997).
    pub item: Option<String>,
    /// The item panel opening or closing.
    pub item_anim: Option<ItemAnim>,
    /// The HP gauges (1015).
    pub battle: Option<Battle>,
    /// The monitor of the DNA PC over its screen (1014).
    pub monitor: bool,
    /// The Apeiria system window on that screen (1014).
    pub system: bool,
    /// The system window opening or closing.
    pub system_anim: Option<WindowAnim>,
    /// The list of conclusions drawn into this background (992): cells
    /// of [`CONCLUSIONS_SHEET`], top first.
    pub conclusions: Option<(String, Vec<u8>)>,
    /// A mail header and its text drawn into this background (992 sub
    /// 1): background, cell, text, page.
    pub note: Option<(String, i32, String, i32)>,
}

impl Panels {
    /// Steps the fades; whether anything on screen changed.
    pub fn advance(&mut self, ms: u64) -> bool {
        let mut moved = false;
        if let Some(s) = &mut self.swarm {
            moved |= s.advance(ms);
        }
        if let Some(t) = &mut self.explain_fade {
            moved |= t.advance(ms);
        }
        if let Some((_, t)) = &mut self.explain_over {
            moved |= t.advance(ms);
        }
        if let Some(a) = &mut self.item_anim {
            a.ms += ms;
            moved = true;
        }
        if let Some(a) = &mut self.system_anim {
            a.ms += ms;
            moved = true;
        }
        self.settle();
        moved
    }

    /// Ends the fades that the script waits for.
    pub fn finish(&mut self) {
        if let Some(s) = &mut self.swarm {
            s.end_change();
        }
        if let Some(t) = &mut self.explain_fade {
            t.ms = t.length_ms();
        }
        if let Some((_, t)) = self.explain_over.as_mut().filter(|(_, t)| !t.looped) {
            t.ms = t.length_ms();
        }
        if let Some(a) = &mut self.item_anim {
            a.ms = a.length_ms();
        }
        if let Some(a) = &mut self.system_anim {
            a.ms = a.length_ms();
        }
        if let Some(b) = &mut self.battle {
            b.finish();
        }
        self.settle();
    }

    /// A picture that came takes the screen's place; a screen that went
    /// is gone.
    fn settle(&mut self) {
        if let Some(a) = self.system_anim.filter(|a| a.done()) {
            if !a.opening {
                self.system = false;
            }
            self.system_anim = None;
        }
        if let Some(a) = self.item_anim.filter(|a| a.done()) {
            if !a.opening {
                self.item = None;
            }
            self.item_anim = None;
        }
        if self.explain_fade.as_ref().is_some_and(|t| t.done()) {
            self.explain = None;
            self.explain_fade = None;
        }
        let came = |(_, t): &(String, Tween)| !t.looped && t.stopped.is_none() && t.done();
        if self.explain_over.as_ref().is_some_and(came) {
            self.explain = self.explain_over.take().map(|(p, _)| p);
        }
    }

    /// The background changed: what was drawn into the old one goes.
    pub fn background_changed(&mut self) {
        self.explain_kept = None;
    }
}

/// Length of a frame of the engine's sprite animation.
pub const TICK_MS: u64 = super::anim::TICK_MS;

/// An opacity changing as the engine's keyframes change it: keys of
/// (wait, frames, from, to), each holding `from` through its wait and
/// then going linearly to `to`, played once or for ever.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tween {
    pub keys: Vec<(i32, i32, u8, u8)>,
    pub looped: bool,
    /// Time since the start.
    pub ms: u64,
    /// Stopped at this value.
    pub stopped: Option<u8>,
}

impl Tween {
    /// From `from` to `to` over `frames` frames (one at least).
    pub fn once(frames: i32, from: u8, to: u8) -> Self {
        Self {
            keys: vec![(0, frames.max(1), from, to)],
            ..Self::default()
        }
    }

    pub fn looping(keys: Vec<(i32, i32, u8, u8)>) -> Self {
        Self {
            keys,
            looped: true,
            ..Self::default()
        }
    }

    /// Frames of one round.
    fn length(&self) -> u64 {
        self.keys
            .iter()
            .map(|&(w, n, _, _)| (w.max(0) + n.max(1)) as u64)
            .sum()
    }

    pub fn length_ms(&self) -> u64 {
        self.length() * TICK_MS
    }

    pub fn value(&self) -> u8 {
        if let Some(v) = self.stopped {
            return v;
        }
        let len = self.length();
        let mut t = self.ms / TICK_MS;
        if len == 0 {
            return 255;
        }
        if self.looped {
            t %= len;
        } else if t >= len {
            return self.keys.last().map_or(255, |k| k.3);
        }
        for &(w, n, a, b) in &self.keys {
            let (w, n) = (w.max(0) as u64, n.max(1) as u64);
            if t < w + n {
                let f = t.saturating_sub(w) as i32;
                let (a, b) = (a as i32, b as i32);
                return ((b - a) * f / n as i32 + a) as u8;
            }
            t -= w + n;
        }
        255
    }

    pub fn done(&self) -> bool {
        self.stopped.is_some() || !self.looped && self.ms >= self.length_ms()
    }

    /// Steps `ms`; whether the value changed.
    pub fn advance(&mut self, ms: u64) -> bool {
        if self.done() {
            return false;
        }
        let before = self.value();
        self.ms += ms;
        self.value() != before || self.done()
    }

    /// Stops where it is.
    pub fn stop(&mut self) {
        self.stopped = Some(self.value());
    }
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
    /// Copies coming or going (subs 1 and 2).
    pub change: Option<SwarmChange>,
}

/// Copies `from` to `to` (exclusive) fading in from nothing to the
/// swarm's opacity, or out to nothing, while the script waits. With
/// `ripple` the first of them wavers row by row as it goes
/// (`CRasterAnime` modes 1 and 2): coming, from 256 pixels and 20
/// degrees a row down to nothing and 1 degree; going, the other way.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SwarmChange {
    pub from: usize,
    pub to: usize,
    pub coming: bool,
    pub ripple: bool,
    pub alpha: Tween,
}

impl SwarmChange {
    /// Swing and step (degrees a row) of the ripple, and its phase.
    pub fn ripple(&self) -> Option<(f32, f32, f32)> {
        if !self.ripple {
            return None;
        }
        let n = self.alpha.length().max(1) as f32;
        let f = (self.alpha.ms / TICK_MS).clamp(1, n as u64) as f32;
        Some(if self.coming {
            (256.0 * (1.0 - f / n), 20.0 - 19.0 * f / n, f)
        } else {
            (256.0 * f / n, 19.0 * f / n + 1.0, f)
        })
    }
}

/// The deduction board: 1016 x 424 at (132, 15).
pub const BOARD_AT: (i32, i32) = (132, 15);
/// The notes panel: 958 x 538 at (145, 0).
pub const NOTES_AT: (i32, i32) = (145, 0);
/// The explanation screen's frame, drawn under its picture.
pub const EXPLAIN_FRAME: &str = "kbg.akb";
/// The item panel's bottom edge; it sits at the left edge.
pub const ITEM_BOTTOM: i32 = 495;

/// The DNA PC's system window opening (its height growing from nothing)
/// or closing (going to nothing) about its middle over 16 frames, while
/// the script waits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WindowAnim {
    pub opening: bool,
    /// Time since the start.
    pub ms: u64,
}

impl WindowAnim {
    pub const FRAMES: u64 = 16;

    pub fn new(opening: bool) -> Self {
        Self { opening, ms: 0 }
    }

    pub fn length_ms(&self) -> u64 {
        Self::FRAMES * TICK_MS
    }

    pub fn done(&self) -> bool {
        self.ms >= self.length_ms()
    }

    /// Height factor now.
    pub fn height(&self) -> f32 {
        let f = (self.ms / TICK_MS).min(Self::FRAMES) as f32 / Self::FRAMES as f32;
        if self.opening {
            f
        } else {
            1.0 - f
        }
    }
}

/// Middle of the system window, about which it opens and closes.
pub const SYSTEM_MIDDLE: f32 = (SYSTEM_AT.1 + 215) as f32;

/// The item panel opening like a screen switched on, about its middle:
/// a line 1 % high grows to its width over 8 frames, then to its height
/// over 4; or closing, its height going to nothing over 12. The script
/// waits for either.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ItemAnim {
    pub opening: bool,
    /// Time since the start.
    pub ms: u64,
}

impl ItemAnim {
    pub const FRAMES: u64 = 12;

    pub fn new(opening: bool) -> Self {
        Self { opening, ms: 0 }
    }

    pub fn length_ms(&self) -> u64 {
        Self::FRAMES * TICK_MS
    }

    pub fn done(&self) -> bool {
        self.ms >= self.length_ms()
    }

    /// Width and height factors now.
    pub fn scale(&self) -> (f32, f32) {
        let f = (self.ms / TICK_MS).min(Self::FRAMES) as f32;
        match self.opening {
            true if f < 8.0 => (f / 8.0, 0.01 * f / 8.0),
            true => (1.0, 0.01 + 0.99 * (f - 8.0) / 4.0),
            false => (1.0, 1.0 - f / 12.0),
        }
    }
}

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
            change: None,
        }
    }

    /// Starts copies `from` to `to` coming (or going) over `frames`
    /// frames; the time the script waits.
    pub fn change(&mut self, from: i32, to: i32, coming: bool, frames: i32, ripple: bool) -> u64 {
        self.end_change();
        let (from, to) = self.range(from, to);
        let alpha = if coming {
            Tween::once(frames, 0, self.alpha)
        } else {
            Tween::once(frames, self.alpha, 0)
        };
        let ms = alpha.length_ms();
        self.change = Some(SwarmChange {
            from,
            to,
            coming,
            ripple,
            alpha,
        });
        ms
    }

    /// Ends a change at once.
    pub fn end_change(&mut self) {
        if let Some(c) = self.change.take() {
            for s in &mut self.shown[c.from..c.to] {
                *s = c.coming;
            }
        }
    }

    /// Steps the change; whether anything moved.
    pub fn advance(&mut self, ms: u64) -> bool {
        let Some(c) = &mut self.change else {
            return false;
        };
        let moved = c.alpha.advance(ms) || c.ripple;
        if c.alpha.done() {
            self.end_change();
        }
        moved
    }

    fn range(&self, from: i32, to: i32) -> (usize, usize) {
        let n = self.shown.len() as i32;
        let from = if from < 0 { 0 } else { from.min(n) };
        let to = if to < 0 { n } else { to.min(n) };
        (from as usize, to.max(from) as usize)
    }

    /// Shows or hides copies `from` to `to` (exclusive); -1 means from the
    /// first or to the last.
    pub fn set(&mut self, from: i32, to: i32, shown: bool) {
        let (from, to) = self.range(from, to);
        for s in &mut self.shown[from..to] {
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
    /// Gauges sliding: their left edge going from one x to another.
    pub slides: [Option<Slide>; 5],
    /// HP going from one value to another (sub 3).
    pub hp_moves: [Option<Slide>; 5],
    /// Frame of the clock last stepped.
    pub frame: Option<u64>,
}

/// A value going linearly from `from` to `to` over `frames` frames, one
/// step a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Slide {
    pub from: i32,
    pub to: i32,
    pub frames: i32,
    /// Frames stepped.
    pub done: i32,
}

impl Slide {
    pub fn new(from: i32, to: i32, frames: i32) -> Self {
        Self {
            from,
            to,
            frames: frames.max(1),
            done: 0,
        }
    }

    pub fn value(&self) -> i32 {
        (self.to - self.from) * self.done.min(self.frames) / self.frames + self.from
    }

    pub fn over(&self) -> bool {
        self.done > self.frames
    }
}

/// Left edge of a gauge slid in, and out of view.
pub const GAUGE_IN: i32 = 8;
pub const GAUGE_OUT: i32 = -424;
/// Frames of a slide (subs 1 and 2) and of an HP move (sub 3).
pub const SLIDE_FRAMES: i32 = 16;
pub const HP_FRAMES: i32 = 128;

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
            slides: [None; 5],
            hp_moves: [None; 5],
            frame: None,
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

    /// Left edge of gauge `i` now, if it is in view.
    pub fn x(&self, i: usize) -> Option<i32> {
        match (self.slides[i], self.shown[i]) {
            (Some(s), _) => Some(s.value()),
            (None, true) => Some(GAUGE_IN),
            (None, false) => None,
        }
    }

    /// Slides gauge `i` in or out over [`SLIDE_FRAMES`] frames from where
    /// it is; a slide in drops the queued slides out of it, a slide out
    /// the whole queue.
    pub fn slide(&mut self, i: usize, coming: bool) {
        let from = self.x(i).unwrap_or(GAUGE_OUT);
        let to = if coming { GAUGE_IN } else { GAUGE_OUT };
        if coming {
            self.queue.retain(|&(k, g, _)| !(k == 1 && g == i as i32));
        } else {
            self.queue.clear();
        }
        self.slides[i] = Some(Slide::new(from, to, SLIDE_FRAMES));
        self.shown[i] = true;
    }

    /// Moves gauge `i`'s HP to `hp` over [`HP_FRAMES`] frames, dropping
    /// the queued HP moves of it.
    pub fn move_hp(&mut self, i: usize, hp: i32) {
        self.queue.retain(|&(k, g, _)| !(k == 2 && g == i as i32));
        self.hp_moves[i] = Some(Slide::new(self.hp[i], hp, HP_FRAMES));
    }

    /// Steps the slides and HP moves once a frame up to clock time `now`;
    /// true when anything moved.
    pub fn step(&mut self, now: u64) -> bool {
        let frame = now / TICK_MS;
        let last = *self.frame.get_or_insert(frame);
        self.frame = Some(frame);
        let mut moved = false;
        for _ in last..frame {
            if self
                .slides
                .iter()
                .chain(&self.hp_moves)
                .all(Option::is_none)
            {
                break;
            }
            for i in 0..5 {
                if let Some(s) = &mut self.slides[i] {
                    s.done += 1;
                    moved = true;
                    if s.over() {
                        self.shown[i] = s.to == GAUGE_IN;
                        self.slides[i] = None;
                    }
                }
                if let Some(m) = &mut self.hp_moves[i] {
                    m.done += 1;
                    self.hp[i] = m.value();
                    moved = true;
                    if m.over() {
                        self.hp_moves[i] = None;
                    }
                }
            }
        }
        moved
    }

    /// Ends the slides and HP moves at once.
    pub fn finish(&mut self) {
        for i in 0..5 {
            if let Some(s) = self.slides[i].take() {
                self.shown[i] = s.to == GAUGE_IN;
            }
            if let Some(m) = self.hp_moves[i].take() {
                self.hp[i] = m.to;
            }
        }
    }

    /// Carries out the queued steps (sub 9).
    pub fn run(&mut self) {
        if !self.queue.is_empty() {
            self.finish();
        }
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

    #[test]
    fn a_tween_holds_through_its_wait_then_loops() {
        let mut t = Tween::looping(vec![(0, 1, 255, 255), (2, 4, 255, 0)]);
        let mut seen = vec![t.value()];
        for _ in 0..8 {
            t.advance(TICK_MS);
            seen.push(t.value());
        }
        assert_eq!(seen, [255, 255, 255, 255, 192, 128, 64, 255, 255]);
        t.stop();
        assert!(!t.advance(TICK_MS * 3));
        assert_eq!(t.value(), 255);
    }

    #[test]
    fn copies_fade_in_and_are_shown_at_the_end() {
        let mut s = Swarm::new("x.akb".into(), 4, 100, 128);
        let ms = s.change(-1, 2, true, 10, true);
        assert_eq!(ms, 10 * TICK_MS);
        let c = s.change.as_ref().unwrap();
        assert_eq!((c.from, c.to, c.alpha.value()), (0, 2, 0));
        assert_eq!(c.ripple(), Some((256.0 * 0.9, 18.1, 1.0)));
        s.advance(5 * TICK_MS);
        assert_eq!(s.change.as_ref().unwrap().alpha.value(), 64);
        s.advance(5 * TICK_MS);
        assert!(s.change.is_none());
        assert_eq!(s.shown, [true, true, false, false]);
    }

    #[test]
    fn a_new_explanation_takes_the_old_ones_place() {
        let mut p = Panels {
            explain: Some("a.akb".into()),
            explain_over: Some(("b.akb".into(), Tween::once(12, 0, 255))),
            ..Panels::default()
        };
        p.advance(6 * TICK_MS);
        assert_eq!(p.explain.as_deref(), Some("a.akb"));
        p.finish();
        assert_eq!(p.explain.as_deref(), Some("b.akb"));
        assert!(p.explain_over.is_none());
    }
}
