//! The title menu of きまぐれテンプテーション (`Game.110`) and its extras
//! page.
//!
//! Each page is a row of six cells of `title_p.akb` along the bottom edge
//! (62 pixels high, four state rows per cell: normal, hover with a heart
//! above, pressed, disabled). The cells, their places and the bytes that
//! unlock the extras come from the executable. The buttons jump in from
//! below one after the other, 100 ms apart, and a page that is left falls
//! out of sight the same way. The picture of the heroine on the title
//! changes her face now and then from the five on `titlea_p.akb`. Faint
//! sparkles (`Prism.akb`) drift over the whole title.

use std::sync::Mutex;

use super::fx::Mt;
use crate::assets::Assets;
use crate::render::Canvas;
use crate::ui::{title, Button, Ui};
use crate::Result;

/// Row of the buttons, and the cell height.
const TOP: i32 = 652;
const ROW: u32 = 62;
/// Where a button comes from and where a fallen one stays.
const BELOW: f32 = 720.0;
const DROP: f32 = 800.0;
/// A button leaves 16 pixels a frame up and gains this much speed each
/// frame; a bounce keeps half of it.
const KICK: f32 = -16.0;
const GRAVITY: f32 = 0.98;
/// Milliseconds between one button starting and the next.
const STAGGER: u64 = 100;

/// Face picture: where it goes, the size of a face on the sheet, and how
/// long each stays before the next fades in.
const FACE_AT: (i32, i32) = (602, 13);
const FACE: (u32, u32) = (447, 341);
/// Faces the engine picks from (the fourth is never shown), and the wait.
const FACES: [u32; 4] = [0, 1, 2, 4];
const FACE_WAIT: u64 = 5000;
/// A fade adds 16 to the alpha each frame, so it takes 16 frames.
const FADE_FRAMES: u64 = 16;
const FADE_MS: u64 = FADE_FRAMES * 1000 / 60;

/// Bytes of the script that unlock the extras (CG, scene, music, アンネ
/// Viewer, 捜査手帳); 戻る always works.
pub const UNLOCKS: [(usize, usize); 5] = [
    (title::CG_MODE, 0xbba),
    (title::SCENE_MODE, 0xbbb),
    (title::MUSIC_MODE, 0xbbc),
    (title::VIEWER, 0xbbd),
    (title::NOTEBOOK, 0xbbe),
];

/// One button: entry, left edge, width, and its cell on the sheet.
struct Cell {
    entry: usize,
    x: i32,
    w: u32,
    src: (u32, u32),
}

const fn cell(entry: usize, x: i32, w: u32, sx: u32, sy: u32) -> Cell {
    Cell {
        entry,
        x,
        w,
        src: (sx, sy),
    }
}

const MAIN: [Cell; 6] = [
    cell(title::NEW_GAME, 188, 115, 0, 0),
    cell(title::CONTINUE, 335, 161, 115, 0),
    cell(title::LOAD, 528, 101, 276, 0),
    cell(title::SETTINGS, 661, 101, 377, 0),
    cell(title::EXTRAS, 795, 78, 767, 0),
    cell(title::QUIT, 905, 186, 845, 0),
];

const EXTRAS: [Cell; 6] = [
    cell(title::CG_MODE, 189, 110, 0, 248),
    cell(title::SCENE_MODE, 331, 160, 110, 248),
    cell(title::MUSIC_MODE, 523, 163, 270, 248),
    cell(title::VIEWER, 718, 166, 433, 248),
    cell(title::NOTEBOOK, 916, 103, 599, 248),
    cell(title::BACK, 1050, 57, 702, 248),
];

/// What the script is answered for an entry that leaves the menu. The
/// quit button asks on its own and the notebook is a screen of the menu.
pub fn result(entry: usize) -> Option<i32> {
    match entry {
        title::QUIT | title::NOTEBOOK => None,
        e => title::result(e),
    }
}

fn page(extras: bool) -> &'static [Cell; 6] {
    if extras {
        &EXTRAS
    } else {
        &MAIN
    }
}

/// Height of a button `ms` after it starts moving from `y0` with speed
/// `v0`, one step per 60 Hz frame, bouncing on `place` at half speed until
/// the bounce is under a pixel.
fn step_y(ms: u64, y0: f32, v0: f32, place: f32) -> f32 {
    let (mut y, mut v) = (y0, v0);
    for _ in 0..ms * 60 / 1000 {
        v += GRAVITY;
        y += v;
        if v > 0.0 && y >= place {
            y = place;
            v *= -0.5;
            if v as i32 == 0 {
                return place;
            }
        }
    }
    y
}

/// Top of button `k` of a page that came up `ms` ago.
fn jump_top(ms: u64, k: usize) -> i32 {
    match ms.checked_sub(STAGGER * k as u64) {
        Some(ms) => step_y(ms, BELOW, KICK, TOP as f32) as i32,
        None => BELOW as i32,
    }
}

