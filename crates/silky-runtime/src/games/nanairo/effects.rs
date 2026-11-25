//! Event pictures, effect pictures and scene movies of なないろリンカネーション
//! (`Game.22` subs 1 and 2, `Game.23`, `Game.1001` to `1006`).
//!
//! The effects are sprites of the shared effect stage
//! ([`crate::games::apeiria::anim`]): a picture, a keyframe list of fades,
//! scalings and turns, and a timeline that steps it once every frame of
//! 1/60 s. A call that makes the script wait gives the sprite a timeline
//! the script waits for; the others leave their timeline running. The
//! times the scripts give are scaled by the effect speed setting (half at
//! the second step, almost nothing at the third).
//!
//! What is approximate is said at each effect: where the original's
//! numbers are not known, the effect keeps the right pictures on screen
//! and uses plain values.

use silky_format::game::Kind;
use silky_vm::Flow;

use super::video::{video, BRIGHTNESS_AS_OPACITY, GREEN_AS_OPACITY, OPAQUE};
use crate::games::apeiria::anim::{Key, Look, Sprite, Stage, Step, TICK_MS};
use crate::games::Call;
use crate::host::{frames_ms, Pause, Runtime, Transition};

/// Owners on the stage: the command each effect belongs to.
const KIRARA: u32 = 22;
const EVENT: u32 = 1001;
const MOVIE: u32 = 1002;
const OVERLAY: u32 = 1003;
const PULSE: u32 = 1005;
pub(super) const ANIMATOR: u32 = 1006;
/// The ending's message overlay (`Game.50`) and the staff roll (`Game.34`).
pub(super) const ENDING: u32 = 50;
pub(super) const STAFF: u32 = 34;

/// Drawing order: the zoomed copy of the screen, the movie's picture, the
/// effect pictures, the second layer of an animator, and the flash.
const Z_ZOOM: i32 = 0;
const Z_MOVIE: i32 = 4;
pub(super) const Z_EFFECT: i32 = 5;
pub(super) const Z_SECOND: i32 = 6;
pub(super) const Z_THIRD: i32 = 7;
pub(super) const Z_FLASH: i32 = 10;

/// The middle of the screen.
pub(super) const CENTRE: (f32, f32) = (640.0, 360.0);

/// The sparkle picture of `Game.22`.
const KIRARA_PICTURE: &str = "kirara.akb";
/// The picture the event overlay of `Game.1001` lays over the scene.
const EVENT_PICTURE: &str = "HG091010_h05_eff.akb";
/// The picture of `Game.1005`.
const PULSE_PICTURE: &str = "HG080403_eff.akb";

/// Frames of a time the script gave, at the effect speed (a factor, 1, a
/// half, or 0 for instant): never less than one.
pub fn scaled(frames: i32, factor: f32) -> i32 {
    ((frames as f32 * factor).round() as i32).max(1)
}

/// Frames of a time in an effect that goes on for ever: it keeps its own
/// length when the speed is instant, so that it does not flicker.
pub(super) fn looped(frames: i32, factor: f32) -> i32 {
    if factor > 0.0 {
        scaled(frames, factor)
    } else {
        frames.max(1)
    }
}

/// Collects the steps of an effect.
pub(super) struct Fx<'a> {
    pub(super) st: &'a mut Stage,
    pub(super) owner: u32,
    pub(super) steps: Vec<Step>,
}

impl<'a> Fx<'a> {
    pub(super) fn new(st: &'a mut Stage, owner: u32) -> Self {
        Self {
            st,
            owner,
            steps: Vec::new(),
        }
    }

    /// Puts up a sprite, remembered under `slot`.
    pub(super) fn sprite(&mut self, slot: u32, look: Look, z: i32) -> u32 {
        let id = self.st.id();
        self.steps
            .push(Step::Add(Sprite::new(id, self.owner, look, z)));
        self.st.slots.insert((self.owner, slot), id);
        id
    }

