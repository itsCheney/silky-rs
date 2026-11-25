//! The sprite animations behind 景の海のアペイリア's attack and spell
//! effects (`Game.998`, `1000`, `1001`, `1002`), played the way the engine
//! plays them.
//!
//! Every sprite carries a list of keyframes. A keyframe starts the changes
//! it names (a wait, showing or hiding, a move, a fade, a turn, a scaling,
//! or a hand-written step of its own, [`super::effects::User`]) and lasts a
//! number of frames; the values go linearly from the start to the end
//! value over those frames, and the last keyframe can jump back to an
//! earlier one to loop. A timeline steps all its sprites once every 16 ms
//! (the engine's task period) and stops when none of them is changing.
//!
//! Sprites are placed by their top left corner; scaling is about
//! `scale_at` and turning about `turn_at`, both from that corner, as the
//! game's Direct3D sprites do.

use std::collections::{BTreeMap, VecDeque};

use super::effects::User;

/// Length of a frame: the engine runs a task once more than 15 ms passed.
pub const TICK_MS: u64 = 16;

/// What a keyframe changes.
pub mod flag {
    /// Wait this many frames first.
    pub const WAIT: u32 = 0x1;
    /// Show or hide the sprite (hiding at the end of the keyframe).
    pub const SHOW: u32 = 0x2;
    /// Move from one place to another.
    pub const MOVE: u32 = 0x4;
    /// Fade from one opacity to another.
    pub const FADE: u32 = 0x8;
    /// Wipe in or out through a rule picture.
    pub const WIPE: u32 = 0x100;
    /// Turn from one angle to another (degrees, clockwise).
    pub const TURN: u32 = 0x10_0000;
    /// Scale from one size to another.
    pub const SCALE: u32 = 0x20_0000;
    /// A step of the effect's own each frame.
    pub const USER: u32 = 0x200_0000;
    /// Go on with an earlier keyframe.
    pub const JUMP: u32 = 0x1000_0000;
}

/// One keyframe.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Key {
    pub flags: u32,
    pub frames: i32,
    pub wait: i32,
    pub show: bool,
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub alpha: (u8, u8),
    pub turn: (i32, i32),
    pub turn_at: (f32, f32),
    pub scale: ((f32, f32), (f32, f32)),
    pub scale_at: (f32, f32),
    /// Rule picture (`NN.bin`) and whether the sprite is shown at the end.
    pub wipe: (u32, bool),
    /// Number of the [`User`] step on the stage.
    pub user: u32,
    /// Keyframe to go on with.
    pub jump: usize,
}

impl Key {
    /// A keyframe of `frames` frames that changes nothing yet.
    pub fn new(frames: i32) -> Self {
        Self {
            frames,
            ..Self::default()
        }
    }

    /// Goes on with keyframe `to`, for ever.
    pub fn jump(to: usize) -> Self {
        Self {
            flags: flag::JUMP,
            frames: 1,
            jump: to,
            ..Self::default()
        }
    }

    pub fn wait(mut self, frames: i32) -> Self {
        self.flags |= flag::WAIT;
        self.wait = frames;
        self
    }

    pub fn show(mut self, show: bool) -> Self {
        self.flags |= flag::SHOW;
        self.show = show;
        self
    }

    pub fn moving(mut self, from: (i32, i32), to: (i32, i32)) -> Self {
        self.flags |= flag::MOVE;
        self.from = from;
        self.to = to;
        self
    }

    pub fn fade(mut self, from: u8, to: u8) -> Self {
        self.flags |= flag::FADE;
        self.alpha = (from, to);
        self
    }

    pub fn turning(mut self, from: i32, to: i32, at: (f32, f32)) -> Self {
        self.flags |= flag::TURN;
        self.turn = (from, to);
        self.turn_at = at;
        self
    }

    pub fn scaling(mut self, from: (f32, f32), to: (f32, f32), at: (f32, f32)) -> Self {
        self.flags |= flag::SCALE;
        self.scale = (from, to);
        self.scale_at = at;
        self
    }

    pub fn wiping(mut self, rule: u32, shown: bool) -> Self {
        self.flags |= flag::WIPE;
        self.wipe = (rule, shown);
        self
    }

