//! The event animator of なないろリンカネーション (`Game.1006`): the moving
//! pictures of the scenes that have a figure over their background.
//!
//! The animator has two picture layers: A, a figure picture, and B, a
//! second figure picture over it (a third, C, comes with subs 8, 20 and
//! 30). Each sub changes what they show and starts a timeline of motion;
//! only subs 4, 5 and 6 make the script wait. Frame counts are those of
//! the original (1 frame is 16 ms), scaled by the effect speed setting.
//!
//! What is approximate is said where it is: the original's numbers that
//! are not known are replaced by the plainest reading of what is known.

use silky_vm::Flow;

use super::effects::{clear, looped, scaled, Fx, ANIMATOR, CENTRE, Z_EFFECT, Z_SECOND, Z_THIRD};
use crate::games::apeiria::anim::{Key, Look, Sprite, Step};
use crate::games::apeiria::effects::User;
use crate::games::Call;
use crate::host::Runtime;

/// The pictures of the animator.
const FIGURE_A: &str = "HG090201_h01.akb";
const FIGURE_B: &str = "HG090202_h01.akb";
const FIGURE_C: &str = "HG090202_h02.akb";
const LINES: &str = "HG090203.akb";
const BIG: &str = "HG090204.akb";
const BEAM: &str = "HG090205.akb";
const BEAM_FIGURE: &str = "HG090205_h02.akb";

/// Where each layer is remembered.
const A: u32 = 0;
const B: u32 = 1;
const C: u32 = 2;

/// The rule picture of the wipes (number 18).
const RULE: u32 = 18;
/// What the figure's bob moves by, in pixels, and how long a move takes.
const BOB: i32 = 20;
const BOB_FRAMES: i32 = 32;
/// The middle `Game.1006.3` scales and turns the lines about.
const LINES_AT: (f32, f32) = (590.0, 502.0);

/// The layer in `slot`, put up with `look` if it is not there yet (shown
/// when it is made), or given `look` if it is.
fn layer(fx: &mut Fx, slot: u32, look: Look, z: i32) -> u32 {
    match fx.alive(slot) {
        Some(id) => {
            fx.steps.push(Step::Look(id, look));
            id
        }
        None => {
            let id = fx.sprite(slot, look, z);
            if let Some(Step::Add(s)) = fx.steps.last_mut() {
                s.shown = true;
            }
            id
        }
    }
}

/// Changes a layer, whether it is on the stage already or is still to be
/// put there by this call.
fn edit(fx: &mut Fx, id: u32, f: impl FnOnce(&mut Sprite)) {
    if let Some(s) = fx.st.sprite(id) {
        return f(s);
    }
    for step in fx.steps.iter_mut() {
        if let Step::Add(s) = step {
            if s.id == id {
                return f(s);
            }
        }
    }
}

/// Puts a layer where the common set up has it: at the top left, full
/// size.
fn at_rest(fx: &mut Fx, id: u32) {
    edit(fx, id, |s| {
        s.x = 0.0;
        s.y = 0.0;
        s.scale = (1.0, 1.0);
    });
}

fn set_shown(fx: &mut Fx, id: u32, shown: bool) {
    edit(fx, id, |s| s.shown = shown);
}

/// What nearly every sub does first: the animator is made if it is not
/// there (the two figures, A over the background and B over A, B not
/// shown yet), or put back to the top left at full size if it is.
fn set_up(fx: &mut Fx) -> (u32, u32) {
    if let (Some(a), Some(b)) = (fx.alive(A), fx.alive(B)) {
        at_rest(fx, a);
        at_rest(fx, b);
        return (a, b);
    }
    let a = layer(fx, A, Look::picture(FIGURE_A), Z_EFFECT);
    let b = layer(fx, B, Look::picture(FIGURE_B), Z_SECOND);
    if let Some(Step::Add(s)) = fx.steps.last_mut() {
        s.shown = false;
    }
    (a, b)
}

/// Stops a layer where it is.
fn stop(fx: &mut Fx, id: u32) {
    fx.steps.push(Step::Keys(id, Vec::new()));
}