    /// The sprite remembered under `slot`, if it is on the stage.
    pub(super) fn alive(&mut self, slot: u32) -> Option<u32> {
        let id = self.st.slots.get(&(self.owner, slot)).copied()?;
        self.st.sprite(id).map(|s| s.id)
    }

    /// Opacity a sprite has now.
    pub(super) fn alpha(&mut self, id: u32) -> u8 {
        self.st.sprite(id).map_or(255, |s| s.alpha)
    }

    /// Gives sprites keyframes and starts a timeline of them; `block`
    /// makes the script wait for it.
    pub(super) fn play(&mut self, sprites: Vec<(u32, Vec<Key>)>, block: bool) {
        let g = self.st.id();
        let ids: Vec<u32> = sprites.iter().map(|(id, _)| *id).collect();
        for (id, keys) in sprites {
            self.steps.push(Step::Keys(id, keys));
        }
        self.steps.push(Step::Start(g, ids));
        if block {
            self.steps.push(Step::Wait(g));
        }
    }

    /// Registers a step of the effect's own that keyframes can run.
    pub(super) fn user(&mut self, u: crate::games::apeiria::effects::User) -> u32 {
        let id = self.st.id();
        self.st.users.insert(id, u);
        id
    }

    pub(super) fn remove(&mut self, ids: &[u32]) {
        self.steps.push(Step::Remove(ids.to_vec()));
        let owner = self.owner;
        self.st
            .slots
            .retain(|(o, _), id| *o != owner || !ids.contains(id));
    }

    pub(super) fn run(self) {
        let steps = self.steps;
        self.st.play(steps);
    }
}

/// Takes an effect's sprites off the stage.
pub fn clear(st: &mut Stage, owner: u32) {
    st.clear(owner);
    st.slots.retain(|(o, _), _| *o != owner);
}

/// Takes every effect of this game off the stage.
pub fn clear_all(st: &mut Stage) {
    for owner in [
        KIRARA,
        EVENT,
        MOVIE,
        OVERLAY,
        PULSE,
        ANIMATOR,
        ENDING,
        STAFF,
        super::calendar::CALENDAR,
        super::figures::FX,
    ] {
        clear(st, owner);
    }
}

/// `Game.22.1`: the sparkle picture is put up, shown but fully clear.
pub fn kirara_ready(st: &mut Stage) {
    clear(st, KIRARA);
    let mut fx = Fx::new(st, KIRARA);
    fx.sprite(0, Look::picture(KIRARA_PICTURE), Z_FLASH);
    if let Some(Step::Add(s)) = fx.steps.last_mut() {
        s.alpha = 0;
        s.shown = true;
    }
    fx.run();
}

/// `Game.22.2`: over 16 frames the sparkle picture goes from opaque to
/// clear while it grows to 1.2 times about the middle; then it is
/// released. The script waits.
pub fn kirara_flash(st: &mut Stage, factor: f32) {
    let mut fx = Fx::new(st, KIRARA);
    let id = match fx.alive(0) {
        Some(id) => id,
        None => fx.sprite(0, Look::picture(KIRARA_PICTURE), Z_FLASH),
    };
    let key = Key::new(scaled(16, factor))
        .show(true)
        .fade(255, 0)
        .scaling((1.0, 1.0), (1.2, 1.2), CENTRE);
    fx.play(vec![(id, vec![key])], true);
    fx.remove(&[id]);
    fx.run();
}