    pub fn user(mut self, index: u32) -> Self {
        self.flags |= flag::USER;
        self.user = index;
        self
    }
}

/// What a sprite shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Look {
    #[default]
    Nothing,
    /// A picture, or the `part` (left, top, width, height) of one.
    Picture {
        name: String,
        part: Option<(u32, u32, u32, u32)>,
    },
    /// A rectangle in one colour (RGBA).
    Fill { color: [u8; 4], w: u32, h: u32 },
    /// The screen as drawn under the effects.
    Screen,
    /// `source` with its rows moved sideways along a sine wave: row `y`
    /// (from 0) moves right by `sin((phase + step * (y + 1))°) * amp`
    /// pixels, whole pixels towards zero.
    Raster {
        source: Box<Look>,
        amp: f32,
        step: f32,
        phase: f32,
    },
    /// Two looks mixed: `mix` from 0 (all `from`) to 255 (all `to`).
    Mix {
        from: Box<Look>,
        to: Box<Look>,
        mix: u8,
    },
    /// `source` in blocks of `block` pixels, each the average of its
    /// pixels, from the top left.
    Mosaic { source: Box<Look>, block: u32 },
    /// White (RGB) bars (left, top, width, height) on nothing, a screen
    /// big.
    Bars { bars: Vec<[i32; 4]> },
    /// `source` shown through a rule picture (`NN.bin`): a pixel's
    /// opacity is `threshold - rule` at most its own, or, `erasing`, what
    /// is left of it after taking that off.
    Reveal {
        source: Box<Look>,
        rule: u32,
        threshold: f32,
        erasing: bool,
        /// Width of a row of the rule picture when it is not the source's
        /// own (the rule is then a picture of that width whose top left
        /// corner the source's top left corner is laid on); 0 reads the
        /// rule a row of the source at a time.
        stride: u32,
    },
}

impl Look {
    pub fn picture(name: &str) -> Self {
        Self::Picture {
            name: name.to_string(),
            part: None,
        }
    }

    pub fn part(name: &str, part: (u32, u32, u32, u32)) -> Self {
        Self::Picture {
            name: name.to_string(),
            part: Some(part),
        }
    }
}

/// A sprite and where its keyframes are.
#[derive(Debug, Clone, Default)]
pub struct Sprite {
    /// Its number on the stage.
    pub id: u32,
    /// The command that put it there.
    pub owner: u32,
    /// The timeline it was last started in.
    pub group: u32,
    pub look: Look,
    /// Drawing order: higher is in front.
    pub z: i32,
    /// Added to what is under it instead of laid over it.
    pub add: bool,
    pub x: f32,
    pub y: f32,
    pub shown: bool,
    pub alpha: u8,
    pub turn: f32,
    pub turn_at: (f32, f32),
    pub scale: (f32, f32),
    pub scale_at: (f32, f32),
    /// Colour the picture is multiplied by (RGB).
    pub tint: [u8; 3],
    /// Wiping in or out: rule picture, progress (0 to 1) and whether it is
    /// shown at the end.
    pub wipe: Option<(u32, f32, bool)>,
    pub keys: Vec<Key>,
    next: usize,
    /// Changes under way (the keyframe's flags, less the wait once over).
    active: u32,
    frame: i32,
    frames: i32,
    wait: i32,
    key: Key,
}

impl Sprite {
    pub fn new(id: u32, owner: u32, look: Look, z: i32) -> Self {
        Self {
            id,
            owner,
            look,
            z,
            shown: false,
            alpha: 255,
            scale: (1.0, 1.0),
            tint: [255; 3],
            ..Self::default()
        }
    }

    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn adding(mut self) -> Self {
        self.add = true;
        self
    }

    pub fn shown(mut self) -> Self {
        self.shown = true;
        self
    }

    pub fn keys(mut self, keys: Vec<Key>) -> Self {
        self.keys = keys;
        self
    }

    /// Whether it changes now.
    pub fn busy(&self) -> bool {
        self.active != 0
    }
}

