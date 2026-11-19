//! The intro of the very first start of 同級生 Remake (`Game.31`): a
//! petal tumbles down the left of the sepia title picture and across the
//! bottom of it, and then the full colour picture (`title_n_bg`) is
//! wiped in as the petal fades. The script waits for all of it.
//!
//! Read from the executable. There are two objects, the petal
//! (`hana_0000.akb`, 128 x 128) and the colour picture, both on layer 0 and
//! the picture made after the petal, so it lies over it. The first wait
//! runs a timeline of 320 frames whose steps (frame 1 to 319) move,
//! turn and scale the petal: it starts turned 60 degrees at 1.5 times
//! its size, and a table of steps ([`intro_data::STEPS`]) gives its turn
//! and its scale across and down after each step (it turns on to -110
//! degrees, tips over about its axis ten frames at a time, and ends at
//! half its size). The place is a uniform cubic B-spline through 17
//! waypoints ([`WAYPOINTS`], the end ones counted twice), the parameter
//! starting at -1 and growing 0.053125 a step. The turn and the scale are
//! about the middle of the petal, and the place is its top left corner.
//!
//! The second wait takes 64 frames: the petal's opacity goes from 255 to
//! 0 in whole steps (and it is taken away at the end), while the colour
//! picture, which starts with no opacity, takes the opacity of a rule
//! mask (`62.bin`): from step 1 on, a threshold `T` rises by 512 / 63 a
//! step, and a pixel with mask value `r` takes the opacity `T - r` once
//! `T` has passed it (up to its own opacity). The original skips the
//! pixels that are already past the ramp, which leaves a few of them a
//! little short of full until the end; this does the same. Then the
//! picture is drawn into the background for good.
//!
//! A pass of a wait is one frame, here a sixtieth of a second as for the
//! other waits; the original's pace was not measured under Wine. Each
//! wait ends one pass after its last step. A click does not end the wait
//! (the original's wait takes no click). See `docs/games/doukyuusei.md`.

use std::sync::Arc;

use silky_format::akb::Image;

use super::intro_data::STEPS;
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// The petal's picture.
pub const PETAL: &str = "hana_0000.akb";
/// The colour picture, before the language's ending.
pub const BACKGROUND: &str = "title_n_bg";
/// The rule mask of the wipe, `62.bin`.
pub const RULE: u32 = 62;

/// The length of the timeline in frames; its steps are 1 to
/// `TIMELINE - 1`.
pub const TIMELINE: i32 = 320;
/// Steps of the timeline.
pub const TICKS: i32 = TIMELINE - 1;
/// The length of the fade and wipe in frames.
pub const FADE: i32 = 64;
/// Passes of the first wait (the timeline and one more) and of the
/// second.
pub const FIRST_WAIT: i32 = TIMELINE + 1;
pub const SECOND_WAIT: i32 = FADE + 1;
/// Passes of the whole call.
pub const FRAMES: i32 = FIRST_WAIT + SECOND_WAIT;

/// The waypoints of the petal's way, left to right in the order it takes
/// them.
pub const WAYPOINTS: [(i32, i32); 17] = [
    (375, -128),
    (247, -55),
    (118, 73),
    (35, 201),
    (-6, 329),
    (-3, 456),
    (50, 584),
    (142, 713),
    (270, 807),
    (398, 868),
    (526, 906),
    (654, 923),
    (782, 924),
    (910, 919),
    (1038, 902),
    (1166, 872),
    (1294, 816),
];
/// The spline parameter at step 1 and its growth per step.
const T_FIRST: f64 = -1.0;
const T_STEP: f64 = 0.053125;
/// The petal's size, and the place it turns and scales about.
const PETAL_SIZE: u32 = 128;
const CENTER: f32 = 64.0;
/// Where the petal is before its first step, and how it is turned and
/// scaled.
const START: (f32, f32) = (-200.0, -200.0);
const START_TURN: f32 = 60.0;
const START_SCALE: f32 = 1.5;

/// The petal as it is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Petal {
    /// The top left corner of the picture before it is turned and scaled.
    pub x: f32,
    pub y: f32,
    /// Degrees clockwise (the original keeps whole degrees when drawing).
    pub turn: f32,
    /// Scale across and down.
    pub scale: (f32, f32),
    pub alpha: u8,
}

/// How much of the colour picture is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wipe {
    /// Nothing yet.
    Hidden,
    /// Step `n` (1 to 63) of the wipe.
    Step(i32),
    /// All of it.
    Whole,
}

