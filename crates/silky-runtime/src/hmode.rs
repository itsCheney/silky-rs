//! きまぐれテンプテーション2's H mode (game command 49, read from the
//! executable: object vtable 0x542c60, init 0x4844b0, frame 0x484a50,
//! buttons 0x4852f0, actions 0x486b10, held actions 0x48a090, climax
//! 0x48ab90).
//!
//! The script loads Anneliese's `hg` model into standing slot 0 and calls
//! the command; the engine then runs its own screen until the finish
//! button. The player picks a tool (right column of `action_p.akb`) and
//! clicks a body region of `anne_h.carea` (a 1280×720 byte map, regions
//! 1 to 7). Each tool × region pair plays timelines on the model, a face,
//! a voice from tables in the executable, and raises her gauge (left,
//! 0..1000) and with insertion his gauge (bottom left). Held actions turn
//! into loops once their one-shot ends. A full gauge plays the climax and
//! waits for its voice. Buttons on `hmode.area`: tools (256..260), clothes
//! and horns (512..514), finish (768), dream mode (1024, the second model
//! file and `yume.akb` mist), inside/outside (1280, 1281), auto repeat
//! (1536), climax together (1792) and gauge locks (2048, 2049).
//!
//! What is not carried out: the mouse cursors per tool and region (the
//! frontends keep their own), and one model call of the right breast
//! rotor (vtable +0x140, meaning not read). The mist's scroll speed and
//! the white flash's length are guesses.

use std::sync::Arc;
use silky_format::akb::Image;
use silky_format::area::Area;

use crate::render::Canvas;