/// `Game.1001.0`: the picture on screen grows to 1.2 times about the
/// middle over 16 frames (the script waits); then the effect picture
/// fades in, stays 24 frames and starts again, for ever.
///
/// Approximate: the length of the fade in is not known (24 frames are
/// used), nor is the middle the scene grows about (the screen's is used).
/// The movie `movie` plays over the picture, its brightness as opacity.
pub fn event_begin(st: &mut Stage, movie: Option<&str>, factor: f32) {
    clear(st, EVENT);
    let mut fx = Fx::new(st, EVENT);
    let zoom = fx.sprite(0, Look::Screen, Z_ZOOM);
    let grow = Key::new(scaled(16, factor))
        .show(true)
        .scaling((1.0, 1.0), (1.2, 1.2), CENTRE);
    fx.play(vec![(zoom, vec![grow])], true);
    let effect = fx.sprite(1, Look::picture(EVENT_PICTURE), Z_EFFECT);
    let pulse = vec![
        Key::new(looped(24, factor)).show(true).fade(0, 255),
        Key::new(1).wait(looped(24, factor)),
        Key::jump(0),
    ];
    fx.play(vec![(effect, pulse)], false);
    if let Some(name) = movie {
        fx.sprite(2, video(name, BRIGHTNESS_AS_OPACITY, true), Z_SECOND);
        if let Some(Step::Add(s)) = fx.steps.last_mut() {
            s.shown = true;
        }
    }
    fx.run();
}

/// `Game.1001.1`: over 24 frames the effect and the enlarged picture fade
/// away (the script waits), and the effect ends. The picture goes back to
/// its size by the fade, which is approximate: the original's way back is
/// not known.
pub fn event_end(st: &mut Stage, factor: f32) {
    let mut fx = Fx::new(st, EVENT);
    let ids: Vec<u32> = [0, 1, 2]
        .iter()
        .filter_map(|&slot| fx.alive(slot))
        .collect();
    if ids.is_empty() {
        return;
    }
    let frames = scaled(24, factor);
    let fades = ids
        .iter()
        .map(|&id| {
            let from = fx.alpha(id);
            (id, vec![Key::new(frames).show(false).fade(from, 0)])
        })
        .collect();
    fx.play(fades, true);
    fx.remove(&ids);
    fx.run();
}

/// `Game.1002.0` and `1002.3`: the scene movie fades in over `frames`
/// frames and stays (the script waits for the fade). `look` is the movie,
/// or its picture standing in when the movies are off, or nothing.
pub fn movie_begin(st: &mut Stage, look: Look, frames: i32, factor: f32) {
    clear(st, MOVIE);
    let mut fx = Fx::new(st, MOVIE);
    let id = fx.sprite(0, look, Z_MOVIE);
    let key = Key::new(scaled(frames, factor)).show(true).fade(0, 255);
    fx.play(vec![(id, vec![key])], true);
    fx.run();
}

/// `Game.1002.1` and `1002.4`: the movie fades out over `frames` frames
/// and is stopped (the script waits).
pub fn movie_end(st: &mut Stage, frames: i32, factor: f32) {
    fade_away(st, MOVIE, Z_MOVIE, Look::Nothing, frames, factor);
}

/// `Game.1003.0`: the effect picture fades in over `frames` frames (the
/// script waits) and stays. In the original it also keeps itself over the
/// scene's picture with a repeating key, which draws nothing more.
pub fn overlay_begin(st: &mut Stage, name: &str, frames: i32, factor: f32) {
    clear(st, OVERLAY);
    let mut fx = Fx::new(st, OVERLAY);
    let id = fx.sprite(0, Look::picture(name), Z_EFFECT);
    let key = Key::new(scaled(frames, factor)).show(true).fade(0, 255);
    fx.play(vec![(id, vec![key])], true);
    fx.run();
}

/// `Game.1003.1`: the effect picture fades out over `frames` frames (the
/// script waits) and is released.
pub fn overlay_end(st: &mut Stage, frames: i32, factor: f32) {
    fade_away(st, OVERLAY, Z_EFFECT, Look::Nothing, frames, factor);
}

/// Fades the sprite of `owner` from its opacity to nothing, with the
/// script waiting, and takes it off. A missing sprite is made (with
/// `look`) so that the time still passes.
fn fade_away(st: &mut Stage, owner: u32, z: i32, look: Look, frames: i32, factor: f32) {
    let mut fx = Fx::new(st, owner);
    let id = match fx.alive(0) {
        Some(id) => id,
        None => {
            let id = fx.sprite(0, look, z);
            if let Some(Step::Add(s)) = fx.steps.last_mut() {
                s.shown = true;
            }
            id
        }
    };
    let from = fx.alpha(id);
    let key = Key::new(scaled(frames, factor)).show(false).fade(from, 0);
    fx.play(vec![(id, vec![key])], true);
    fx.remove(&[id]);
    fx.run();
}