/// Top of button `k` of a page that was left `ms` ago, or `None` once it
/// is out of sight.
fn fall_top(ms: u64, k: usize) -> Option<i32> {
    let ms = ms.saturating_sub(STAGGER * k as u64);
    let y = step_y(ms, TOP as f32, 0.0, DROP) as i32;
    (y < BELOW as i32).then_some(y)
}

/// Whether the menu moves, so that a frame has to be drawn: the sparkles
/// never rest while the title is up.
pub fn animating(_ui: &Ui, _before: u64) -> bool {
    true
}

/// Face shown after `k` changes, and the one the next change picks: the
/// engine picks among the other three of [`FACES`] at random.
fn faces(k: u64) -> u32 {
    let mut face = 0;
    for n in 0..k {
        let rest: Vec<u32> = FACES.iter().copied().filter(|&f| f != face).collect();
        let r = (n + 1).wrapping_mul(0x9e37_79b1) >> 7;
        face = rest[r as usize % rest.len()];
    }
    face
}

/// Draws the title buttons of one page. `shown` is the time since the
/// page came up, `opened` since the menu came up; `from` is the page that
/// was left for this one, if any.
pub fn draw(
    canvas: &mut Canvas,
    assets: &mut Assets,
    ui: &Ui,
    extras: bool,
    from: Option<bool>,
) -> Result<()> {
    let opened = ui.clock.saturating_sub(ui.title_opened);
    let shown = ui.clock.saturating_sub(ui.title_shown);
    draw_face(canvas, assets, opened)?;
    let sheet = assets.image("title_p.akb")?;
    let mut put = |c: &Cell, top: i32, state: u32| {
        let part = (c.src.0, c.src.1 + ROW * state, c.w, ROW);
        canvas.draw_part(&sheet, part, c.x, top, false, 255);
    };
    let state_of = |c: &Cell| {
        let b = Button::Title(c.entry);
        ui.state(b, ui.enabled(b), false) as u32
    };
    if let Some(old) = from {
        for (k, c) in page(old).iter().enumerate() {
            if let Some(top) = fall_top(shown, k) {
                let b = Button::Title(c.entry);
                put(c, top, if ui.enabled(b) { 0 } else { 3 });
            }
        }
    }
    for (k, c) in page(extras).iter().enumerate() {
        put(c, jump_top(shown, k), state_of(c));
    }
    draw_prism(canvas, assets, ui.title_opened, opened)
}

/// The sparkle effect of the title: twenty sprites of `Prism.akb` (64 by
/// 64, turned on at the middle) of which a new one is started at random
/// intervals. A sparkle starts in the lower right of the screen at a random
/// size and drifts up and to the right until its life of two to four
/// seconds is over or it leaves the top (64) or the right edge, pulsing
/// through a table of faint alphas. It is a sprite with a priority, so it
/// sits over what the title itself draws, the buttons included (the
/// original does not say it in so many words; the draw order of the two
/// lists is read from the sprite list code, not compared with the game).
mod prism {
    use super::Mt;

    pub const COUNT: usize = 20;
    /// The next sparkle starts within this many milliseconds.
    const SPAWN_GAP: u32 = 500;
    /// The original draws every number from a freshly seeded engine with
    /// `below(0xb70a7c3) % n`; one engine seeded here keeps runs equal.
    const RANGE: u32 = 0xb70a_7c3;
    /// Alpha by (counter / 8) % 10, where the counter starts at random and
    /// grows one a frame.
    pub const ALPHA: [u8; 10] = [48, 48, 92, 0, 48, 48, 48, 48, 48, 48];
    const SEED: u32 = 0x5eed_0001;

    #[derive(Debug, Clone, Copy, Default)]
    pub struct Spark {
        pub on: bool,
        pub x: f32,
        pub y: f32,
        pub vx: f32,
        pub vy: f32,
        pub scale: f32,
        pub counter: u32,
        start: u64,
        life: u64,
    }

    impl Spark {
        pub fn alpha(&self) -> u8 {
            ALPHA[(self.counter as usize >> 3) % 10]
        }
    }

    #[derive(Debug, Clone)]
    pub struct Prism {
        rng: Mt,
        pub sparks: [Spark; COUNT],
        next: u64,
        /// Frames done so far.
        pub frames: u64,
    }

    impl Prism {
        pub fn new() -> Self {
            Self {
                rng: Mt::new(SEED),
                sparks: [Spark::default(); COUNT],
                next: 0,
                frames: 0,
            }
        }

        fn rand(&mut self, n: u32) -> u32 {
            self.rng.below(RANGE) % n
        }

        fn spawn(&mut self, i: usize, now: u64) {
            let x = (self.rand(1000) + 0x78) as f32;
            let y = (self.rand(0x168) + 0x168) as f32;
            let vy = (self.rand(300) as f32 / 100.0) * -1.0 - 0.1;
            let vx = self.rand(300) as f32 / 100.0 + 0.1;
            let scale = self.rand(100) as f32 / 100.0 + 0.1;
            let counter = self.rand(1000) % 100;
            let life = (self.rand(2000) + 2000) as u64;
            self.sparks[i] = Spark {
                on: true,
                x,
                y,
                vx,
                vy,
                scale,
                counter,
                start: now,
                life,
            };
        }