/// The bob of the first subs: up, marker, down, down, marker, up, round
/// again, 20 pixels either side of `mid`, 32 frames a move.
fn bob(keys: &mut Vec<Key>, mid: i32, factor: f32) {
    let n = looped(BOB_FRAMES, factor);
    let start = keys.len();
    keys.push(Key::new(n).moving((0, mid), (0, mid - BOB)));
    keys.push(Key::new(1).wait(1));
    keys.push(Key::new(n).moving((0, mid - BOB), (0, mid)));
    keys.push(Key::new(n).moving((0, mid), (0, mid + BOB)));
    keys.push(Key::new(1).wait(1));
    keys.push(Key::new(n).moving((0, mid + BOB), (0, mid)));
    keys.push(Key::jump(start));
}

/// The bob of subs 7, 12, 14 and 17: down 20 over 32 frames, back, a
/// marker, a hold of 64 frames and a marker, round again.
fn bob_down(keys: &mut Vec<Key>, factor: f32) {
    let n = looped(BOB_FRAMES, factor);
    let start = keys.len();
    keys.push(Key::new(n).moving((0, 0), (0, BOB)));
    keys.push(Key::new(n).moving((0, BOB), (0, 0)));
    keys.push(Key::new(1).wait(1));
    keys.push(Key::new(1).wait(n));
    keys.push(Key::new(1).wait(n));
    keys.push(Key::new(1).wait(1));
    keys.push(Key::jump(start));
}

/// Layer A the way the bob of subs 7, 12, 14 and 17 starts it: full
/// size, opaque, shown.
fn bob_a(fx: &mut Fx, a: u32, factor: f32) {
    edit(fx, a, |s| {
        s.scale = (1.0, 1.0);
        s.scale_at = CENTRE;
        s.alpha = 255;
        s.shown = true;
    });
    let mut keys = Vec::new();
    bob_down(&mut keys, factor);
    fx.play(vec![(a, keys)], false);
}

/// Sub 0 (and 10 without the fade in): A comes up at a quarter of its
/// size, fading in while it rises 20 pixels, then bobs for ever.
///
/// The original sets A's size to 0.25 about the middle of the screen
/// (640, 360) and animates no size after; how big that is on screen
/// depends on the picture's own size, which is not checked. It is put on A
/// as read.
fn bobbing(fx: &mut Fx, fade: bool, factor: f32) {
    let (a, b) = set_up(fx);
    edit(fx, a, |s| {
        s.y = 100.0;
        s.scale = (0.25, 0.25);
        s.scale_at = CENTRE;
        s.alpha = if fade { 0 } else { 255 };
        s.shown = true;
    });
    // B is shown, but clear: sub 2 fades it in.
    edit(fx, b, |s| {
        s.alpha = 0;
        s.shown = true;
    });
    let mut first = Key::new(scaled(64, factor))
        .show(true)
        .moving((0, 100), (0, 100 - BOB));
    if fade {
        first = first.fade(0, 255);
    }
    let mut keys = vec![first];
    // The loop of the bob goes on from the second move.
    let mut rest = Vec::new();
    bob(&mut rest, 100 - BOB, factor);
    let base = keys.len();
    for k in rest {
        keys.push(if k.flags & crate::games::apeiria::anim::flag::JUMP != 0 {
            Key::jump(base)
        } else {
            k
        });
    }
    fx.play(vec![(a, keys)], false);
}

/// Subs 1 and 11: A pans slowly sideways and back, fading out and in
/// over 64 frames each way.
///
/// The original moves A 100 pixels to the left and 20 down while it fades
/// out, then brings it in from 100 to the left of its place and 20 up to
/// it; the first round is done as read, and the way the rounds after it
/// alternate (they also move to the right) is not read far enough.
fn pan(fx: &mut Fx, factor: f32) {
    let (a, _) = set_up(fx);
    edit(fx, a, |s| {
        s.y = 100.0;
        s.scale = (0.25, 0.25);
        s.scale_at = CENTRE;
        s.shown = true;
    });
    let n = looped(64, factor);
    let keys = vec![
        Key::new(n)
            .show(true)
            .moving((0, 100), (-100, 120))
            .fade(255, 0),
        Key::new(n).moving((-100, 100), (0, 80)).fade(0, 255),
        Key::jump(0),
    ];
    fx.play(vec![(a, keys)], false);
}