/// `Game.1005.0`: the effect picture fades in over 64 frames; then it
/// pulses slowly, one round of 24 frames held, then a fade out and a fade
/// in of 96 and 80 frames, which the script waits for; the pulse goes on
/// for ever.
///
/// Approximate: the picture fades down to half and not to nothing (the
/// original's low value is not known), and its first copy being drawn into
/// the picture is not separated from the pulsing layer.
pub fn pulse_begin(st: &mut Stage, factor: f32) {
    clear(st, PULSE);
    let mut fx = Fx::new(st, PULSE);
    let id = fx.sprite(0, Look::picture(PULSE_PICTURE), Z_EFFECT);
    fx.play(
        vec![(
            id,
            vec![Key::new(scaled(64, factor)).show(true).fade(0, 255)],
        )],
        true,
    );
    let round = |f: f32| {
        vec![
            Key::new(1).wait(scaled(24, f)),
            Key::new(scaled(96, f)).fade(255, 128),
            Key::new(scaled(80, f)).fade(128, 255),
        ]
    };
    fx.play(vec![(id, round(factor))], true);
    let mut forever = round(if factor > 0.0 { factor } else { 1.0 });
    forever.push(Key::jump(0));
    fx.play(vec![(id, forever)], false);
    fx.run();
}

impl Runtime {
    /// Carries out the effect commands; `None` for the calls that are not
    /// read far enough to be carried out (they are reported). With the
    /// scenes skipped in the configuration they do nothing.
    pub(crate) fn nanairo_effects(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        // The clean up calls the library makes before a scene take the
        // effects away.
        let owner = match (c.command, c.sub) {
            (1001, 3) => Some(EVENT),
            (1002, 2) => Some(MOVIE),
            (1003, 2) => Some(OVERLAY),
            (1005, 2) => Some(PULSE),
            (1006, 100) => Some(ANIMATOR),
            _ => None,
        };
        if let Some(owner) = owner {
            clear(&mut self.scene.effects, owner);
            return self.done();
        }
        if c.command == 1006 {
            return self.nanairo_animator(c);
        }
        let known = matches!(
            (c.command, c.sub),
            (22, 1 | 2)
                | (23, 0)
                | (1001, 0 | 1)
                | (1002, 0 | 1 | 3 | 4)
                | (1003, 0 | 1)
                | (1005, 0 | 1)
        );
        if !known {
            return None;
        }
        if self.scene_skip() {
            return self.done();
        }
        let f = self.system.settings.effect_scale();
        let st = &mut self.scene.effects;
        match (c.command, c.sub) {
            (22, 1) => kirara_ready(st),
            (22, _) => kirara_flash(st, f),
            (23, _) => return self.nanairo_scroll(c),
            (1001, 0) => {
                let name = "HG091010_H05.mov";
                let movie = self.movie_playable(name).then_some(name);
                event_begin(&mut self.scene.effects, movie, f);
            }
            (1001, _) => event_end(st, f),
            (1002, sub @ (0 | 3)) => {
                // The movie of `.3` supplies only the opacity of an
                // effect, so no picture stands in for it.
                let name = c.str(0).unwrap_or_default();
                let look = if self.movie_playable(&name) {
                    if sub == 0 {
                        self.movie_sound(&name);
                        video(&name, OPAQUE, false)
                    } else {
                        video(&name, GREEN_AS_OPACITY, false)
                    }
                } else {
                    let stem = name.rsplit_once('.').map_or(name.as_str(), |n| n.0);
                    let still = format!("{stem}.akb");
                    if sub == 0 && self.assets.exists(Kind::Layer, &still) {
                        Look::picture(&still)
                    } else {
                        Look::Nothing
                    }
                };
                movie_begin(&mut self.scene.effects, look, c.int(1), f);
            }
            (1002, _) => {
                self.movie_sound_stop();
                movie_end(&mut self.scene.effects, c.int(0), f);
            }
            (1003, 0) => overlay_begin(st, &c.str(0)?, c.int(1), f),
            (1003, _) => overlay_end(st, c.int(0), f),
            (1005, 0) => pulse_begin(st, f),
            (1005, _) => clear(st, PULSE),
            _ => return None,
        }
        self.nanairo_effect_wait()
    }