/// Voice files (without `.ogg`) of the tables at 0x58bc50..0x58ea44.
/// Tables of three are by her gauge: below 300, below 600, the rest.
pub mod voices {
    /// Breathing between actions by gauge band (48bbc0).
    pub const BREATH: [&[&str]; 3] = [&["k2a0265"], &["k2a0266"], &["k2a0267"]];
    /// Breathing after her first climax.
    pub const BREATH_AFTER: &[&str] = &["k2a0268"];
    /// Breathing in dream mode.
    pub const BREATH_YUME: &[&str] = &["k2a0269"];
    /// Breathing in dream mode after her first climax.
    pub const BREATH_YUME_AFTER: &[&str] = &["k2a0270"];
    /// Touches by gauge band.
    pub const TOUCH: [&[&str]; 3] = [
        &["k2a0271", "k2a0272", "k2a0273"],
        &["k2a0274", "k2a0275", "k2a0276"],
        &["k2a0277", "k2a0278", "k2a0279"],
    ];
    /// Touches in dream mode.
    pub const TOUCH_YUME: &[&str] = &["k2a0280", "k2a0281", "k2a0282"];
    /// While the finger action is held, by band.
    pub const FINGER_LOOP: [&[&str]; 3] = [&["k2a0283"], &["k2a0284"], &["k2a0285"]];
    pub const FINGER_LOOP_YUME: &[&str] = &["k2a0286"];
    /// While the mouth action is held, by band.
    pub const TONGUE_LOOP: [&[&str]; 3] = [&["k2a0287"], &["k2a0288"], &["k2a0289"]];
    pub const TONGUE_LOOP_YUME: &[&str] = &["k2a0290"];
    /// Kiss, by band.
    pub const KISS: [&[&str]; 3] = [&["k2a0291"], &["k2a0292"], &["k2a0293"]];
    pub const KISS_YUME: &[&str] = &["k2a0294"];
    /// While the insertion is held, by band.
    pub const PISTON_LOOP: [&[&str]; 3] = [
        &["k2a0299a", "k2a0299b"],
        &["k2a0300a", "k2a0300b", "k2a0300c"],
        &["k2a0301a", "k2a0301b", "k2a0301c", "k2a0301d"],
    ];
    pub const PISTON_LOOP_YUME: &[&str] = &["k2a0302a", "k2a0302b", "k2a0302c"];
    /// Rotor on a breast, by band.
    pub const NIPPLE_ROTOR: [&[&str]; 3] = [&["k2a0303"], &["k2a0304"], &["k2a0305"]];
    pub const NIPPLE_ROTOR_YUME: &[&str] = &["k2a0306"];
    /// Rotor below, by band.
    pub const ROTOR: [&[&str]; 3] = [&["k2a0307"], &["k2a0308"], &["k2a0309"]];
    pub const ROTOR_YUME: &[&str] = &["k2a0310"];
    /// Vibrator, by band; also its held loop.
    pub const VIBE: [&[&str]; 3] = [&["k2a0311"], &["k2a0312"], &["k2a0313"]];
    pub const VIBE_YUME: &[&str] = &["k2a0314"];
    /// Her climax.
    pub const CLIMAX: &[&str] = &["k2a0315", "k2a0316", "k2a0317"];
    pub const CLIMAX_YUME: &[&str] = &["k2a0318", "k2a0319", "k2a0320"];
    /// Both gauges full together.
    pub const CLIMAX_TOGETHER: &[&str] = &["k2a0315c", "k2a0316a", "k2a0317c"];
    pub const CLIMAX_TOGETHER_YUME: &[&str] = &["k2a0318c", "k2a0320a", "k2a0320c"];
    /// Finish button: neither climaxed.
    pub const FINISH_NONE: &[&str] = &["k2a0321", "k2a0322", "k2a0323"];
    /// Finish: three or more of each.
    pub const FINISH_BOTH: &[&str] = &["k2a0324", "k2a0325", "k2a0326"];
    /// Finish: she climaxed no more often than he did.
    pub const FINISH_HERS: &[&str] = &["k2a0327", "k2a0328", "k2a0329"];
    /// Finish: she climaxed more often.
    pub const FINISH_MORE: &[&str] = &["k2a0330", "k2a0331", "k2a0332"];
    /// One insertion stroke, by band.
    pub const PISTON: [&[&str]; 3] = [
        &[
            "k2a0295aa",
            "k2a0295ab",
            "k2a0295ac",
            "k2a0295ad",
            "k2a0295ae",
            "k2a0295ba",
            "k2a0295bb",
            "k2a0295bc",
            "k2a0295bd",
            "k2a0295be",
            "k2a0295bf",
            "k2a0295ca",
            "k2a0295cb",
            "k2a0295cc",
            "k2a0295cd",
            "k2a0295ce",
            "k2a0295cf",
            "k2a0295da",
            "k2a0295db",
            "k2a0295dc",
            "k2a0295dd",
            "k2a0295de",
            "k2a0295df",
            "k2a0295dg",
        ],
        &[
            "k2a0296aa",
            "k2a0296ab",
            "k2a0296ac",
            "k2a0296ad",
            "k2a0296ba",
            "k2a0296bb",
            "k2a0296bc",
            "k2a0296bd",
            "k2a0296be",
            "k2a0296bf",
            "k2a0296ca",
            "k2a0296cb",
            "k2a0296cc",
            "k2a0296cd",
            "k2a0296ce",
            "k2a0296da",
            "k2a0296db",
            "k2a0296dc",
            "k2a0296dd",
            "k2a0296ea",
            "k2a0296eb",
            "k2a0296ec",
            "k2a0296ed",
            "k2a0296ee",
            "k2a0296ef",
            "k2a0296eg",
        ],
        &[
            "k2a0297aa",
            "k2a0297ab",
            "k2a0297ac",
            "k2a0297ba",
            "k2a0297bb",
            "k2a0297bc",
            "k2a0297bd",
            "k2a0297ca",
            "k2a0297cb",
            "k2a0297cc",
            "k2a0297cd",
            "k2a0297ce",
            "k2a0297cf",
            "k2a0297da",
            "k2a0297db",
            "k2a0297dc",
            "k2a0297dd",
            "k2a0297de",
            "k2a0297ea",
            "k2a0297eb",
            "k2a0297ec",
        ],
    ];
    pub const PISTON_YUME: &[&str] = &[
        "k2a0298aa",
        "k2a0298ab",
        "k2a0298ac",
        "k2a0298ad",
        "k2a0298ae",
        "k2a0298af",
        "k2a0298ag",
        "k2a0298ah",
        "k2a0298ba",
        "k2a0298bb",
        "k2a0298bc",
        "k2a0298bd",
        "k2a0298be",
        "k2a0298bf",
        "k2a0298bg",
        "k2a0298bh",
        "k2a0298bi",
        "k2a0298bj",
        "k2a0298bk",
        "k2a0298bl",
        "k2a0298ca",
        "k2a0298cb",
        "k2a0298cc",
        "k2a0298cd",
        "k2a0298ce",
    ];
}

/// A model, sound or end request for the session to carry out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Main timeline on both files; `cut` stops the one before.
    Main {
        index: usize,
        file: usize,
        cut: bool,
    },
    StopMain,
    /// Difference timeline on both files.
    Diff {
        index: usize,
        file: usize,
    },
    StopDiff(usize),
    /// Face 0..13 (7..13 on the second file).
    Face(usize),
    /// Voice file, cutting the one playing.
    Voice(String),
    /// Sound effect file.
    Sound(&'static str),
}

pub mod ids {
    pub const TOOL: i32 = 256;
    pub const DRESSED: i32 = 512;
    pub const NAKED: i32 = 513;
    pub const HORNS: i32 = 514;
    pub const FINISH: i32 = 768;
    pub const DREAM: i32 = 1024;
    pub const INSIDE: i32 = 1280;
    pub const AUTO: i32 = 1536;
    pub const TOGETHER: i32 = 1792;
    pub const LOCK: i32 = 2048;
}