/// The spline's weight for a distance of `u` (not negative) between the
/// parameter and a control point.
fn basis(u: f64) -> f64 {
    if u < 1.0 {
        (3.0 * u.powf(3.0) - 6.0 * u * u + 4.0) / 6.0
    } else if u < 2.0 {
        (u - 2.0).powf(3.0) / -6.0
    } else {
        0.0
    }
}

/// Where the petal's corner is after step `tick` (1 to 319).
pub fn position(tick: i32) -> (f32, f32) {
    let mut t = T_FIRST;
    for _ in 1..tick {
        t += T_STEP;
    }
    let (mut x, mut y) = (0.0f64, 0.0f64);
    for i in -2..=18i32 {
        let w = basis((t - i as f64).abs());
        let (px, py) = WAYPOINTS[i.clamp(0, WAYPOINTS.len() as i32 - 1) as usize];
        x += px as f64 * w;
        y += py as f64 * w;
    }
    (x as f32, y as f32)
}

/// The petal after step `tick` of the timeline (0 is before the first)
/// with full opacity.
pub fn petal_at(tick: i32) -> Petal {
    let tick = tick.clamp(0, TICKS);
    if tick == 0 {
        return Petal {
            x: START.0,
            y: START.1,
            turn: START_TURN,
            scale: (START_SCALE, START_SCALE),
            alpha: 255,
        };
    }
    let (turn, sx, sy) = STEPS[tick as usize - 1];
    let (x, y) = position(tick);
    Petal {
        x,
        y,
        turn,
        scale: (sx, sy),
        alpha: 255,
    }
}

/// The petal's opacity in step `n` of the fade.
pub fn fade_alpha(n: i32) -> u8 {
    (255 + (-255 * n.clamp(0, FADE)) / FADE) as u8
}

/// What is on show after `frame` passes of the whole call: the petal (if
/// it is still there) and the colour picture.
pub fn stage(frame: i32) -> (Option<Petal>, Wipe) {
    let frame = frame.clamp(0, FRAMES);
    if frame <= FIRST_WAIT {
        return (Some(petal_at(frame.min(TICKS))), Wipe::Hidden);
    }
    let n = frame - FIRST_WAIT;
    let mut petal = petal_at(TICKS);
    if n >= FADE {
        return (None, Wipe::Whole);
    }
    petal.alpha = fade_alpha(n);
    (
        Some(petal),
        if n == 0 { Wipe::Hidden } else { Wipe::Step(n) },
    )
}

/// What step `n` of the wipe has set for each mask value, as the
/// threshold less the value (0 where nothing has).
pub fn wipe_levels(n: i32) -> [u16; 256] {
    let step = 512.0f32 / (FADE - 1) as f32;
    let width = 512 / (FADE as u32 - 1);
    let reach = width + 256;
    let mut levels = [0u16; 256];
    let mut t = step;
    for _ in 1..=n.min(FADE - 1) {
        let top = t as u32 as u16;
        // Mask values below this are past the ramp and are left alone.
        let low = if (reach as u16) < top {
            let mut s = 0u32;
            loop {
                s += width;
                if (s as u16 as u32) + reach >= top as u32 {
                    break s as u16;
                }
            }
        } else {
            0
        };
        for (r, level) in levels.iter_mut().enumerate() {
            let r = r as u16;
            if low <= r && r < top {
                *level = top - r;
            }
        }
        t += step;
    }
    levels
}

/// The corners (top left, top right, bottom left) the petal's picture is
/// put at, for [`Canvas::draw_sprite`].
pub fn corners(p: &Petal) -> [[f32; 2]; 3] {
    // The turn is kept in whole degrees.
    let rad = (p.turn as i32) as f32 * (std::f32::consts::PI / 180.0);
    let (s, c) = rad.sin_cos();
    let at = |lx: f32, ly: f32| {
        let (vx, vy) = ((lx - CENTER) * p.scale.0, (ly - CENTER) * p.scale.1);
        [
            p.x + CENTER + vx * c - vy * s,
            p.y + CENTER + vx * s + vy * c,
        ]
    };
    let size = PETAL_SIZE as f32;
    [at(0.0, 0.0), at(size, 0.0), at(0.0, size)]
}

/// The colour picture with the opacity step `n` of the wipe gives it.
fn wiped(img: &Image, rule: &[u8], n: i32) -> Image {
    let levels = wipe_levels(n);
    let mut rgba = img.rgba.clone();
    for (px, &r) in rgba.chunks_exact_mut(4).zip(rule) {
        px[3] = px[3].min(levels[r as usize].min(255) as u8);
    }
    Image {
        width: img.width,
        height: img.height,
        rgba,
    }
}