/// One step of an effect's program.
#[derive(Debug, Clone)]
pub enum Step {
    /// Puts a sprite on the stage.
    Add(Sprite),
    /// Gives a sprite new keyframes (dropping those it had).
    Keys(u32, Vec<Key>),
    /// Starts a timeline of these sprites.
    Start(u32, Vec<u32>),
    /// Waits until the timeline stops (the script waits with it).
    Wait(u32),
    /// Takes sprites off the stage.
    Remove(Vec<u32>),
    /// Plays the timeline to its end at once and stops it.
    Stop(u32),
    /// Gives a sprite a new look.
    Look(u32, Look),
    /// Draws into the background at this point of the program.
    Paint(super::panels::Paint),
}

/// The effect sprites on screen and the timelines stepping them.
#[derive(Debug, Clone, Default)]
pub struct Stage {
    pub sprites: Vec<Sprite>,
    pub users: BTreeMap<u32, User>,
    /// Timelines running.
    running: Vec<u32>,
    /// Steps not taken yet: they wait for the [`Step::Wait`] in front.
    queue: VecDeque<Step>,
    /// Milliseconds not yet stepped.
    carry: u64,
    next_id: u32,
    /// Sprites, timelines and steps a command comes back to, by command
    /// and place.
    pub slots: BTreeMap<(u32, u32), u32>,
    /// Sprites of an effect built to be started later, with their
    /// keyframes, by command and number.
    pub sets: BTreeMap<(u32, u32), Vec<(u32, Vec<Key>)>>,
    /// Sprites left covering the screen: they go when the background
    /// changes.
    pub until_background: Vec<u32>,
    /// Sprites that are drawn into the background once their keyframes
    /// end, with what is drawn.
    pub to_background: Vec<(u32, super::panels::Paint)>,
    /// What was drawn into the background and is not handed over yet
    /// (see [`Stage::take_painted`]).
    painted: Vec<super::panels::Paint>,
    /// Random numbers for the effects that scatter things.
    pub rng: u32,
    /// Casablanca's effects: they wipe in where Apeiria's fade.
    pub casablanca: bool,
    /// Counts the changes, so that two stages compare equal while
    /// nothing changed.
    version: u64,
}

impl PartialEq for Stage {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version && self.sprites.len() == other.sprites.len()
    }
}

impl Eq for Stage {}