/// Byte variables the screen reads: rotor and vibrator tools, dream
/// mode, and the outfit (1 starts naked).
pub const ROTOR_FLAG: usize = 3010;
pub const DREAM_FLAG: usize = 3014;
pub const VIBE_FLAG: usize = 3015;
pub const OUTFIT_VAR: usize = 708;

/// Column of `action_p.akb` for each tool: insertion, hand, mouth,
/// rotor, vibrator.
const TOOL_COLUMN: [u32; 5] = [4, 0, 2, 3, 5];
/// Body region below (the only one with insertion and toys).
const CROTCH: usize = 5;
const FULL: i32 = 1000;
const IDLE_MS: u64 = 7500;
const RISE_MS: u64 = 500;
/// Body clicks closer than this are ignored.
const CLICK_GAP_MS: u64 = 1000;
const FLASH_MS: u64 = 550;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Inserted {
    Penis,
    Vibe,
}

/// What a point of the screen is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Button(i32),
    Region(usize),
}

pub struct HMode {
    areas: Vec<Area>,
    carea: Vec<u8>,
    flags: [bool; 3],
    now: u64,
    rng: u64,
    tool: Option<usize>,
    /// Hers and his, and as drawn (moving 2 a frame).
    gauge: [i32; 2],
    shown: [i32; 2],
    locks: [bool; 2],
    rising: [bool; 2],
    rise_at: u64,
    idle_at: u64,
    click_at: Option<u64>,
    dream: bool,
    naked: bool,
    horns: bool,
    inside: bool,
    auto: bool,
    together: bool,
    his: i32,
    hers: i32,
    inserted: Option<Inserted>,
    rotor: bool,
    finger: bool,
    liquid: u8,
    /// Held action: 1 finger, 2 mouth, 3 insertion, 4 vibrator; and
    /// where (for the mouth sprite).
    action: u8,
    action_region: usize,
    holding: bool,
    looped: bool,
    anim_at: u64,
    /// Last action, repeated by auto mode.
    last: Option<(usize, usize)>,
    face: usize,
    /// Last picks of the random tables, not picked twice in a row.
    picks: [Option<usize>; 4],
    queued: Option<String>,
    voice_busy: bool,
    /// Waiting for the climax voice to end; then the base loop resumes
    /// (and her face, after her climax).
    waiting: Option<bool>,
    flash: Option<u64>,
    pointer_down: bool,
    pub hover: Option<i32>,
    pressed: Option<i32>,
    pub done: bool,
}

impl HMode {
    /// `var` reads the script's byte variables.
    pub fn new(areas: Vec<Area>, carea: Vec<u8>, var: impl Fn(usize) -> u8, seed: u64) -> Self {
        Self {
            areas,
            carea,
            flags: [
                var(ROTOR_FLAG) != 0,
                var(VIBE_FLAG) != 0,
                var(DREAM_FLAG) != 0,
            ],
            now: 0,
            rng: seed | 1,
            tool: None,
            gauge: [0; 2],
            shown: [0; 2],
            locks: [false; 2],
            rising: [false; 2],
            rise_at: 0,
            idle_at: 0,
            click_at: None,
            dream: false,
            naked: var(OUTFIT_VAR) == 1,
            horns: true,
            inside: false,
            auto: false,
            together: false,
            his: 0,
            hers: 0,
            inserted: None,
            rotor: false,
            finger: false,
            liquid: 0,
            action: 0,
            action_region: 0,
            holding: false,
            looped: false,
            anim_at: 0,
            last: None,
            face: 0,
            picks: [None; 4],
            queued: None,
            voice_busy: false,
            waiting: None,
            flash: None,
            pointer_down: false,
            hover: None,
            pressed: None,
            done: false,
        }
    }

    fn tool_open(&self, i: usize) -> bool {
        match i {
            3 => self.flags[0],
            4 => self.flags[1],
            _ => i < 5,
        }
    }