/// The intro while the script waits for it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Intro {
    /// The colour picture (in the language's version), which becomes the
    /// background when the intro is over.
    pub background: String,
    /// The rule mask, one byte a pixel.
    #[serde(skip)]
    pub rule: Option<Arc<Vec<u8>>>,
    /// Passes of the wait gone by.
    #[serde(skip)]
    pub frame: i32,
}

impl Intro {
    pub fn new(background: String, rule: Option<Arc<Vec<u8>>>) -> Self {
        Self {
            background,
            rule,
            frame: 0,
        }
    }

    /// The wait has gone `elapsed` of `total` milliseconds.
    pub fn progress(&mut self, elapsed: u32, total: u32) {
        let f = elapsed as u64 * FRAMES as u64 / total.max(1) as u64;
        self.frame = f.min(FRAMES as u64) as i32;
    }

    /// Draws the petal and the colour picture over what is there.
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets) -> Result<()> {
        let (petal, wipe) = stage(self.frame);
        if let Some(p) = petal.filter(|p| p.alpha > 0) {
            let img = assets.image(PETAL)?;
            let part = (0, 0, PETAL_SIZE, PETAL_SIZE);
            canvas.draw_sprite(
                Some(&img),
                part,
                corners(&p),
                [255, 255, 255, p.alpha],
                false,
            );
        }
        if wipe == Wipe::Hidden {
            return Ok(());
        }
        let img = assets.image(&self.background)?;
        let x = (canvas.width as i32 - img.width as i32) / 2;
        let y = (canvas.height as i32 - img.height as i32) / 2;
        let all = (0, 0, img.width, img.height);
        match (wipe, self.rule.as_deref()) {
            (Wipe::Step(n), Some(rule)) if rule.len() >= (img.width * img.height) as usize => {
                let img = Arc::new(wiped(&img, rule, n));
                canvas.draw_part(&img, all, x, y, false, 255);
            }
            // Without the mask, an even fade.
            (Wipe::Step(n), _) => {
                let opacity = (n * 255 / (FADE - 1)).min(255) as u8;
                canvas.draw_part(&img, all, x, y, false, opacity);
            }
            _ => canvas.draw_part(&img, all, x, y, false, 255),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: (f32, f32), b: (f32, f32)) {
        assert!(
            (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3,
            "{a:?} != {b:?}"
        );
    }

    #[test]
    fn calls_add_up() {
        assert_eq!(STEPS.len() as i32, TICKS);
        assert_eq!(FIRST_WAIT, 321);
        assert_eq!(SECOND_WAIT, 65);
        assert_eq!(FRAMES, 386);
    }

    #[test]
    fn the_way_through_the_waypoints() {
        // The first step is at the first waypoint (the end ones count
        // twice), the rest as the original works them out.
        near(position(1), (375.0, -128.0));
        near(position(2), (374.99680, -127.99818));
        near(position(20), (353.06107, -115.48794));
        near(position(60), (111.83660, 90.20000));
        near(position(100), (-2.09728, 361.87582));
        near(position(200), (471.20001, 888.77582));
        near(position(300), (1151.19995, 872.46802));
        near(position(319), (1265.16980, 828.60803));
    }

    #[test]
    fn the_steps_of_the_petal() {
        assert_eq!(petal_at(0).x, -200.0);
        assert_eq!((petal_at(0).turn, petal_at(0).scale), (60.0, (1.5, 1.5)));
        // It starts to turn and shrink at step 20.
        assert_eq!(petal_at(19).turn, 60.0);
        assert_eq!(petal_at(21).turn, 59.0);
        assert!((petal_at(21).scale.0 - 1.49625).abs() < 1e-6);
        assert_eq!(petal_at(60).turn, 30.0);
        // About step 250 it is edge on and turns over.
        assert_eq!(petal_at(250).scale.0, 0.0);
        assert!((petal_at(250).scale.1 - 0.7916667).abs() < 1e-6);
        let last = petal_at(TICKS);
        assert_eq!(last.turn, -109.75);
        assert!((last.scale.0 - 0.5041667).abs() < 1e-6);
        // Past the timeline the last step stays.
        assert_eq!(petal_at(400), last);
    }

    #[test]
    fn the_stages_of_the_call() {
        // The first wait: the petal only, from before its first step.
        assert_eq!(stage(0), (Some(petal_at(0)), Wipe::Hidden));
        assert_eq!(stage(1), (Some(petal_at(1)), Wipe::Hidden));
        assert_eq!(stage(FIRST_WAIT), (Some(petal_at(TICKS)), Wipe::Hidden));
        // The second: it fades and the picture is wiped in.
        let (p, w) = stage(FIRST_WAIT + 1);
        assert_eq!((p.map(|p| p.alpha), w), (Some(252), Wipe::Step(1)));
        let (p, w) = stage(FIRST_WAIT + 63);
        assert_eq!((p.map(|p| p.alpha), w), (Some(4), Wipe::Step(63)));
        assert_eq!(stage(FIRST_WAIT + 64), (None, Wipe::Whole));
        assert_eq!(stage(FRAMES), (None, Wipe::Whole));
        assert_eq!(stage(FRAMES + 9), (None, Wipe::Whole));
    }

    #[test]
    fn the_fade_steps() {
        assert_eq!(fade_alpha(0), 255);
        assert_eq!(fade_alpha(1), 252);
        assert_eq!(fade_alpha(32), 128);
        assert_eq!(fade_alpha(63), 4);
        assert_eq!(fade_alpha(64), 0);
    }

    #[test]
    fn the_wipe_rises_through_the_mask() {
        // Step 1: the threshold is 8, so mask values 0 to 7 have a
        // little opacity, the lowest the most.
        let l = wipe_levels(1);
        assert_eq!((l[0], l[1], l[7], l[8], l[255]), (8, 7, 1, 0, 0));
        // Step 31: the threshold is 251.
        let mid = wipe_levels(31);
        assert_eq!((mid[0], mid[100], mid[250], mid[251]), (251, 151, 1, 0));
        // Value 0 is last touched at step 32 (threshold 260), before the
        // ramp, 264 wide, has left it.
        assert_eq!(wipe_levels(32)[0], 260);
        assert_eq!(wipe_levels(63)[0], 260);
        // At the end every value has been reached, the last ones with
        // the threshold just past 255.
        let end = wipe_levels(63);
        assert!(end.iter().all(|&v| v >= 200));
        assert!(end[255] >= 256);
        // More steps than the wipe has change nothing.
        assert_eq!(wipe_levels(99), end);
    }

    #[test]
    fn pixels_past_the_ramp_are_skipped() {
        // Step 40: the threshold is 325; the ramp 264 wide starts at 64,
        // so mask values below it are no longer touched and keep what the
        // step before left them.
        let before = wipe_levels(39);
        let now = wipe_levels(40);
        assert_eq!(&before[..60], &now[..60]);
    }

    #[test]
    fn wiped_picture_takes_the_mask() {
        let img = Image {
            width: 2,
            height: 1,
            rgba: vec![10, 20, 30, 255, 40, 50, 60, 100],
        };
        let out = wiped(&img, &[0, 200], 1);
        // The first pixel has mask value 0 (opacity 8), the second is
        // beyond the threshold.
        assert_eq!(out.rgba, vec![10, 20, 30, 8, 40, 50, 60, 0]);
        // The pixel keeps its own opacity if that is lower.
        let out = wiped(&img, &[200, 0], 63);
        assert_eq!(out.rgba[3], 255);
        assert_eq!(out.rgba[7], 100);
    }

    #[test]
    fn corners_turn_about_the_middle() {
        let p = Petal {
            x: 100.0,
            y: 50.0,
            turn: 0.0,
            scale: (1.0, 1.0),
            alpha: 255,
        };
        assert_eq!(corners(&p), [[100.0, 50.0], [228.0, 50.0], [100.0, 178.0]]);
        // Half size about the middle.
        let half = Petal {
            scale: (0.5, 0.5),
            ..p
        };
        assert_eq!(
            corners(&half),
            [[132.0, 82.0], [196.0, 82.0], [132.0, 146.0]]
        );
        // A quarter turn clockwise sends the top left to the top right.
        let quarter = Petal { turn: 90.0, ..p };
        let c = corners(&quarter);
        assert!((c[0][0] - 228.0).abs() < 1e-3 && (c[0][1] - 50.0).abs() < 1e-3);
        // Whole degrees only.
        let a = corners(&Petal { turn: 33.9, ..p });
        let b = corners(&Petal { turn: 33.0, ..p });
        assert_eq!(a, b);
    }

    #[test]
    fn progress_maps_the_wait_to_passes() {
        let mut i = Intro::new("title_n_bg_en.akb".into(), None);
        i.progress(0, 6433);
        assert_eq!(i.frame, 0);
        i.progress(6433 / 2, 6433);
        assert_eq!(i.frame, 192);
        i.progress(9999, 6433);
        assert_eq!(i.frame, FRAMES);
    }
}
