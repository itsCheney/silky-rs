//! The アンネViewer of きまぐれテンプテーション (`Game.33`, the fourth mode of
//! the extras).
//!
//! It shows Anne's E-mote model over `bg004`. The left column holds twelve
//! outfits (112 pixels square, two columns of six); a locked one shows a
//! question mark. The face picture at the top (a 224 pixel picture from
//! `tab_p`, which depends on the outfit) is cut into fourteen cells of 56
//! pixels, one per expression. The big heart at the bottom steps through
//! the poses of the outfit. Under it a slider sets the size of the model.
//! On the right 180 motions are listed 18 at a time with a scroll bar; a
//! press plays the motion's timeline. A heart toggles the blush, one button
//! hides every button (a right click does too, and any press brings them
//! back), and one opens everything after a question.
//!
//! An outfit is open when byte 3010 plus its number is 1. Unlocking sets
//! them all to 1.
//!
//! The model is not drawn here: the screen keeps the scene's slot 0 up to
//! date (see [`Viewer::sync_scene`]) and draws the buttons over it.

use std::cell::Cell;

use silky_format::area::Area;

use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::scene::{Emote, Place, Scene};
use crate::Result;

/// First of the bytes that open the twelve outfits.
pub const OUTFIT_FIRST: usize = 3010;
pub const OUTFITS: usize = 12;
/// What the screen answers when it is left.
pub const LEAVE: i32 = -1;
/// The music that starts when none plays.
pub const BGM: &str = "kima_07.wav";
/// The background picture of the screen.
pub const BACKGROUND: &str = "bg004.akb";

const FACES: usize = 14;
const ROWS: usize = 18;
/// Motions in the list: the timelines 0 to 183 but four.
const MOTIONS: usize = 180;
/// The timelines the list leaves out.
const SKIPPED: [i32; 4] = [29, 75, 121, 167];
const SCROLL_MAX: usize = MOTIONS - ROWS;
const SCROLL_STEP: usize = 5;

/// Element ids, as the area file numbers them: the outfit cells are 0 to
/// 11, then the others are a kind in the high byte and a number.
const FACE: i32 = 0x100;
const POSE: i32 = 0x200;
const ROW: i32 = 0x300;
const ZOOM: i32 = 0x400;
const BACK: i32 = 0x500;
const SCROLLBAR: i32 = 0x600;
const HIDE: i32 = 0x700;
const BLUSH: i32 = 0x800;
const UNLOCK: i32 = 0x900;
/// Any press while the buttons are hidden.
const SHOWN_AGAIN: i32 = 0xf00;
const YES: i32 = 0xd00;
const NO: i32 = 0xd01;

/// The sounds of a press and of leaving.
const OK_SOUND: &str = "ses_001.wav";
const CANCEL_SOUND: &str = "ses_002.wav";

/// The size of the model: where the slider starts (0.28 of its length).
const ZOOM_START: f32 = 0.28;
const ZOOM_LEFT: i32 = 672;
const ZOOM_TOP: i32 = 632;
/// The slider's length in pixels; the pointer holds the heart by its middle.
const ZOOM_SPAN: i32 = 256;
const ZOOM_GRIP: i32 = 702;
/// The model is 32 to 100 percent of its size.
const SIZE_MIN: i32 = 32;
const SIZE_RANGE: f32 = 68.0;
/// The model is held lower by this much times the slider's value, more in
/// the second set of outfits.
const DROP: [f32; 2] = [150.0, 180.0];

const LIST_LEFT: i32 = 1005;
const LIST_TOP: i32 = 162;
const BAR_TOP: i32 = 150;
const BAR_HEIGHT: i32 = 405;
const BAR_X: i32 = 1229;
const HELP_POS: (f32, f32) = (648.0, 709.0);
const HELP_SIZE: f32 = 16.0;