/// Sub 2: B shows its figure, fading in over 8 frames, then bobs.
fn figure_in(fx: &mut Fx, factor: f32) {
    let (_, b) = set_up(fx);
    fx.steps.push(Step::Look(b, Look::picture(FIGURE_B)));
    let mut keys = vec![Key::new(scaled(8, factor)).show(true).fade(0, 255).scaling(
        (1.0, 1.0),
        (1.0, 1.0),
        CENTRE,
    )];
    bob(&mut keys, 0, factor);
    fx.play(vec![(b, keys)], false);
}

/// Subs 3 and 13: the figure picture and a picture of lines over it; the
/// lines grow to twice their size in 8 frames (fading in too in sub 3),
/// then turn 30 degrees one way and back and the other way and back, 12
/// frames each, for ever.
///
/// Unconfirmed: the lines are scaled and turned about the one point the
/// original gives for the growing, and the figure stays still.
fn sway(fx: &mut Fx, fade: bool, factor: f32) {
    let (a, b) = set_up(fx);
    fx.steps.push(Step::Look(a, Look::picture(FIGURE_B)));
    fx.steps.push(Step::Look(b, Look::picture(LINES)));
    stop(fx, a);
    set_shown(fx, a, true);
    let step = looped(12, factor);
    let mut grow = Key::new(scaled(8, factor))
        .show(true)
        .scaling((1.0, 1.0), (2.0, 2.0), LINES_AT);
    if fade {
        grow = grow.fade(0, 255);
    }
    let mut keys = vec![grow];
    for (from, to) in [(0, 30), (30, 0), (0, -30), (-30, 0)] {
        keys.push(Key::new(step).turning(from, to, LINES_AT));
    }
    keys.push(Key::jump(1));
    edit(fx, b, |s| {
        s.alpha = 255;
        s.shown = true;
        s.turn = 0.0;
    });
    fx.play(vec![(b, keys)], false);
}

/// Sub 4 (it makes the script wait): A dissolves into the picture the
/// script put on the background layer, 22 of 255 at a time, in 12 steps.
fn dissolve(fx: &mut Fx, to: &str) {
    let (a, b) = set_up(fx);
    let _ = b;
    let mut from = Look::picture(FIGURE_A);
    edit(fx, a, |s| {
        if s.look != Look::Nothing {
            from = s.look.clone();
        }
    });
    let sprite = a;
    let u = fx.user(User::Morph {
        owner: ANIMATOR,
        sprite,
    });
    fx.steps.push(Step::Look(
        a,
        Look::Mix {
            from: Box::new(from),
            to: Box::new(Look::picture(to)),
            mix: 0,
        },
    ));
    set_shown(fx, a, true);
    fx.play(vec![(a, vec![Key::new(12).show(true).user(u)])], true);
}

/// Sub 14: A takes the background layer's picture at once, and bobs.
fn take(fx: &mut Fx, from: &str, factor: f32) {
    let (a, _) = set_up(fx);
    fx.steps.push(Step::Look(a, Look::picture(from)));
    bob_a(fx, a, factor);
}

/// Subs 5 and 6 (they make the script wait): the animator's pictures
/// fade away (or, in sub 6, wipe away through rule 18) over 16 frames,
/// leaving the picture of the background layer, and the animator is gone.
fn leave(fx: &mut Fx, wipe: bool, factor: f32) {
    let frames = scaled(16, factor);
    let mut sprites = Vec::new();
    for slot in [A, B] {
        if let Some(id) = fx.alive(slot) {
            let key = if wipe {
                Key::new(frames).wiping(RULE, false)
            } else {
                let from = fx.alpha(id);
                Key::new(frames).show(false).fade(from, 0)
            };
            sprites.push((id, vec![key]));
        }
    }
    if sprites.is_empty() {
        // Nothing to show away, but the time passes.
        let id = fx.sprite(A, Look::Nothing, Z_EFFECT);
        if let Some(Step::Add(s)) = fx.steps.last_mut() {
            s.shown = true;
        }
        sprites.push((id, vec![Key::new(frames).show(false).fade(255, 0)]));
    }
    let gone: Vec<u32> = sprites.iter().map(|(id, _)| *id).collect();
    fx.play(sprites, true);
    // The animator is released; the picture the script put on the
    // background layer stays (the third layer, C, is kept too).
    fx.remove(&gone);
}