        /// One frame at time `now` (ms): maybe start a sparkle, then move
        /// them all.
        pub fn step(&mut self, now: u64) {
            if let Some(i) = self.sparks.iter().position(|s| !s.on) {
                if now >= self.next {
                    self.spawn(i, now);
                    self.next = now + self.rand(SPAWN_GAP) as u64;
                }
            }
            for s in self.sparks.iter_mut().filter(|s| s.on) {
                s.y += s.vy;
                s.x += s.vx;
                if now >= s.start + s.life || s.x >= 1280.0 || s.y <= 64.0 {
                    s.on = false;
                }
                s.counter += 1;
            }
            self.frames += 1;
        }
    }
}

/// The running sparkles, kept between frames; restarted when the title is
/// opened again or the clock goes back.
static PRISM: Mutex<Option<(u64, prism::Prism)>> = Mutex::new(None);

fn draw_prism(canvas: &mut Canvas, assets: &mut Assets, opened_at: u64, opened: u64) -> Result<()> {
    let sheet = assets.image("Prism.akb")?;
    let want = opened * 60 / 1000 + 1;
    let mut guard = PRISM.lock().unwrap();
    match guard.as_mut() {
        Some((at, p)) if *at == opened_at && p.frames <= want => {}
        _ => *guard = Some((opened_at, prism::Prism::new())),
    }
    let (_, p) = guard.as_mut().unwrap();
    while p.frames < want {
        p.step(p.frames * 1000 / 60);
    }
    for s in p.sparks.iter().filter(|s| s.on) {
        let size = 64.0 * s.scale;
        let (x, y) = (s.x - 32.0 * s.scale, s.y - 32.0 * s.scale);
        canvas.draw_sprite(
            Some(&sheet),
            (0, 0, 64, 64),
            [[x, y], [x + size, y], [x, y + size]],
            [255, 255, 255, s.alpha()],
            false,
        );
    }
    Ok(())
}

/// The face of the heroine, `opened` ms after the menu came up.
fn draw_face(canvas: &mut Canvas, assets: &mut Assets, opened: u64) -> Result<()> {
    let cycle = FACE_WAIT + FADE_MS;
    let (k, r) = (opened / cycle, opened % cycle);
    let sheet = assets.image("titlea_p.akb")?;
    let mut put = |face: u32, alpha: u8| {
        let src = (face % 3 * FACE.0, face / 3 * FACE.1, FACE.0, FACE.1);
        canvas.draw_part(&sheet, src, FACE_AT.0, FACE_AT.1, false, alpha);
    };
    // The first face is the one of the picture itself.
    if k > 0 {
        put(faces(k), 255);
    }
    if r >= FACE_WAIT {
        let frames = ((r - FACE_WAIT) * 60 / 1000 + 1).min(FADE_FRAMES);
        put(faces(k + 1), (frames * 16).min(255) as u8);
    }
    Ok(())
}

/// The button under (x, y) on a page.
pub fn hit(extras: bool, x: i32, y: i32) -> Option<Button> {
    page(extras)
        .iter()
        .find(|c| x >= c.x && x < c.x + c.w as i32 && y >= TOP && y < TOP + ROW as i32)
        .map(|c| Button::Title(c.entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_jump_in_and_settle() {
        assert_eq!(jump_top(0, 0), 720);
        assert!(jump_top(200, 0) < TOP);
        assert_eq!(jump_top(0, 3), 720);
        assert_eq!(jump_top(3000, 5), TOP);
    }

    #[test]
    fn a_left_page_falls_away() {
        assert_eq!(fall_top(0, 0), Some(TOP));
        assert_eq!(fall_top(1000, 0), None);
    }

    #[test]
    fn faces_never_repeat_or_show_the_fourth() {
        let mut last = 0;
        for k in 1..50 {
            let f = faces(k);
            assert!(f != last && f != 3);
            last = f;
        }
    }

    #[test]
    fn sparkles_start_drift_and_die() {
        let mut p = prism::Prism::new();
        p.step(0);
        assert_eq!(p.sparks.iter().filter(|s| s.on).count(), 1);
        for k in 1..600 {
            p.step(k * 1000 / 60);
        }
        let on: Vec<_> = p.sparks.iter().filter(|s| s.on).collect();
        assert!(!on.is_empty() && on.len() <= prism::COUNT);
        assert!(on
            .iter()
            .all(|s| s.x < 1280.0 && s.y > 64.0 && s.scale >= 0.1));
    }

    #[test]
    fn hits_follow_the_page() {
        assert_eq!(hit(false, 200, 680), Some(Button::Title(title::NEW_GAME)));
        assert_eq!(hit(true, 1060, 680), Some(Button::Title(title::BACK)));
        assert_eq!(hit(false, 200, 600), None);
    }
}