    /// Waits for the timelines the script has to wait for.
    pub(super) fn nanairo_effect_wait(&mut self) -> Option<silky_vm::Result<Flow>> {
        match self.scene.effects.blocking_ms() {
            Some(ms) if ms > 0 => {
                let ms = frames_ms((ms / TICK_MS) as i32);
                self.wait_for(Pause::Transition(Transition {
                    ms,
                    rule: None,
                    spin: None,
                    live: true,
                }))
            }
            _ => self.done(),
        }
    }

    /// `Game.23.0(from, to, _)`: the picture in sprite 2, wider than the
    /// screen, moves sideways one pixel a frame from window position `from`
    /// to `to` (a quarter of the steps at the second effect speed, none at
    /// the third); the script waits.
    ///
    /// Approximate: the direction is assumed, from the first number to the
    /// second as the scripts give them (0 and 820 with a 2100 pixel wide
    /// picture, so the window moves right); only that the picture is
    /// 820 pixels wider than the screen is certain.
    fn nanairo_scroll(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let layer = super::BACKGROUND_SPRITE;
        let Some(name) = self.scene.layers.get(&layer).cloned() else {
            return self.done();
        };
        let width = self.assets.image(&name).map_or(0, |i| i.width as i32);
        let most = (width - 1280).max(0);
        let (from, to) = (c.int(0).clamp(0, most), c.int(1).clamp(0, most));
        let steps = scroll_steps(from, to, self.system.settings.effect_speed);
        // The window ends at the end; the wait shows it on its way.
        self.scene.layer_view.insert(layer, (to, 0));
        if steps == 0 {
            return self.done();
        }
        let mut pan = crate::games::doukyuusei::pan::Pan {
            layer,
            from: (from, 0),
            to: (to, 0),
            steps,
            delay_ms: frames_ms(1) as i32,
            frame: 0,
        };
        let ms = pan.total_ms();
        pan.progress(0, ms);
        self.scene.pan = Some(pan);
        self.wait_for(Pause::Transition(Transition {
            ms,
            rule: None,
            spin: None,
            live: true,
        }))
    }
}