impl Stage {
    /// A number for a new sprite, timeline or step of its own.
    pub fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// Whether nothing is on the stage.
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty() && self.queue.is_empty()
    }

    /// The sprite with this number.
    pub fn sprite(&mut self, id: u32) -> Option<&mut Sprite> {
        self.sprites.iter_mut().find(|s| s.id == id)
    }

    /// Takes off every sprite of an owner (a command), with its
    /// timelines and steps.
    pub fn clear(&mut self, owner: u32) {
        self.version += 1;
        let gone: Vec<u32> = self
            .sprites
            .iter()
            .filter(|s| s.owner == owner)
            .map(|s| s.group)
            .collect();
        self.sprites.retain(|s| s.owner != owner);
        self.users.retain(|_, u| u.owner() != owner);
        self.running
            .retain(|g| !gone.contains(g) || self.sprites.iter().any(|s| s.group == *g));
    }

    /// The background changed: what was to be drawn into the old one is
    /// not drawn.
    pub fn background_changed(&mut self) {
        let ids: Vec<u32> = std::mem::take(&mut self.to_background)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        self.painted.clear();
        let covers = std::mem::take(&mut self.until_background);
        let ids: Vec<u32> = ids.into_iter().chain(covers).collect();
        if ids.is_empty() {
            return;
        }
        self.version += 1;
        self.sprites.retain(|s| !ids.contains(&s.id));
        self.users.retain(|_, u| !ids.contains(&u.sprite()));
    }

    /// What was drawn into the background since the last call.
    pub fn take_painted(&mut self) -> Vec<super::panels::Paint> {
        std::mem::take(&mut self.painted)
    }

    /// Sprites whose keyframes are all over go into the background.
    fn paint_ended(&mut self) {
        if self.to_background.is_empty() {
            return;
        }
        let ended: Vec<u32> = self
            .sprites
            .iter()
            .filter(|s| s.active == 0 && !s.keys.is_empty() && s.next >= s.keys.len())
            .map(|s| s.id)
            .collect();
        let mut i = 0;
        while i < self.to_background.len() {
            if ended.contains(&self.to_background[i].0) {
                let (id, paint) = self.to_background.remove(i);
                self.painted.push(paint);
                self.sprites.retain(|s| s.id != id);
                self.users.retain(|_, u| u.sprite() != id);
            } else {
                i += 1;
            }
        }
    }

    /// Takes up a program of steps, up to its first wait.
    pub fn play(&mut self, steps: Vec<Step>) {
        self.version += 1;
        self.queue.extend(steps);
        self.take_steps();
    }

    /// Milliseconds until the steps queued now are all taken, if within a
    /// minute: how long the script waits.
    pub fn blocking_ms(&self) -> Option<u64> {
        if self.queue.is_empty() {
            return Some(0);
        }
        let mut t = self.clone();
        for n in 0..3750u64 {
            if t.queue.is_empty() {
                return Some(n * TICK_MS);
            }
            t.tick();
        }
        None
    }

    /// Takes every step queued at once, playing waited-for timelines to
    /// their end (skipping, or the wait being over).
    pub fn settle(&mut self) {
        self.version += 1;
        while let Some(step) = self.queue.front().cloned() {
            if let Step::Wait(g) = step {
                if self.running.contains(&g) {
                    self.finish(g);
                }
            }
            self.take_steps();
            if self.queue.is_empty() {
                break;
            }
        }
    }

    fn take_steps(&mut self) {
        self.take_queued();
        self.paint_ended();
    }

    fn take_queued(&mut self) {
        while let Some(step) = self.queue.front().cloned() {
            match step {
                Step::Wait(g) if self.running.contains(&g) => return,
                Step::Wait(_) => {}
                Step::Add(s) => {
                    self.sprites.retain(|o| o.id != s.id);
                    self.sprites.push(s);
                }
                Step::Keys(id, keys) => {
                    if let Some(s) = self.sprite(id) {
                        s.keys = keys;
                        s.next = 0;
                        s.active = 0;
                    }
                }
                Step::Start(g, ids) => {
                    for i in 0..self.sprites.len() {
                        if ids.contains(&self.sprites[i].id) {
                            self.sprites[i].group = g;
                            self.sprites[i].next = 0;
                            self.sprites[i].active = 0;
                            self.load_next(i, false);
                        }
                    }
                    if !self.running.contains(&g) {
                        self.running.push(g);
                    }
                }
                Step::Remove(ids) => {
                    self.sprites.retain(|s| !ids.contains(&s.id));
                    self.users.retain(|_, u| !ids.contains(&u.sprite()));
                }
                Step::Stop(g) => self.finish(g),
                Step::Look(id, look) => {
                    if let Some(s) = self.sprite(id) {
                        s.look = look;
                    }
                }
                Step::Paint(paint) => self.painted.push(paint),
            }
            self.queue.pop_front();
        }
    }

    /// Steps for `ms` milliseconds. True when anything may have changed.
    pub fn advance(&mut self, ms: u64) -> bool {
        if self.running.is_empty() && self.queue.is_empty() {
            self.carry = 0;
            return false;
        }
        self.carry += ms;
        let mut stepped = false;
        while self.carry >= TICK_MS {
            self.carry -= TICK_MS;
            self.tick();
            stepped = true;
        }
        stepped
    }

    /// Plays a timeline to the end of every sprite's keyframes, without
    /// the jumps, and stops it.
    fn finish(&mut self, g: u32) {
        for i in 0..self.sprites.len() {
            if self.sprites[i].group != g {
                continue;
            }
            loop {
                if self.sprites[i].active != 0 {
                    self.end_key(i);
                }
                if self.sprites[i].next >= self.sprites[i].keys.len() {
                    break;
                }
                self.load_next(i, true);
            }
        }
        self.running.retain(|&r| r != g);
    }

    fn tick(&mut self) {
        self.version += 1;
        for g in self.running.clone() {
            let mut any = false;
            for i in 0..self.sprites.len() {
                let s = &self.sprites[i];
                if s.group == g && s.active != 0 && s.shown {
                    self.step_sprite(i);
                    any = true;
                }
            }
            if !any {
                self.running.retain(|&r| r != g);
            }
        }
        self.take_steps();
    }

    fn step_sprite(&mut self, i: usize) {
        if self.sprites[i].active & flag::WAIT != 0 {
            self.step(i);
            if self.sprites[i].active & flag::WAIT != 0 {
                return;
            }
        }
        if self.sprites[i].frames <= self.sprites[i].frame {
            self.end_key(i);
            self.load_next(i, false);
        }
        self.step(i);
    }

    /// Runs a user step on the sprites.
    fn with_user(&mut self, u: u32, f: impl FnOnce(&mut User, &mut Vec<Sprite>, &mut u32)) {
        if let Some(mut step) = self.users.remove(&u) {
            f(&mut step, &mut self.sprites, &mut self.rng);
            self.users.insert(u, step);
        }
    }

    /// One frame of the keyframe under way.
    fn step(&mut self, i: usize) {
        let s = &mut self.sprites[i];
        if s.active & flag::WAIT != 0 {
            if s.wait > 0 {
                s.wait -= 1;
                return;
            }
            s.active &= !flag::WAIT;
        }
        let (f, n) = (s.frame, s.frames.max(1));
        let k = &s.key;
        if s.active & flag::MOVE != 0 {
            s.x = ((k.to.0 - k.from.0) * f / n + k.from.0) as f32;
            s.y = ((k.to.1 - k.from.1) * f / n + k.from.1) as f32;
        }
        if s.active & flag::FADE != 0 {
            let (a, b) = (k.alpha.0 as i32, k.alpha.1 as i32);
            s.alpha = ((b - a) * f / n + a) as u8;
        }
        if s.active & flag::TURN != 0 {
            s.turn = (k.turn.1 - k.turn.0) as f32 * f as f32 / n as f32 + k.turn.0 as f32;
        }
        if s.active & flag::SCALE != 0 {
            let ((ax, ay), (bx, by)) = k.scale;
            let t = f as f32 / n as f32;
            s.scale = ((bx - ax) * t + ax, (by - ay) * t + ay);
        }
        if s.active & flag::WIPE != 0 {
            s.wipe = Some((k.wipe.0, f as f32 / n as f32, k.wipe.1));
        }
        let user = (s.active & flag::USER != 0).then_some(k.user);
        s.frame += 1;
        if let Some(u) = user {
            self.with_user(u, |step, sprites, rng| step.tick(sprites, rng, f, n));
        }
    }

    /// Sets the end values of the keyframe under way.
    fn end_key(&mut self, i: usize) {
        let s = &mut self.sprites[i];
        let k = s.key.clone();
        if s.active & flag::WAIT != 0 {
            s.wait = 0;
        }
        if s.active & flag::MOVE != 0 {
            s.x = k.to.0 as f32;
            s.y = k.to.1 as f32;
        }
        if s.active & flag::SHOW != 0 {
            s.shown = k.show;
        }
        if s.active & flag::FADE != 0 {
            s.alpha = k.alpha.1;
        }
        if s.active & flag::TURN != 0 {
            s.turn = k.turn.1 as f32;
        }
        if s.active & flag::SCALE != 0 {
            s.scale = k.scale.1;
        }
        if s.active & flag::WIPE != 0 {
            s.wipe = None;
            s.shown = k.wipe.1;
        }
        let user = (s.active & flag::USER != 0).then_some(k.user);
        s.active = 0;
        if let Some(u) = user {
            self.with_user(u, |step, sprites, _| step.finish(sprites));
        }
    }

    /// Takes up the sprite's next keyframe; `no_jump` passes over jumps
    /// (when finishing).
    fn load_next(&mut self, i: usize, no_jump: bool) {
        let s = &mut self.sprites[i];
        let Some(mut k) = s.keys.get(s.next).cloned() else {
            return;
        };
        if k.flags & flag::JUMP != 0 {
            if no_jump {
                s.next = s.keys.len();
                return;
            }
            s.next = k.jump;
            match s.keys.get(s.next) {
                Some(j) => k = j.clone(),
                None => return,
            }
        }
        s.next += 1;
        let f = k.flags & !flag::JUMP;
        s.active |= f;
        if f & flag::WAIT != 0 {
            s.wait = k.wait;
        }
        if f & flag::TURN != 0 {
            s.turn_at = k.turn_at;
        }
        if f & flag::SCALE != 0 {
            s.scale_at = k.scale_at;
        }
        if k.flags == 0 {
            s.active = 0;
        } else {
            s.frame = 1;
            s.frames = k.frames.max(1);
        }
        // Start values.
        if f & flag::MOVE != 0 {
            s.x = k.from.0 as f32;
            s.y = k.from.1 as f32;
        }
        if f & flag::SHOW != 0 && k.show {
            s.shown = true;
        }
        if f & flag::FADE != 0 {
            s.alpha = k.alpha.0;
        }
        if f & flag::TURN != 0 {
            s.turn = k.turn.0 as f32;
        }
        if f & flag::SCALE != 0 {
            s.scale = k.scale.0;
        }
        if f & flag::WIPE != 0 {
            s.shown = true;
            s.wipe = Some((k.wipe.0, 0.0, k.wipe.1));
        }
        let user = (f & flag::USER != 0).then_some(k.user);
        s.key = k;
        if let Some(u) = user {
            self.with_user(u, |step, sprites, _| step.start(sprites));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(keys: Vec<Key>) -> Stage {
        let mut st = Stage::default();
        st.play(vec![
            Step::Add(Sprite::new(1, 0, Look::Nothing, 0)),
            Step::Keys(1, keys),
            Step::Start(2, vec![1]),
            Step::Wait(2),
        ]);
        st
    }

    #[test]
    fn fades_over_its_frames_then_stops() {
        let mut st = stage(vec![Key::new(4).show(true).fade(0, 200)]);
        assert_eq!(st.blocking_ms(), Some(5 * TICK_MS));
        assert!(st.sprites[0].shown);
        assert_eq!(st.sprites[0].alpha, 0);
        let mut seen = vec![];
        for _ in 0..6 {
            st.advance(TICK_MS);
            seen.push(st.sprites[0].alpha);
        }
        assert_eq!(seen, [50, 100, 150, 200, 200, 200]);
        assert_eq!(st.blocking_ms(), Some(0));
    }

    #[test]
    fn loops_for_ever_until_settled() {
        let mut st = stage(vec![
            Key::new(2).show(true).fade(0, 255),
            Key::new(2).fade(255, 0),
            Key::jump(0),
        ]);
        assert_eq!(st.blocking_ms(), None);
        st.settle();
        assert_eq!(st.blocking_ms(), Some(0));
        assert_eq!(st.sprites[0].alpha, 0);
    }

    #[test]
    fn hides_at_the_end_and_waits_first() {
        let mut st = stage(vec![
            Key::new(1).show(true),
            Key::new(2).wait(3).show(false),
        ]);
        let mut shown = vec![];
        for _ in 0..8 {
            st.advance(TICK_MS);
            shown.push(st.sprites[0].shown);
        }
        // Three frames of waiting; the wait's last frame is the first of
        // the keyframe's two.
        assert_eq!(shown[..5], [true, true, true, false, false]);
    }

    #[test]
    fn steps_after_a_wait_come_when_it_ends() {
        let mut st = stage(vec![Key::new(3).show(true).fade(0, 255)]);
        st.play(vec![Step::Remove(vec![1])]);
        assert_eq!(st.sprites.len(), 1);
        st.advance(10 * TICK_MS);
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn a_sprite_goes_into_the_background_as_it_ends() {
        use super::super::panels::Paint;
        let mut st = stage(vec![Key::new(3).show(true).fade(0, 255)]);
        st.to_background.push((1, Paint::Fill(0x123456)));
        st.advance(2 * TICK_MS);
        assert_eq!(st.sprites.len(), 1);
        assert!(st.take_painted().is_empty());
        st.advance(10 * TICK_MS);
        assert!(st.sprites.is_empty());
        assert_eq!(st.take_painted(), [Paint::Fill(0x123456)]);
        assert!(st.take_painted().is_empty());
    }

    #[test]
    fn a_new_background_drops_what_was_not_drawn_yet() {
        use super::super::panels::Paint;
        let mut st = stage(vec![Key::new(30).show(true).fade(0, 255)]);
        st.to_background.push((1, Paint::Fill(0)));
        st.background_changed();
        assert!(st.sprites.is_empty());
        st.settle();
        assert!(st.take_painted().is_empty());
    }
}