/// Subs 8 and 18: the big picture (B) comes in turning once, over 14
/// frames, at two and a half times its size, and goes on turning; the
/// figure (C) fades in over it.
fn spin(fx: &mut Fx, factor: f32) {
    let (_, b) = set_up(fx);
    let c = layer(fx, C, Look::picture(FIGURE_C), Z_THIRD);
    fx.steps.push(Step::Look(b, Look::picture(BIG)));
    let n = looped(14, factor);
    let big = (2.5, 2.5);
    let place = (-40, 140);
    set_shown(fx, b, true);
    let keys = vec![
        Key::new(n)
            .show(true)
            .fade(0, 255)
            .turning(0, 360, CENTRE)
            .scaling(big, big, CENTRE)
            .moving(place, place),
        Key::new(n).turning(0, 360, CENTRE),
        Key::jump(1),
    ];
    let figure = vec![
        Key::new(1).wait(n).show(true).fade(0, 0),
        Key::new(n).fade(0, 255),
    ];
    edit(fx, c, |s| s.alpha = 0);
    fx.play(vec![(b, keys)], false);
    fx.play(vec![(c, figure)], false);
}

/// Sub 9: the motion stops, the big picture is shown and the figure is
/// not.
fn freeze(fx: &mut Fx) {
    for slot in [A, B, C] {
        if let Some(id) = fx.alive(slot) {
            stop(fx, id);
        }
    }
    if let Some(a) = fx.alive(A) {
        set_shown(fx, a, false);
    }
    if let Some(b) = fx.alive(B) {
        set_shown(fx, b, true);
    }
}

/// Sub 19: the motion stops and the figure of the background layer stands
/// still; B is hidden. (No script uses it.)
fn still(fx: &mut Fx, from: &str) {
    let (a, b) = set_up(fx);
    stop(fx, a);
    stop(fx, b);
    fx.steps.push(Step::Look(a, Look::picture(from)));
    set_shown(fx, a, true);
    set_shown(fx, b, false);
}

/// Subs 20 and 30: a beam of light wipes in through rule 18 over 128
/// frames, and the figure over it fades up from 52 to 169 and back over
/// 128 frames each way, for ever. Sub 20 hides A and B first; sub 30
/// leaves them.
///
/// Unconfirmed: what sub 30 does beside leaving A and B (the original
/// gives it one more key of an unknown kind), and whether the beam's wipe
/// repeats; it is played once here. The units of the 128 frames of the
/// fade come from an earlier reading of the executable.
fn beam(fx: &mut Fx, hide: bool, factor: f32) {
    let (a, b) = set_up(fx);
    if hide {
        set_shown(fx, a, false);
        set_shown(fx, b, false);
    }
    let figure = layer(fx, C, Look::picture(BEAM_FIGURE), Z_THIRD);
    fx.steps.push(Step::Look(b, Look::picture(BEAM)));
    edit(fx, b, |s| {
        s.scale = (1.0, 1.0);
        s.alpha = 255;
        s.turn = 0.0;
    });
    stop(fx, a);
    let f = looped(128, factor);
    fx.play(
        vec![(
            b,
            vec![Key::new(scaled(128, factor)).show(true).wiping(RULE, true)],
        )],
        false,
    );
    fx.play(
        vec![(
            figure,
            vec![
                Key::new(f).show(true).fade(52, 169),
                Key::new(f).fade(169, 52),
                Key::jump(0),
            ],
        )],
        false,
    );
}