/// Steps of the scroll: one pixel each, a quarter as many at the second
/// effect speed, none (the picture is just moved) at the third.
fn scroll_steps(from: i32, to: i32, speed: u8) -> i32 {
    let distance = (to - from).abs();
    match speed {
        0 => distance,
        1 => distance / 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays the stage `frames` frames.
    fn play(st: &mut Stage, frames: u64) {
        st.advance(frames * TICK_MS);
    }

    #[test]
    fn times_follow_the_effect_speed() {
        assert_eq!(scaled(16, 1.0), 16);
        assert_eq!(scaled(16, 0.5), 8);
        assert_eq!(scaled(16, 0.0), 1);
        assert_eq!(looped(24, 0.0), 24);
        assert_eq!(looped(24, 0.5), 12);
    }

    #[test]
    fn the_sparkle_flashes_out_while_it_grows() {
        let mut st = Stage::default();
        kirara_ready(&mut st);
        assert_eq!(st.sprites.len(), 1);
        assert!(st.sprites[0].shown);
        assert_eq!(st.sprites[0].alpha, 0);
        kirara_flash(&mut st, 1.0);
        // The script waits the 16 frames and a little over.
        let wait = st.blocking_ms().unwrap();
        assert!(wait >= 16 * TICK_MS && wait <= 18 * TICK_MS, "{wait}");
        assert_eq!(st.sprites[0].alpha, 255);
        play(&mut st, 8);
        let s = &st.sprites[0];
        assert!((125..=135).contains(&s.alpha), "{}", s.alpha);
        assert!((s.scale.0 - 1.1).abs() < 0.02, "{:?}", s.scale);
        assert_eq!(s.scale_at, CENTRE);
        play(&mut st, 20);
        // Released at the end.
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn the_event_picture_grows_then_its_effect_repeats() {
        let mut st = Stage::default();
        event_begin(&mut st, None, 1.0);
        let wait = st.blocking_ms().unwrap();
        assert!(wait >= 16 * TICK_MS && wait <= 18 * TICK_MS, "{wait}");
        // Halfway through the 16 frames the picture is 1.1 times as big.
        play(&mut st, 8);
        let zoom = st.sprites.iter().find(|s| s.look == Look::Screen).unwrap();
        assert!((zoom.scale.0 - 1.1).abs() < 0.02, "{:?}", zoom.scale);
        st.settle();
        let zoom = st.sprites.iter().find(|s| s.look == Look::Screen).unwrap();
        assert!((zoom.scale.0 - 1.2).abs() < 1e-4);
        // The effect picture is on the stage and keeps going.
        assert_eq!(st.sprites.len(), 2);
        assert_eq!(st.blocking_ms(), Some(0));
        play(&mut st, 500);
        assert_eq!(st.sprites.len(), 2);
        // The end takes 24 frames and leaves nothing.
        event_end(&mut st, 1.0);
        let wait = st.blocking_ms().unwrap();
        assert!(wait >= 24 * TICK_MS && wait <= 26 * TICK_MS, "{wait}");
        st.settle();
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn a_movie_without_a_picture_only_takes_time() {
        let mut st = Stage::default();
        movie_begin(&mut st, Look::Nothing, 100, 1.0);
        let wait = st.blocking_ms().unwrap();
        assert!(wait >= 100 * TICK_MS && wait <= 102 * TICK_MS, "{wait}");
        st.settle();
        assert_eq!(st.sprites[0].alpha, 255);
        // Faded out at half the speed: 50 frames.
        movie_end(&mut st, 100, 0.5);
        let wait = st.blocking_ms().unwrap();
        assert!(wait >= 50 * TICK_MS && wait <= 52 * TICK_MS, "{wait}");
        st.settle();
        assert!(st.sprites.is_empty());
        // An end without a start still waits.
        movie_end(&mut st, 16, 1.0);
        assert!(st.blocking_ms().unwrap() >= 16 * TICK_MS);
    }

    #[test]
    fn the_effect_picture_fades_in_and_out() {
        let mut st = Stage::default();
        overlay_begin(&mut st, "HG091013_eff.akb", 80, 1.0);
        assert!(st.blocking_ms().unwrap() >= 80 * TICK_MS);
        play(&mut st, 40);
        assert!((120..=135).contains(&st.sprites[0].alpha));
        st.settle();
        assert_eq!(st.sprites[0].alpha, 255);
        overlay_end(&mut st, 80, 1.0);
        play(&mut st, 40);
        assert!((120..=135).contains(&st.sprites[0].alpha));
        st.settle();
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn the_pulse_waits_one_round() {
        let mut st = Stage::default();
        pulse_begin(&mut st, 1.0);
        let wait = st.blocking_ms().unwrap() / TICK_MS;
        // 64 frames in, then 24 + 96 + 80.
        assert!((264..=272).contains(&wait), "{wait}");
        st.settle();
        assert_eq!(st.blocking_ms(), Some(0));
        play(&mut st, 1000);
        assert_eq!(st.sprites.len(), 1);
        assert!(st.sprites[0].alpha >= 128);
    }

    #[test]
    fn the_scroll_moves_a_pixel_a_frame() {
        assert_eq!(scroll_steps(0, 820, 0), 820);
        assert_eq!(scroll_steps(0, 820, 1), 205);
        assert_eq!(scroll_steps(0, 820, 2), 0);
        assert_eq!(scroll_steps(820, 0, 0), 820);
    }
}