const OUTFIT_NAMES: [&str; OUTFITS] = [
    "悪魔",
    "半悪魔",
    "悪魔裸",
    "半悪魔裸",
    "私服１",
    "私服１裸",
    "私服２",
    "私服２裸",
    "私服３",
    "私服３裸",
    "私服４",
    "私服４裸",
];
/// The faces of the first set of outfits, and of the second.
const FACE_NAMES: [[&str; FACES]; 2] = [
    [
        "微笑",
        "笑顔",
        "真剣",
        "呆れ",
        "悲しみ",
        "驚き",
        "微笑２",
        "笑顔２",
        "無表情",
        "気だるげ",
        "おどおど",
        "悲しみ２",
        "怒り",
        "やさぐれ",
    ],
    [
        "微笑",
        "ドヤ顔",
        "拗ね",
        "きょとん",
        "思案",
        "焦り",
        "困り微笑",
        "怒り微笑",
        "眠気",
        "ため息",
        "涙目",
        "いじけ",
        "イライラ",
        "ニヤリ",
    ],
];
const HELP: [(i32, &str); 8] = [
    (POSE, "ポーズを変更します"),
    (ROW, "アクションモーションを再生します"),
    (ZOOM, "アンネリーゼの拡大率を変更します"),
    (BACK, "アンネViewer画面から戻ります"),
    (SCROLLBAR, "アクションリストをスクロールします"),
    (HIDE, "各種ボタン類を非表示にします"),
    (BLUSH, "頬の赤らめのON/OFFを切り替えます"),
    (UNLOCK, "未解放の衣装も全て解放します"),
];

/// The scene as it was when the screen opened.
#[derive(Debug, Clone, Default)]
struct Saved {
    background: Option<String>,
    offset: (i32, i32),
    zoom: i32,
    overlays: std::collections::BTreeMap<i32, String>,
    emotes: std::collections::BTreeMap<i32, Emote>,
    emote: Option<Emote>,
}

#[derive(Debug, Clone)]
pub struct Viewer {
    areas: Vec<Area>,
    dialog_areas: Vec<Area>,
    unlocked: [bool; OUTFITS],
    /// The outfit shown, and which set (0 or 1) of the two.
    outfit: usize,
    set: usize,
    /// The face and whether the blush is on (the second timeline).
    face: usize,
    blush: bool,
    /// The timeline of the pose (0 to 2) and the presses of the pose
    /// button so far.
    pose: i32,
    steps: u32,
    /// The motion picked (its place in the list of 180).
    motion: Option<usize>,
    scroll: usize,
    /// The slider, 0 to 1.
    zoom: f32,
    hidden: bool,
    dialog: bool,
    bytes: Vec<(usize, u8)>,
    sound: Option<&'static str>,
    saved: Option<Box<Saved>>,
    /// The model changed since the scene last took it.
    dirty: bool,
    /// The help line stays on the outfit last pointed at while the pointer
    /// is over a locked one.
    help_id: Cell<Option<i32>>,
}

/// The timeline the `n`th motion of the list plays.
pub fn timeline(n: usize) -> i32 {
    let mut t = n as i32;
    for s in SKIPPED {
        if t >= s {
            t += 1;
        }
    }
    t
}

/// The size of the model in percent for a slider value.
pub fn size_percent(zoom: f32) -> i32 {
    SIZE_MIN + (zoom * SIZE_RANGE) as i32
}

/// The scroll position for a pointer at height `y` on the bar.
pub fn scroll_at(y: i32) -> usize {
    let v = (y - 178).clamp(0, BAR_HEIGHT);
    ((v * 1620 / BAR_HEIGHT + 5) / 10) as usize
}

impl Viewer {
    pub fn open(areas: Vec<Area>, dialog: Vec<Area>, bytes: impl Fn(usize) -> u8) -> Self {
        let mut unlocked = [false; OUTFITS];
        for (i, u) in unlocked.iter_mut().enumerate() {
            *u = bytes(OUTFIT_FIRST + i) == 1;
        }
        Self {
            areas,
            dialog_areas: dialog,
            unlocked,
            outfit: 0,
            set: 0,
            face: 0,
            blush: false,
            pose: 0,
            steps: 0,
            motion: None,
            scroll: 0,
            zoom: ZOOM_START,
            hidden: false,
            dialog: false,
            bytes: Vec::new(),
            sound: None,
            saved: None,
            dirty: true,
            help_id: Cell::new(None),
        }
    }