/// Carries a sub out on the stage. False for a sub that does nothing.
/// `plane` is the picture the script put on the background layer.
pub fn animate(
    st: &mut crate::games::apeiria::anim::Stage,
    sub: i32,
    factor: f32,
    plane: Option<&str>,
) -> bool {
    let mut fx = Fx::new(st, ANIMATOR);
    let plane = plane.unwrap_or(FIGURE_C).to_string();
    match sub {
        0 => bobbing(&mut fx, true, factor),
        10 => bobbing(&mut fx, false, factor),
        1 | 11 => pan(&mut fx, factor),
        2 => figure_in(&mut fx, factor),
        3 => sway(&mut fx, true, factor),
        13 => sway(&mut fx, false, factor),
        4 => dissolve(&mut fx, &plane),
        14 => take(&mut fx, &plane, factor),
        5 => leave(&mut fx, false, factor),
        6 => leave(&mut fx, true, factor),
        7 | 12 | 17 => {
            let (a, b) = set_up(&mut fx);
            fx.steps.push(Step::Look(b, Look::picture(FIGURE_B)));
            bob_a(&mut fx, a, factor);
        }
        8 | 18 => spin(&mut fx, factor),
        9 => freeze(&mut fx),
        19 => still(&mut fx, &plane),
        20 => beam(&mut fx, true, factor),
        30 => beam(&mut fx, false, factor),
        _ => return false,
    }
    fx.run();
    true
}