    fn button_shown(&self, id: i32) -> bool {
        match id {
            256..=260 => self.tool_open((id - ids::TOOL) as usize),
            ids::DREAM => self.flags[2],
            _ => true,
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        if let Some(a) = self
            .areas
            .iter()
            .find(|a| a.contains(x, y) && self.button_shown(a.id))
        {
            return Some(Hit::Button(a.id));
        }
        if !(0..1280).contains(&x) || !(0..720).contains(&y) {
            return None;
        }
        match self.carea.get((y * 1280 + x) as usize) {
            Some(&r) if (1..=7).contains(&r) => Some(Hit::Region(r as usize)),
            _ => None,
        }
    }

    fn random(&mut self, n: usize) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % n.max(1) as u64) as usize
    }

    /// 0x48ba00: one of `n`, not `picks[slot]` again.
    fn pick(&mut self, n: usize, slot: usize) -> usize {
        let r = match self.picks[slot] {
            Some(p) if n > 1 && p < n => {
                let r = self.random(n - 1);
                if r >= p {
                    r + 1
                } else {
                    r
                }
            }
            _ => self.random(n),
        };
        self.picks[slot] = Some(r);
        r
    }

    fn band(&self) -> usize {
        match self.gauge[0] {
            ..=299 => 0,
            300..=599 => 1,
            _ => 2,
        }
    }

    fn file(&self) -> usize {
        usize::from(self.dream)
    }

    fn main(&self, ops: &mut Vec<Op>, index: usize, cut: bool) {
        ops.push(Op::Main {
            index,
            file: self.file(),
            cut,
        });
    }

    fn diff(&self, ops: &mut Vec<Op>, index: usize) {
        ops.push(Op::Diff {
            index,
            file: self.file(),
        });
    }

    fn set_face(&mut self, ops: &mut Vec<Op>, face: usize) {
        self.face = face;
        ops.push(Op::Face(face));
    }

    /// A face of 3..5 (dream mode 10..12), not the last one again.
    fn random_face(&mut self, ops: &mut Vec<Op>) {
        let f = self.pick(3, 2) + if self.dream { 10 } else { 3 };
        self.set_face(ops, f);
    }

    /// 0x48b6b0: plays now, or after the voice playing.
    fn voice(&mut self, ops: &mut Vec<Op>, name: &str) {
        if self.voice_busy {
            self.queued = Some(name.to_string());
        } else {
            self.voice_busy = true;
            ops.push(Op::Voice(name.to_string()));
        }
    }

    /// 0x48b8e0: plays now, dropping a queued one.
    fn voice_now(&mut self, ops: &mut Vec<Op>, name: &str) {
        self.queued = None;
        self.voice_busy = true;
        ops.push(Op::Voice(name.to_string()));
    }

    fn pick_voice(&mut self, ops: &mut Vec<Op>, table: &[&str], slot: usize) {
        let i = self.pick(table.len(), slot);
        self.voice(ops, table[i]);
    }

    /// Touch voice: the dream table or the one of her gauge band.
    fn touch_voice(&mut self, ops: &mut Vec<Op>, dream_too: bool) {
        use voices::*;
        if dream_too && self.dream {
            self.pick_voice(ops, TOUCH_YUME, 0);
        } else {
            self.pick_voice(ops, TOUCH[self.band()], 0);
        }
    }

    fn banded(&mut self, ops: &mut Vec<Op>, table: [&[&str]; 3], dream: &[&str]) {
        let name = if self.dream {
            dream[0]
        } else {
            table[self.band()][0]
        };
        self.voice(ops, name);
    }

    fn raise(&mut self, which: usize, by: i32) {
        if !self.locks[which] {
            self.gauge[which] = (self.gauge[which] + by).min(FULL);
        }
    }

    /// Starts a held action at the current time.
    fn hold(&mut self, action: u8, region: usize) {
        self.action = action;
        self.action_region = region;
        self.looped = false;
        self.holding = true;
        self.rising[0] = true;
        self.rise_at = self.now;
        self.anim_at = self.now;
    }

    /// 0x486b10: `tool` on body `region`.
    fn act(&mut self, region: usize, tool: usize, ops: &mut Vec<Op>) {
        use voices::*;
        self.last = Some((region, tool));
        self.inserted = None;
        let dream = self.dream;
        match (region, tool) {
            // Head: patting.
            (1, 1) => {
                self.diff(ops, 6);
                self.raise(0, if dream { 30 } else { 15 });
                self.touch_voice(ops, true);
            }
            (2, 2) => {
                self.diff(ops, 5);
                self.raise(0, if dream { 30 } else { 15 });
                self.banded(ops, KISS, KISS_YUME);
            }
            (3 | 6, 1) | (4 | 7, 1) => {
                self.diff(ops, if matches!(region, 3 | 6) { 1 } else { 2 });
                self.random_face(ops);
                self.raise(0, if dream { 45 } else { 30 });
                self.touch_voice(ops, true);
            }
            (CROTCH, 0) => {
                self.raise(0, 45);
                self.raise(1, 45);
                ops.push(Op::StopMain);
                self.main(ops, 11, false);
                let (table, n): (&[&str], _) = if dream {
                    (PISTON_YUME, PISTON_YUME.len())
                } else {
                    let t = PISTON[self.band()];
                    (t, t.len())
                };
                let i = self.pick(n, 1);
                self.voice(ops, table[i]);
                self.inserted = Some(Inserted::Penis);
                self.rotor = false;
                self.hold(3, region);
                self.rising[1] = true;
                self.finger = false;
            }
            (CROTCH, 1) => {
                self.raise(0, 30);
                ops.push(Op::StopMain);
                self.main(ops, 8, false);
                self.touch_voice(ops, true);
                self.hold(1, region);
                self.rotor = false;
                self.finger = true;
            }
            (CROTCH, 2) => {
                self.raise(0, 30);
                ops.push(Op::StopMain);
                self.main(ops, 9, false);
                self.hold(2, region);
                self.rotor = false;
                self.finger = false;
                self.touch_voice(ops, false);
            }
            (CROTCH, 3) => {
                self.raise(0, 45);
                self.diff(ops, 0);
                ops.push(Op::StopMain);
                self.main(ops, 7, false);
                self.banded(ops, ROTOR, ROTOR_YUME);
                self.rotor = true;
                self.finger = false;
                self.rising[0] = true;
                self.rise_at = self.now;
            }
            (CROTCH, 4) => {
                self.raise(0, 45);
                ops.push(Op::StopMain);
                self.main(ops, 10, false);
                self.banded(ops, VIBE, VIBE_YUME);
                self.hold(4, region);
                self.inserted = Some(Inserted::Vibe);
                self.rotor = false;
                self.finger = false;
            }
            (6 | 7, 2) => {
                self.random_face(ops);
                self.raise(0, 30);
                self.hold(2, region);
                self.touch_voice(ops, false);
            }
            (6 | 7, 3) => {
                if self.rotor {
                    self.main(ops, 0, false);
                }
                let (other, this) = if region == 6 { (4, 3) } else { (3, 4) };
                ops.push(Op::StopDiff(other));
                self.diff(ops, 0);
                self.diff(ops, this);
                self.rising[0] = true;
                self.rise_at = self.now;
                self.raise(0, if dream { 45 } else { 30 });
                self.banded(ops, NIPPLE_ROTOR, NIPPLE_ROTOR_YUME);
            }
            _ => {}
        }
    }

    /// Primary button down: a body region acts at once (and may be held).
    pub fn pointer_down(&mut self, x: i32, y: i32, ops: &mut Vec<Op>) {
        self.pointer_down = true;
        if self.waiting.is_some() || self.done {
            return;
        }
        match self.hit(x, y) {
            Some(Hit::Region(r)) => {
                let ready = self.click_at.is_none_or(|t| self.now >= t + CLICK_GAP_MS);
                if let (true, false, Some(tool)) = (ready, self.holding, self.tool) {
                    self.act(r, tool, ops);
                    self.click_at = Some(self.now);
                    self.idle_at = self.now;
                }
            }
            Some(Hit::Button(id)) => self.pressed = Some(id),
            None => {}
        }
    }

    /// Primary button up: a button under the pointer since it went down.
    pub fn pointer_up(&mut self, x: i32, y: i32, ops: &mut Vec<Op>) {
        self.pointer_down = false;
        let pressed = self.pressed.take();
        if self.waiting.is_some() || self.done {
            return;
        }
        if let (Some(Hit::Button(id)), Some(p)) = (self.hit(x, y), pressed) {
            if id == p {
                self.press(id, ops);
            }
        }
    }

    pub fn pointer_move(&mut self, x: i32, y: i32) {
        self.hover = match self.hit(x, y) {
            Some(Hit::Button(id)) => Some(id),
            _ => None,
        };
    }

    /// 0x4852f0: a button.
    fn press(&mut self, id: i32, ops: &mut Vec<Op>) {
        use voices::*;
        let index = (id & 0xff) as usize;
        match id >> 8 {
            1 if self.tool_open(index) => self.tool = Some(index),
            2 => match index {
                0 if self.naked => {
                    self.naked = false;
                    self.diff(ops, 0x13);
                }
                1 if !self.naked => {
                    self.naked = true;
                    self.diff(ops, 0x12);
                }
                2 => {
                    self.horns = !self.horns;
                    self.diff(ops, if self.horns { 0xf } else { 0xe });
                }
                _ => {}
            },
            3 => {
                let (his, hers, d) = (self.his, self.hers, self.dream);
                let (face, table) = if his == 0 && hers == 0 {
                    (if d { 8 } else { 1 }, FINISH_NONE)
                } else if his >= 3 && hers >= 3 {
                    (if d { 13 } else { 5 }, FINISH_BOTH)
                } else if his < hers {
                    (if d { 11 } else { 5 }, FINISH_MORE)
                } else {
                    (if d { 7 } else { 0 }, FINISH_HERS)
                };
                self.set_face(ops, face);
                let i = self.random(table.len());
                self.voice_now(ops, table[i]);
                self.done = true;
            }
            4 if self.flags[2] => {
                self.dream = !self.dream;
                let f = self.face;
                let f = if f >= 7 { f - 7 } else { f + 7 };
                self.set_face(ops, f);
            }
            5 => self.inside = !self.inside,
            6 => {
                self.auto = !self.auto;
                self.last = None;
            }
            7 => self.together = !self.together,
            8 if index < 2 => self.locks[index] = !self.locks[index],
            _ => {}
        }
    }

    /// 0x48bbc0: breathing between actions.
    fn breath(&mut self, ops: &mut Vec<Op>) {
        use voices::*;
        let name = match (self.hers > 0, self.dream) {
            (false, false) => BREATH[self.band()][0],
            (false, true) => BREATH_YUME[0],
            (true, false) => BREATH_AFTER[0],
            (true, true) => BREATH_YUME_AFTER[0],
        };
        self.voice(ops, name);
    }

    /// 0x48bb10: back to the waiting loop of what is inserted.
    fn resume(&mut self, ops: &mut Vec<Op>) {
        let (index, cut) = match self.inserted {
            Some(Inserted::Penis) => (2, true),
            Some(Inserted::Vibe) => (1, true),
            None => (0, false),
        };
        self.main(ops, index, cut);
        self.breath(ops);
        self.idle_at = self.now;
    }

    /// His gauge full (0x48ab90).
    fn his_climax(&mut self, ops: &mut Vec<Op>) {
        self.his += 1;
        self.flash = Some(self.now);
        ops.push(Op::Sound("seg_100.wav"));
        let second = self.his > 1;
        let hers_full = self.gauge[0] >= FULL;
        if self.inside {
            self.diff(ops, if second { 0x19 } else { 0x18 });
            if !hers_full {
                self.set_face(ops, if self.dream { 13 } else { 6 });
            }
        } else {
            self.diff(ops, if second { 0x1b } else { 0x1a });
            if !hers_full {
                self.set_face(ops, if self.dream { 9 } else { 2 });
            }
            self.main(ops, 0, false);
            self.inserted = None;
        }
        self.gauge[1] = 0;
        self.shown[1] = 0;
    }

    /// The climax timeline and a voice, then a wait for the voice.
    fn climax_scene(&mut self, ops: &mut Vec<Op>, table: &[&str], hers: bool) {
        ops.push(Op::StopMain);
        self.diff(ops, 0);
        self.main(ops, 12, false);
        self.set_face(ops, if self.dream { 9 } else { 2 });
        let i = self.random(table.len());
        self.voice_now(ops, table[i]);
        self.waiting = Some(hers);
    }

    fn her_climax(&mut self, ops: &mut Vec<Op>) {
        use voices::*;
        let table = if self.dream { CLIMAX_YUME } else { CLIMAX };
        self.climax_scene(ops, table, true);
        self.hers += 1;
        self.gauge[0] = 0;
        self.shown[0] = 0;
        self.rising = [false; 2];
    }

    fn climax(&mut self, ops: &mut Vec<Op>) {
        use voices::*;
        let full = [self.gauge[0] >= FULL, self.gauge[1] >= FULL];
        if self.together {
            if full[0] && full[1] {
                self.his_climax(ops);
                self.her_climax(ops);
            }
            return;
        }
        if full[1] {
            self.his_climax(ops);
            if !full[0] {
                // Her answer to his climax.
                let table = if self.dream {
                    CLIMAX_TOGETHER_YUME
                } else {
                    CLIMAX_TOGETHER
                };
                self.climax_scene(ops, table, false);
                return;
            }
        }
        if full[0] {
            self.her_climax(ops);
        }
    }

    /// One frame (0x484a50): `voice` and `main` tell whether a voice and
    /// a main timeline of the model play.
    pub fn tick(&mut self, ms: u64, voice: bool, main: bool, ops: &mut Vec<Op>) {
        use voices::*;
        self.now += ms;
        for i in 0..2 {
            let step = 2 * ms.div_ceil(16).max(1) as i32;
            let (s, g) = (self.shown[i], self.gauge[i]);
            self.shown[i] = if s < g {
                (s + step).min(g)
            } else {
                (s - step).max(g)
            };
        }
        if self.flash.is_some_and(|t| self.now > t + FLASH_MS) {
            self.flash = None;
        }
        self.voice_busy = voice;
        if self.done {
            return;
        }
        if let Some(hers) = self.waiting {
            if voice {
                return;
            }
            self.waiting = None;
            self.resume(ops);
            if hers {
                self.set_face(ops, if self.dream { 9 } else { 2 });
            }
            return;
        }
        if !self.voice_busy {
            if let Some(q) = self.queued.take() {
                self.voice(ops, &q);
            }
        }
        // Auto repeats the last action once the one-shot ends.
        if let (true, Some((r, t)), false, true, false) = (
            self.auto,
            self.last,
            self.holding,
            self.inserted.is_some(),
            main,
        ) {
            self.act(r, t, ops);
            return;
        }
        // Held actions (0x48a090).
        if self.holding {
            if !self.pointer_down {
                if self.looped {
                    let end = match self.action {
                        3 => 11,
                        4 => 10,
                        _ => 0,
                    };
                    self.main(ops, end, false);
                }
                self.holding = false;
                self.looped = false;
                self.rising = [false; 2];
                self.action = 0;
            } else {
                if !main {
                    let index = match self.action {
                        1 => 3,
                        2 => 4,
                        3 => 6,
                        _ => 5,
                    };
                    self.main(ops, index, false);
                    self.looped = true;
                    if self.action == 3 {
                        self.piston_loop_voice(ops);
                    }
                }
                if !self.voice_busy {
                    match self.action {
                        1 => self.banded(ops, FINGER_LOOP, FINGER_LOOP_YUME),
                        2 => self.banded(ops, TONGUE_LOOP, TONGUE_LOOP_YUME),
                        3 => self.piston_loop_voice(ops),
                        _ => self.banded(ops, VIBE, VIBE_YUME),
                    }
                }
            }
        }
        // Her gauge falls after a while without actions.
        if !self.holding && self.now > self.idle_at + IDLE_MS {
            if !self.rising[0] && !self.locks[0] {
                self.gauge[0] = (self.gauge[0] - 15).max(0);
            }
            self.idle_at = self.now;
        }
        if self.now > self.rise_at + RISE_MS {
            for i in 0..2 {
                if self.rising[i] {
                    self.raise(i, 15);
                }
            }
            self.rise_at = self.now;
        }
        if self.gauge[0] >= 200 {
            let wet = self.inserted.is_some() || self.rotor || self.finger;
            let (want, index) = if wet { (2, 0x15) } else { (1, 0x14) };
            if self.liquid != want {
                self.diff(ops, index);
                self.liquid = want;
            }
        }
        self.climax(ops);
        if self.waiting.is_some() {
            return;
        }
        let auto_waits = self.auto
            && self
                .last
                .is_some_and(|(r, t)| !(r == CROTCH && (t == 1 || t == 2)));
        if !main && !self.holding && !auto_waits {
            self.resume(ops);
        }
    }

    fn piston_loop_voice(&mut self, ops: &mut Vec<Op>) {
        use voices::*;
        let table = if self.dream {
            PISTON_LOOP_YUME
        } else {
            PISTON_LOOP[self.band()]
        };
        self.pick_voice(ops, table, 3);
    }

    fn state(&self, id: i32) -> u32 {
        if self.pressed == Some(id) && self.hover == Some(id) {
            2
        } else if self.hover == Some(id) {
            1
        } else {
            0
        }
    }

    /// Draws the dream mist, the action sprites and the buttons. `images`
    /// are `action_p`, `hg030100_hand`, `hg030100_tongue`, `yume`,
    /// `white` (missing ones are left out).
    pub fn draw(&self, c: &mut Canvas, images: [Option<&Arc<Image>>; 5]) {
        let [sheet, hand, tongue, mist, white] = images;
        if let (true, Some(m)) = (self.dream, mist) {
            let off = ((self.now / 16) % 1280) as u32;
            c.draw_part(m, (off, 0, 1280, 720), 0, 0, false, 255);
        }
        // 0x48a940: 22 frames, one each 16 ms, 11 to a column.
        let frame = ((self.now.saturating_sub(self.anim_at) / 16) % 22) as u32;
        match (self.action, hand, tongue) {
            (1, Some(h), _) => c.draw_part(
                h,
                ((frame / 11) * 376, (frame % 11) * 224, 376, 224),
                695,
                464,
                false,
                255,
            ),
            (2, _, Some(t)) => {
                let (x, y) = match self.action_region {
                    6 => (457, 252),
                    7 => (689, 222),
                    _ => (735, 449),
                };
                c.draw_part(
                    t,
                    ((frame / 11) * 104, (frame % 11) * 96, 104, 96),
                    x,
                    y,
                    false,
                    255,
                );
            }
            _ => {}
        }
        if let Some(s) = sheet {
            self.draw_buttons(c, s);
        }
        if let (Some(t), Some(w)) = (self.flash, white) {
            let k = 1.0 - (self.now.saturating_sub(t)) as f32 / FLASH_MS as f32;
            c.draw_part(
                w,
                (0, 0, 1280, 720),
                0,
                0,
                false,
                (k.clamp(0.0, 1.0) * 255.0) as u8,
            );
        }
    }

    /// 0x486600 and the gauges 0x489740, 0x489870.
    fn draw_buttons(&self, c: &mut Canvas, s: &Arc<Image>) {
        let mut put = |sx: u32, sy: u32, w: u32, h: u32, x: i32, y: i32| {
            c.draw_part(s, (sx, sy, w, h), x, y, false, 255);
        };
        for (i, column) in TOOL_COLUMN.iter().enumerate() {
            if !self.tool_open(i) {
                continue;
            }
            let id = ids::TOOL + i as i32;
            let row = if self.tool == Some(i) {
                2
            } else {
                self.state(id)
            };
            put(column * 120, row * 88, 120, 88, 1088, i as i32 * 92 + 64);
        }
        for i in 0..3u32 {
            let on = match i {
                0 => !self.naked,
                1 => self.naked,
                _ => self.horns,
            };
            let row = if on {
                2
            } else {
                self.state(ids::DRESSED + i as i32)
            };
            put(
                (i + 10) * 56,
                row * 58 + 352,
                56,
                58,
                i as i32 * 62 + 120,
                18,
            );
        }
        if self.flags[2] {
            let row = self.state(ids::DREAM) + if self.dream { 3 } else { 0 };
            put(728, row * 58 + 352, 56, 58, 306, 18);
        }
        put(840, self.state(ids::FINISH) * 88, 120, 88, 1120, 600);
        for i in 0..2u32 {
            let selected = self.inside == (i == 0);
            let row = if selected {
                i * 4 + 10
            } else {
                self.state(ids::INSIDE + i as i32) + 8 + i * 4
            };
            put(960, row * 64, 64, 64, 1208, i as i32 * 92 + 196);
        }
        let toggle = |on: bool, st: u32| if on && st == 0 { 2 } else { st };
        put(
            960,
            toggle(self.auto, self.state(ids::AUTO)) * 64,
            64,
            64,
            1208,
            12,
        );
        put(
            960,
            (toggle(self.together, self.state(ids::TOGETHER)) + 4) * 64,
            64,
            64,
            1208,
            104,
        );
        // Her gauge (white while it moves), then his.
        let moving = |i: usize| u32::from(self.shown[i] != self.gauge[i]);
        put(0, 352, 64, 360, 24, 98);
        if self.shown[0] >= FULL {
            put(192, 352, 64, 360, 24, 98);
        }
        let h = (self.shown[0] as f32 * 0.332) as u32;
        if h > 0 {
            put((moving(0) + 1) * 64, 698 - h, 64, h, 24, 444 - h as i32);
        }
        put(256, 352, 76, 192, 18, 490);
        if self.shown[1] >= FULL {
            put(484, 352, 76, 192, 18, 490);
        }
        let h = (self.shown[1] as f32 * 0.166) as u32;
        if h > 0 {
            put(moving(1) * 76 + 332, 533 - h, 76, h, 18, 671 - h as i32);
        }
        for (i, y) in [(0usize, 182), (1, 490)] {
            if self.locks[i] || self.hover == Some(ids::LOCK + i as i32) {
                put(256, 544, 76, 192, 18, y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> HMode {
        let mut carea = vec![255u8; 1280 * 720];
        carea[400 * 1280 + 640] = CROTCH as u8;
        let areas = vec![Area {
            id: ids::TOOL,
            left: 1088,
            top: 64,
            right: 1208,
            bottom: 152,
        }];
        HMode::new(areas, carea, |_| 0, 7)
    }

    #[test]
    fn insertion_raises_both_gauges_and_loops_while_held() {
        let mut h = screen();
        let mut ops = Vec::new();
        h.pointer_down(1100, 100, &mut ops);
        h.pointer_up(1100, 100, &mut ops);
        assert_eq!(h.tool, Some(0));
        h.pointer_down(640, 400, &mut ops);
        assert_eq!(h.gauge, [45, 45]);
        assert!(ops.contains(&Op::Main {
            index: 11,
            file: 0,
            cut: false
        }));
        ops.clear();
        // The one-shot ended while the button is still down.
        h.tick(16, true, false, &mut ops);
        assert!(ops.contains(&Op::Main {
            index: 6,
            file: 0,
            cut: false
        }));
        h.pointer_up(640, 400, &mut ops);
        ops.clear();
        h.tick(16, true, true, &mut ops);
        assert!(ops.contains(&Op::Main {
            index: 11,
            file: 0,
            cut: false
        }));
    }

    #[test]
    fn full_gauge_waits_for_the_climax_voice() {
        let mut h = screen();
        let mut ops = Vec::new();
        h.gauge[0] = FULL;
        h.tick(16, false, true, &mut ops);
        assert!(ops.iter().any(|o| matches!(o, Op::Main { index: 12, .. })));
        assert_eq!((h.hers, h.gauge[0]), (1, 0));
        ops.clear();
        h.tick(16, true, true, &mut ops);
        assert!(ops.is_empty());
        h.tick(16, false, true, &mut ops);
        assert!(ops.iter().any(|o| matches!(o, Op::Main { index: 0, .. })));
    }

    #[test]
    fn picks_do_not_repeat() {
        let mut h = screen();
        let mut last = None;
        for _ in 0..50 {
            let p = h.pick(3, 0);
            assert_ne!(Some(p), last);
            last = Some(p);
        }
    }
}