    /// The names of the files that make an outfit's model: `ta00` is the
    /// first set's first outfit.
    pub fn model_files(set: usize, outfit: usize) -> [String; 2] {
        const NUMBERS: [&str; OUTFITS] = [
            "00", "01", "02", "03", "10", "11", "20", "21", "30", "31", "40", "41",
        ];
        let base = format!("t{}{}", if set == 0 { 'a' } else { 'b' }, NUMBERS[outfit]);
        [format!("{base}_h0007.psb"), format!("{base}_h0813.psb")]
    }

    /// Where the model stands: slot 0, its size from the slider and its
    /// height from the size and the slider.
    pub fn place(&self) -> Place {
        let pct = size_percent(self.zoom);
        let s = pct as f32 / 100.0;
        let y = (s * 400.0 + (1.0 - s) * -50.0) as i32 + (self.zoom * DROP[self.set]) as i32;
        super::emote::place(0, y, pct)
    }

    /// The model as the scene shows it.
    pub fn emote(&self) -> Emote {
        Emote {
            files: Self::model_files(self.set, self.outfit),
            face: Some((self.face as i32, i32::from(self.blush))),
            sub: Some(self.pose),
            pose: self.motion.map(timeline),
            visible: true,
            place: Some(self.place()),
            ..Default::default()
        }
    }

    /// Takes the scene over: the background, and no other model.
    pub fn enter(&mut self, scene: &mut Scene) {
        self.saved = Some(Box::new(Saved {
            background: scene.background.take(),
            offset: std::mem::take(&mut scene.background_offset),
            zoom: std::mem::take(&mut scene.background_zoom),
            overlays: std::mem::take(&mut scene.overlays),
            emotes: std::mem::take(&mut scene.emotes),
            emote: scene.emote.take(),
        }));
        scene.background = Some(BACKGROUND.to_string());
        self.dirty = true;
        self.sync_scene(scene);
    }

    /// Puts the model in the scene if it changed; true when it did.
    pub fn sync_scene(&mut self, scene: &mut Scene) -> bool {
        if !std::mem::take(&mut self.dirty) {
            return false;
        }
        scene.emotes.insert(0, self.emote());
        true
    }

    /// Gives the scene back as it was.
    pub fn leave_scene(&mut self, scene: &mut Scene) {
        if let Some(s) = self.saved.take() {
            scene.background = s.background;
            scene.background_offset = s.offset;
            scene.background_zoom = s.zoom;
            scene.overlays = s.overlays;
            scene.emotes = s.emotes;
            scene.emote = s.emote;
        }
    }