impl Runtime {
    /// `Game.1006`: the event animator. Sub 100 takes it away, the subs
    /// it does not know do nothing, and with the scenes skipped in the
    /// configuration none does anything.
    pub(crate) fn nanairo_animator(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        if c.sub == 100 {
            clear(&mut self.scene.effects, ANIMATOR);
            return self.done();
        }
        if self.scene_skip() {
            return self.done();
        }
        let f = self.system.settings.effect_scale();
        let plane = self.scene.layers.get(&super::BACKGROUND_SPRITE).cloned();
        if !animate(&mut self.scene.effects, c.sub, f, plane.as_deref()) {
            return self.done();
        }
        self.nanairo_effect_wait()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::apeiria::anim::{Stage, TICK_MS};

    fn play(st: &mut Stage, frames: u64) {
        st.advance(frames * TICK_MS);
    }

    fn slot(st: &mut Stage, slot: u32) -> crate::games::apeiria::anim::Sprite {
        let id = st.slots[&(ANIMATOR, slot)];
        st.sprite(id).unwrap().clone()
    }

    #[test]
    fn the_first_sub_fades_the_figure_in_and_bobs() {
        let mut st = Stage::default();
        assert!(animate(&mut st, 0, 1.0, None));
        assert_eq!(st.blocking_ms(), Some(0));
        let a = slot(&mut st, A);
        assert_eq!(a.alpha, 0);
        assert_eq!(a.y, 100.0);
        play(&mut st, 64 + 1);
        let a = slot(&mut st, A);
        assert_eq!(a.alpha, 255);
        assert!((78.0..=80.0).contains(&a.y), "{}", a.y);
        // 20 pixels either side of 80, for ever.
        let mut ys = Vec::new();
        for _ in 0..400 {
            play(&mut st, 1);
            ys.push(slot(&mut st, A).y as i32);
        }
        assert!(ys.iter().all(|y| (60..=100).contains(y)));
        assert!(ys.iter().any(|&y| y <= 62) && ys.iter().any(|&y| y >= 98));
        // Sub 10 has no fade in.
        let mut st = Stage::default();
        assert!(animate(&mut st, 10, 1.0, None));
        assert_eq!(slot(&mut st, A).alpha, 255);
    }

    #[test]
    fn the_pan_fades_out_and_in_for_ever() {
        let mut st = Stage::default();
        assert!(animate(&mut st, 1, 1.0, None));
        play(&mut st, 33);
        let a = slot(&mut st, A);
        assert!((120..=135).contains(&a.alpha), "{}", a.alpha);
        assert!((-55.0..=-45.0).contains(&a.x), "{}", a.x);
        play(&mut st, 64 * 6);
        assert!(st.sprites.iter().any(|s| s.busy()));
    }

    #[test]
    fn the_dissolve_takes_twelve_frames_and_ends_on_the_picture() {
        let mut st = Stage::default();
        animate(&mut st, 7, 1.0, None);
        assert!(animate(&mut st, 4, 1.0, Some("HG090202_h02.akb")));
        let wait = st.blocking_ms().unwrap() / TICK_MS;
        assert!((12..=14).contains(&wait), "{wait}");
        st.settle();
        let a = slot(&mut st, A);
        assert_eq!(a.look, Look::picture("HG090202_h02.akb"));
        assert_eq!(st.blocking_ms(), Some(0));
    }

    #[test]
    fn the_last_pictures_take_the_animator_away_after_16_frames() {
        for (sub, wait_at_half) in [(5, true), (6, false)] {
            let mut st = Stage::default();
            animate(&mut st, 0, 1.0, None);
            st.settle();
            animate(&mut st, 7, 1.0, None);
            let id = st.slots[&(ANIMATOR, A)];
            assert!(animate(&mut st, sub, 1.0, None));
            let wait = st.blocking_ms().unwrap() / TICK_MS;
            assert!((16..=18).contains(&wait), "{wait}");
            play(&mut st, 8);
            let a = st.sprite(id).unwrap().clone();
            if wait_at_half {
                assert!((120..=135).contains(&a.alpha), "{}", a.alpha);
            } else {
                assert!(a.wipe.is_some());
            }
            st.settle();
            assert!(st.sprites.is_empty());
            assert!(!st.slots.contains_key(&(ANIMATOR, A)));
        }
    }

    #[test]
    fn the_sway_turns_both_ways_and_does_not_block() {
        let mut st = Stage::default();
        assert!(animate(&mut st, 3, 1.0, None));
        assert_eq!(st.blocking_ms(), Some(0));
        play(&mut st, 9 + 6);
        let lines = slot(&mut st, B);
        assert!((lines.scale.0 - 2.0).abs() < 1e-4);
        assert!(lines.turn > 5.0 && lines.turn < 25.0, "{}", lines.turn);
        play(&mut st, 48);
        assert!(slot(&mut st, B).shown);
        // Sub 13 has no fade in.
        let mut st = Stage::default();
        assert!(animate(&mut st, 13, 1.0, None));
        assert_eq!(slot(&mut st, B).alpha, 255);
    }

    #[test]
    fn the_spin_turns_the_big_picture_and_shows_the_figure() {
        let mut st = Stage::default();
        assert!(animate(&mut st, 8, 1.0, None));
        assert_eq!(st.blocking_ms(), Some(0));
        play(&mut st, 40);
        let b = slot(&mut st, B);
        assert_eq!(b.scale, (2.5, 2.5));
        assert_eq!((b.x, b.y), (-40.0, 140.0));
        assert!(b.turn > 0.0);
        assert_eq!(slot(&mut st, C).alpha, 255);
        // Sub 9 stops it and shows only the big picture.
        assert!(animate(&mut st, 9, 1.0, None));
        let turn = slot(&mut st, B).turn;
        play(&mut st, 30);
        assert_eq!(slot(&mut st, B).turn, turn);
        assert!(slot(&mut st, B).shown && !slot(&mut st, A).shown);
    }

    #[test]
    fn the_beam_pulses_between_52_and_169_and_sub_30_keeps_the_figures() {
        let mut st = Stage::default();
        animate(&mut st, 7, 1.0, None);
        assert!(animate(&mut st, 20, 1.0, None));
        assert!(!slot(&mut st, A).shown);
        assert_eq!(st.blocking_ms(), Some(0));
        assert_eq!(slot(&mut st, C).alpha, 52);
        play(&mut st, 128 + 1);
        assert!(slot(&mut st, C).alpha >= 165);
        play(&mut st, 128);
        assert!(slot(&mut st, C).alpha <= 56);
        let mut st = Stage::default();
        animate(&mut st, 7, 1.0, None);
        assert!(animate(&mut st, 30, 1.0, None));
        assert!(slot(&mut st, A).shown);
    }

    #[test]
    fn the_subs_the_animator_does_not_know_do_nothing() {
        let mut st = Stage::default();
        for sub in [15, 16, 21, 29, 31, 99, 101] {
            assert!(!animate(&mut st, sub, 1.0, None), "{sub}");
        }
        assert!(st.sprites.is_empty());
    }
}