    /// The knob of the slider, from the left of its track.
    fn knob(&self) -> i32 {
        ((self.zoom * 256.0 * 10.0 + 5.0) / 10.0) as i32
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<i32> {
        if self.dialog {
            return self
                .dialog_areas
                .iter()
                .find(|a| a.id < 2 && a.contains(x, y))
                .map(|a| YES + a.id);
        }
        if self.hidden {
            return Some(SHOWN_AGAIN);
        }
        self.areas.iter().find(|a| a.contains(x, y)).map(|a| a.id)
    }

    /// A locked outfit cannot be pressed.
    pub fn enabled(&self, id: i32) -> bool {
        match id {
            0..=11 => self.unlocked[id as usize],
            _ => true,
        }
    }

    pub fn click(&mut self, id: i32) -> Click {
        if self.dialog {
            self.dialog = false;
            if id == YES {
                self.unlock_all();
            }
            return Click::None;
        }
        if self.hidden {
            self.hidden = false;
            return Click::None;
        }
        match id {
            0..=11 if self.unlocked[id as usize] => {
                self.sound = Some(OK_SOUND);
                self.outfit = id as usize;
                self.dirty = true;
            }
            FACE..=0x10d => {
                self.sound = Some(OK_SOUND);
                self.face = (id - FACE) as usize;
                self.dirty = true;
            }
            POSE => {
                self.sound = Some(OK_SOUND);
                self.step_pose();
            }
            ROW..=0x311 => {
                self.sound = Some(OK_SOUND);
                let n = self.scroll + (id - ROW) as usize;
                self.motion = Some(n);
                self.dirty = true;
            }
            BACK => {
                self.sound = Some(CANCEL_SOUND);
                return Click::Result(LEAVE);
            }
            HIDE => {
                self.sound = Some(OK_SOUND);
                self.hidden = true;
            }
            BLUSH => {
                self.sound = Some(OK_SOUND);
                self.blush = !self.blush;
                self.dirty = true;
            }
            UNLOCK => {
                self.sound = Some(OK_SOUND);
                self.dialog = true;
            }
            _ => {}
        }
        Click::None
    }

    /// The pose button goes round six steps: the first set with the plain
    /// pose, the second and third pose timelines, the second set with the
    /// plain pose, and the second and third pose timelines again. The sets
    /// load their models anew; the poses only change the timeline.
    fn step_pose(&mut self) {
        self.steps += 1;
        match self.steps % 6 {
            0 => {
                self.set = 0;
                self.pose = 0;
            }
            1 | 4 => self.pose = 1,
            2 | 5 => self.pose = 2,
            _ => {
                self.set = 1;
                self.pose = 0;
            }
        }
        self.dirty = true;
    }

    fn unlock_all(&mut self) {
        for i in 0..OUTFITS {
            self.unlocked[i] = true;
            self.bytes.push((OUTFIT_FIRST + i, 1));
        }
    }

    /// A press or drag at (x, y) on the slider or the scroll bar.
    pub fn press_at(&mut self, id: i32, x: i32, y: i32) {
        if self.dialog || self.hidden {
            return;
        }
        match id {
            ZOOM => {
                let v = (x - ZOOM_GRIP).clamp(0, ZOOM_SPAN - 1);
                self.zoom = v as f32 / 256.0;
                self.dirty = true;
            }
            SCROLLBAR => self.scroll = scroll_at(y),
            _ => {}
        }
    }

    /// The mouse wheel: up goes back by five, down forward; true when the
    /// list was there to scroll.
    pub fn wheel(&mut self, up: bool) -> bool {
        if self.dialog || self.hidden {
            return true;
        }
        if up {
            self.scroll = self.scroll.saturating_sub(SCROLL_STEP);
        } else {
            self.scroll = (self.scroll + SCROLL_STEP).min(SCROLL_MAX);
        }
        true
    }

    /// Right click or Escape: the buttons go and come back; a question
    /// is answered no.
    pub fn cancel(&mut self) -> Click {
        if self.dialog {
            self.dialog = false;
        } else if self.hidden {
            self.hidden = false;
        } else {
            self.sound = Some(CANCEL_SOUND);
            self.hidden = true;
        }
        Click::None
    }

    pub fn take_bytes(&mut self) -> Vec<(usize, u8)> {
        std::mem::take(&mut self.bytes)
    }

    pub fn take_sound(&mut self) -> Option<String> {
        self.sound.take().map(str::to_string)
    }

    /// Headless run: the first thing to press.
    pub fn auto_pick(&self) -> Option<i32> {
        Some(BACK)
    }

    // ---- drawing ----

    fn state(&self, look: &Look, id: i32) -> u32 {
        if look.pressed == Some(id) {
            2
        } else if look.hover == Some(id) {
            1
        } else {
            0
        }
    }

    /// The line of help for the element under the pointer.
    fn help(&self, hover: Option<i32>) -> Option<String> {
        let Some(id) = hover else {
            self.help_id.set(None);
            return None;
        };
        if (0..OUTFITS as i32).contains(&id) && !self.unlocked[id as usize] {
            return self.help_id.get().and_then(|i| self.help_for(i));
        }
        self.help_id.set(Some(id));
        self.help_for(id)
    }

    fn help_for(&self, id: i32) -> Option<String> {
        match id {
            0..=11 => Some(OUTFIT_NAMES[id as usize].to_string()),
            FACE..=0x10d => Some(format!(
                "表情：{}",
                FACE_NAMES[self.set][(id - FACE) as usize]
            )),
            ROW..=0x311 => Some(HELP[1].1.to_string()),
            _ => HELP.iter().find(|h| h.0 == id).map(|h| h.1.to_string()),
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        if self.hidden {
            return Ok(());
        }
        let sheet = assets.image("viewer_p.akb")?;
        let tab = assets.image("tab_p.akb")?;
        let calm = Look {
            hover: None,
            pressed: None,
            ..*look
        };
        let look = if self.dialog { &calm } else { look };
        let part = |c: &mut Canvas, p: (u32, u32, u32, u32), at: (i32, i32)| {
            c.draw_part(&sheet, p, at.0, at.1, false, 255);
        };
        // Panels, the face picture and its frame.
        part(canvas, (467, 672, 243, 243), (245, 21));
        let (col, row) = (
            (self.outfit % 4) as u32,
            (self.set * 3 + self.outfit / 4) as u32,
        );
        canvas.draw_part(&tab, (col * 224, row * 224, 224, 224), 254, 30, false, 255);
        part(canvas, (0, 672, 316, 68), (670, 620));
        part(canvas, (710, 672, 444, 105), (834, 8));
        part(canvas, (1712, 0, 261, 487), (1003, 131));
        part(canvas, (1690, 0, 20, 440), (1236, 158));
        part(canvas, (710, 777, 373, 20), (613, 695));
        part(canvas, (710, 797, 157, 36), (245, 272));
        // The outfits.
        for i in 0..OUTFITS {
            let at = ((i % 2) as i32 * 112, (i / 2) as i32 * 112 + 23);
            let st = if !self.unlocked[i] {
                3
            } else if i == self.outfit {
                2
            } else {
                self.state(look, i as i32)
            };
            let p = if st == 3 {
                (0, 740, 112, 112)
            } else {
                (i as u32 * 112, st * 112 + self.set as u32 * 336, 112, 112)
            };
            part(canvas, p, at);
        }
        // The faces: a highlight over the face picture.
        for i in 0..FACES {
            let st = if i == self.face {
                2
            } else {
                self.state(look, FACE + i as i32)
            };
            if st != 0 {
                let at = (254 + (i % 4) as i32 * 56, 30 + (i / 4) as i32 * 56);
                part(canvas, (376, (st + 12) * 56, 56, 56), at);
            }
        }
        let st = self.state(look, POSE);
        let sx = if self.set == 0 { 1517 } else { 1344 };
        part(canvas, (sx, st * 183, 173, 183), (290, 504));
        // The slider.
        let st = self.state(look, ZOOM);
        part(
            canvas,
            (316, 672 + st * 55, 60, 55),
            (ZOOM_LEFT + self.knob(), ZOOM_TOP),
        );
        // The list of motions.
        for i in 0..ROWS {
            let n = self.scroll + i;
            let st = if self.motion == Some(n) {
                2
            } else {
                self.state(look, ROW + i as i32)
            };
            let top = LIST_TOP + 24 * i as i32;
            if let Some(tr) = look.text {
                let color =
                    [[255, 255, 255, 255], [255, 0, 0, 255], [255, 255, 0, 255]][st as usize];
                tr.draw_plain(
                    canvas,
                    &format!("アクション{:03}", n + 1),
                    LIST_LEFT as f32,
                    top as f32 + 20.0,
                    24.0,
                    color,
                );
            }
        }
        let st = self.state(look, SCROLLBAR);
        let y = BAR_TOP + (self.scroll as i32 * BAR_HEIGHT) / (SCROLL_MAX as i32);
        part(canvas, (432, 672 + st * 50, 35, 50), (BAR_X, y));
        // Buttons.
        let st = self.state(look, BACK);
        part(canvas, (1344, 549 + st * 57, 78, 57), (1172, 660));
        let st = self.state(look, HIDE);
        part(canvas, (1422, 549 + st * 57, 78, 57), (1081, 660));
        let st = self.state(look, UNLOCK);
        part(canvas, (1188, 672 + st * 64, 71, 64), (999, 655));
        let st = if self.blush {
            3
        } else {
            self.state(look, BLUSH)
        };
        part(canvas, (1154, 672 + st * 33, 34, 33), (358, 273));
        if let (Some(tr), Some(line)) = (look.text, self.help(look.hover)) {
            tr.draw_plain(
                canvas,
                &line,
                HELP_POS.0,
                HELP_POS.1,
                HELP_SIZE,
                [255, 255, 255, 255],
            );
        }
        if self.dialog {
            self.draw_dialog(canvas, assets, look)?;
        }
        Ok(())
    }

    /// The question before everything is unlocked (the fifth message of
    /// the dialog sheet).
    fn draw_dialog(&self, canvas: &mut Canvas, assets: &mut Assets, look: &Look) -> Result<()> {
        let sheet = assets.image("dialog_p.akb")?;
        canvas.fill(0, 0, 1280, 720, [0, 0, 0, 127]);
        canvas.draw_part(&sheet, (0, 0, 605, 276), 338, 115, false, 255);
        canvas.draw_part(&sheet, (206, 248, 123, 19), 683, 352, false, 255);
        canvas.draw_part(&sheet, (878, 200, 380, 29), 450, 215, false, 255);
        canvas.draw_part(&sheet, (878, 229 + 51 * 4, 380, 51), 450, 244, false, 255);
        let st = |id| match (look.pressed == Some(id), look.hover == Some(id)) {
            (true, _) => 2,
            (_, true) => 1,
            _ => 0,
        };
        canvas.draw_part(
            &sheet,
            (605, 200 + 51 * st(YES), 94, 51),
            494,
            295,
            false,
            255,
        );
        canvas.draw_part(
            &sheet,
            (699, 200 + 51 * st(NO), 94, 51),
            694,
            295,
            false,
            255,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewer(open: &[usize]) -> Viewer {
        let area = |id, l, t, r, b| Area {
            id,
            left: l,
            top: t,
            right: r,
            bottom: b,
        };
        Viewer::open(
            vec![area(0, 0, 23, 112, 135), area(0x100, 254, 30, 310, 86)],
            vec![area(0, 494, 295, 588, 346), area(1, 694, 295, 788, 346)],
            |i| u8::from(open.contains(&(i - OUTFIT_FIRST))),
        )
    }

    #[test]
    fn motions_skip_four_timelines() {
        assert_eq!(timeline(0), 0);
        assert_eq!(timeline(28), 28);
        assert_eq!(timeline(29), 30);
        assert_eq!(timeline(MOTIONS - 1), 183);
        let all: Vec<i32> = (0..MOTIONS).map(timeline).collect();
        assert!(SKIPPED.iter().all(|s| !all.contains(s)));
        assert!(all.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn the_slider_sets_size_and_height() {
        let mut v = viewer(&[0]);
        // 0.28 of the slider is 51 percent, held 42 pixels lower.
        assert_eq!(size_percent(ZOOM_START), 51);
        assert_eq!(v.knob(), 72);
        assert_eq!(v.place(), super::super::emote::place(0, 179 + 42, 51));
        v.press_at(ZOOM, 702 + 255, 650);
        assert_eq!(size_percent(v.zoom), 99);
        v.press_at(ZOOM, 600, 650);
        assert_eq!((v.zoom, size_percent(v.zoom), v.knob()), (0.0, 32, 0));
        assert_eq!(v.place(), super::super::emote::place(0, 94, 32));
    }

    #[test]
    fn the_scroll_bar_runs_over_the_list() {
        assert_eq!(scroll_at(0), 0);
        assert_eq!(scroll_at(178 + 405), SCROLL_MAX);
        assert_eq!(scroll_at(900), SCROLL_MAX);
        let mut v = viewer(&[0]);
        assert!(v.wheel(true));
        assert_eq!(v.scroll, 0);
        for _ in 0..40 {
            v.wheel(false);
        }
        assert_eq!(v.scroll, SCROLL_MAX);
        v.wheel(true);
        assert_eq!(v.scroll, SCROLL_MAX - 5);
        // The row pressed is counted from the top of the shown list.
        v.click(ROW + 2);
        assert_eq!(v.motion, Some(SCROLL_MAX - 5 + 2));
    }

    #[test]
    fn locked_outfits_do_not_answer() {
        let mut v = viewer(&[0, 3]);
        assert!(v.enabled(0) && !v.enabled(1) && v.enabled(3));
        v.click(1);
        assert_eq!(v.outfit, 0);
        v.click(3);
        assert_eq!(v.outfit, 3);
        assert_eq!(v.emote().files[0], "ta03_h0007.psb");
    }

    #[test]
    fn files_follow_the_set_and_the_outfit() {
        assert_eq!(Viewer::model_files(0, 0)[1], "ta00_h0813.psb");
        assert_eq!(Viewer::model_files(1, 5)[0], "tb11_h0007.psb");
        assert_eq!(Viewer::model_files(1, 11)[0], "tb41_h0007.psb");
    }

    #[test]
    fn the_pose_button_goes_round_six_steps() {
        let mut v = viewer(&[0]);
        let seen: Vec<(usize, i32)> = (0..7)
            .map(|_| {
                v.click(POSE);
                (v.set, v.pose)
            })
            .collect();
        assert_eq!(
            seen,
            [(0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 0), (0, 1)]
        );
    }

    #[test]
    fn faces_and_blush_reach_the_model() {
        let mut v = viewer(&[0]);
        v.click(FACE + 9);
        v.click(BLUSH);
        assert_eq!(v.emote().face, Some((9, 1)));
        v.click(BLUSH);
        assert_eq!(v.emote().face, Some((9, 0)));
        assert_eq!(v.help_for(FACE + 1).as_deref(), Some("表情：笑顔"));
    }

    #[test]
    fn the_question_unlocks_every_outfit() {
        let mut v = viewer(&[0]);
        v.click(UNLOCK);
        assert!(v.dialog);
        assert_eq!(v.hit(500, 300), Some(YES));
        assert_eq!(v.hit(700, 300), Some(NO));
        v.click(NO);
        assert!(v.take_bytes().is_empty() && !v.enabled(5));
        v.click(UNLOCK);
        v.click(YES);
        let b = v.take_bytes();
        assert_eq!(b.len(), OUTFITS);
        assert_eq!(b[0], (3010, 1));
        assert_eq!(b[11], (3021, 1));
        assert!(v.enabled(11));
    }

    #[test]
    fn back_leaves_and_right_click_hides_the_buttons() {
        let mut v = viewer(&[0]);
        assert_eq!(v.click(BACK), Click::Result(LEAVE));
        assert_eq!(v.cancel(), Click::None);
        assert!(v.hidden);
        assert_eq!(v.hit(10, 10), Some(SHOWN_AGAIN));
        v.click(SHOWN_AGAIN);
        assert!(!v.hidden);
        assert_eq!(v.hit(10, 30), Some(0));
    }

    #[test]
    fn the_scene_is_taken_and_given_back() {
        let mut scene = Scene {
            background: Some("bg001.akb".into()),
            ..Default::default()
        };
        scene.emotes.insert(1, Emote::default());
        let mut v = viewer(&[0]);
        v.enter(&mut scene);
        assert_eq!(scene.background.as_deref(), Some(BACKGROUND));
        assert_eq!(scene.emotes.keys().copied().collect::<Vec<_>>(), [0]);
        assert!(!v.sync_scene(&mut scene));
        v.click(FACE + 2);
        assert!(v.sync_scene(&mut scene));
        assert_eq!(scene.emotes[&0].face, Some((2, 0)));
        v.leave_scene(&mut scene);
        assert_eq!(scene.background.as_deref(), Some("bg001.akb"));
        assert_eq!(scene.emotes.keys().copied().collect::<Vec<_>>(), [1]);
    }
}
