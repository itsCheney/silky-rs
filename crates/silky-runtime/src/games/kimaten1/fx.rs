//! きまぐれテンプテーション's screen effects (`Game.1001`) and its quick
//! flashes and fades (`Game.1002`), played on the sprite stage of
//! 景の海のアペイリア (the same engine build).
//!
//! `Game.1001 (type, action)` starts (0) or stops (1, 2) a looping effect:
//!
//! | type | what |
//! |------|------|
//! | 0, 1, 4 | specks of soot drifting up (1 adds a dark haze wiped in over the screen, 4 uses the other tile) |
//! | 2, 3 | the screen darkens and lifts over and over; 3 also lays two waving copies of the screen over it |
//! | 5 | a red room with three small lights that bob |
//! | 6 | the three lights swell over the screen (needs 5) |
//! | 7 | the screen turned and zoomed, with streaks over it |
//! | 8 | a short picture film that scales and wipes in and out |
//!
//! The effects of the original keep their own random generator (an
//! MT19937 asked for numbers by rejection sampling); it is ported here and
//! seeded from the stage, so a run always plays the same.

use crate::games::apeiria::anim::{Key, Look, Sprite, Stage, Step};
use crate::games::apeiria::effects::User;
use crate::games::apeiria::panels::Paint;

/// Owner of the sprites of `Game.1001`.
pub const FX: u32 = 1001;
/// Owner of the sprites of `Game.1002`.
pub const FLASH: u32 = 1002;

/// MT19937 as the effects' generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mt {
    state: Vec<u32>,
    at: usize,
}

impl Mt {
    pub fn new(seed: u32) -> Self {
        let mut state = vec![0u32; 624];
        state[0] = seed;
        for i in 1..624 {
            let p = state[i - 1];
            state[i] = (p ^ (p >> 30))
                .wrapping_mul(1_812_433_253)
                .wrapping_add(i as u32);
        }
        Self { state, at: 624 }
    }

    fn twist(&mut self) {
        for i in 0..624 {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
            let mut v = self.state[(i + 397) % 624] ^ (y >> 1);
            if y & 1 != 0 {
                v ^= 0x9908_b0df;
            }
            self.state[i] = v;
        }
        self.at = 0;
    }

    pub fn next(&mut self) -> u32 {
        if self.at >= 624 {
            self.twist();
        }
        let mut y = self.state[self.at];
        self.at += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// A number below `n`, drawn again while it falls in the part of the
    /// range that would favour small numbers.
    pub fn below(&mut self, n: u32) -> u32 {
        if n <= 1 {
            self.next();
            return 0;
        }
        loop {
            let v = self.next();
            let top = u32::MAX;
            if top / n > v / n || top % n == n - 1 {
                return v % n;
            }
        }
    }
}

/// The generator every effect starts from the stage's own random number.
fn mt(st: &mut Stage) -> Mt {
    if st.rng == 0 {
        st.rng = 0x2545_f491;
    }
    st.rng ^= st.rng << 13;
    st.rng ^= st.rng >> 17;
    st.rng ^= st.rng << 5;
    Mt::new(st.rng)
}

/// One speck of soot.
#[derive(Debug, Clone, PartialEq)]
struct Speck {
    x: f32,
    y: f32,
    delay: i32,
    age: i32,
    speed: f32,
    amp: f64,
    phase: f64,
}

/// The soot: up to 128 specks, new ones every so many frames.
#[derive(Debug, Clone)]
pub struct Soot {
    mode: i32,
    ids: Vec<u32>,
    slots: Vec<Option<Speck>>,
    counter: i32,
    made: i32,
    rng: Mt,
}

impl Soot {
    fn spawn(&mut self, sprites: &mut [Sprite]) {
        let count = if self.mode == 1 { 40 } else { 4 };
        for _ in 0..count {
            let Some(slot) = self.slots.iter().position(Option::is_none) else {
                break;
            };
            let r = self.rng.below(0xb70a7c3) as i32;
            let x = (r / 0x500 + 1) * 0x500 - r;
            let y = (self.rng.below(0xb70a7c3) % 100) as i32 + 570;
            let late = (self.rng.below(0xb70a7c3) % 100) as i32;
            // The first batch of all starts at once.
            let delay = if self.made == 0 { 0 } else { late };
            let amp = (self.rng.below(0xb70a7c3) % 20 + 11) as f64;
            let phase = (self.rng.below(0xb70a7c3) % 100 + 101) as f64;
            let speed = ((self.rng.below(0xb70a7c3) % 100 + 1) as f32 / 100.0 * 2.0).max(1.0);
            self.slots[slot] = Some(Speck {
                x: x as f32,
                y: y as f32,
                delay,
                age: 0,
                speed,
                amp,
                phase,
            });
            if let Some(s) = sprites.iter_mut().find(|s| s.id == self.ids[slot]) {
                s.x = x as f32;
                s.y = y as f32;
                s.alpha = 0;
                s.shown = true;
            }
            self.made += 1;
        }
        let k = (self.made / 100).max(1) as u32;
        self.counter = (self.rng.below(0xb70a7c3) % (k * 4)) as i32 + 16;
    }

    fn tick(&mut self, sprites: &mut [Sprite]) {
        self.counter -= 1;
        if self.counter == 0 {
            self.spawn(sprites);
        }
        for (slot, id) in self.slots.iter_mut().zip(&self.ids) {
            let Some(p) = slot else {
                continue;
            };
            let Some(s) = sprites.iter_mut().find(|s| s.id == *id) else {
                continue;
            };
            if p.delay > 0 {
                p.delay -= 1;
                continue;
            }
            p.age += 1;
            if p.age < 33 {
                s.alpha = (p.age * 4) as u8;
            }
            p.y -= p.speed;
            if self.mode != 0 {
                p.x += 0.5;
            }
            let wave = ((p.y as f64 + p.phase) * 3.141592 / 180.0).sin();
            s.x = ((wave * p.amp) as f32 + p.x) as i32 as f32;
            s.y = p.y as i32 as f32;
            if p.y < -8.0 || p.x > 1288.0 {
                s.shown = false;
                *slot = None;
            }
        }
    }
}

/// The darkening that lifts again, and the waving copies of the screen.
#[derive(Debug, Clone)]
pub struct Pulse {
    black: u32,
    waves: [u32; 2],
    peak: i32,
    wave: bool,
    counter: i32,
    phase: f32,
}

impl Pulse {
    fn tick(&mut self, sprites: &mut [Sprite], f: i32) {
        let p = self.peak;
        let alpha = if f < 64 {
            p * f / 64
        } else if f < 72 {
            p
        } else if f < 136 {
            p - (f - 72) * p / 64
        } else {
            0
        };
        if let Some(s) = sprites.iter_mut().find(|s| s.id == self.black) {
            s.shown = true;
            s.alpha = alpha.clamp(0, 255) as u8;
        }
        if !self.wave {
            return;
        }
        let a = self.counter.min(64) as u8;
        for id in self.waves.iter() {
            self.phase += 1.0;
            let Some(s) = sprites.iter_mut().find(|s| s.id == *id) else {
                continue;
            };
            s.shown = true;
            s.alpha = a;
            if let Look::Raster { phase, .. } = &mut s.look {
                *phase = self.phase;
            }
        }
        self.counter += 1;
    }
}

/// One of the three lights of the red room.
#[derive(Debug, Clone)]
pub struct Mover {
    sprite: u32,
    state: i32,
    count: i32,
    total: i32,
    delay: i32,
    flag: u32,
    from: (f32, f32),
    to: (f32, f32),
    next: (f32, f32),
    rng: Mt,
}

impl Mover {
    fn step(&mut self, sprites: &mut [Sprite]) {
        if self.delay != 0 {
            self.delay -= 1;
            return;
        }
        self.count += 1;
        let t = self.count as f32 / self.total as f32;
        if let Some(s) = sprites.iter_mut().find(|s| s.id == self.sprite) {
            s.x = ((self.to.0 - self.from.0) * t + self.from.0) as i32 as f32;
            s.y = ((self.to.1 - self.from.1) * t + self.from.1) as i32 as f32;
        }
        if self.count != self.total {
            return;
        }
        match self.state {
            0 => {
                self.count = 0;
                self.delay = 0;
                self.from = self.to;
                self.next = self.to;
                self.total = 32;
                self.flag = self.rng.below(0xb70a7c3);
                let up = self.flag & 1 == 0;
                self.to.1 = self.from.1 + if up { -5.0 } else { 5.0 };
                self.state = 1;
            }
            1 => {
                let r = self.rng.below(0xb70a7c3) as i32;
                self.state = 2;
                self.delay = r % 5 + 5;
                self.from = self.to;
                self.to = self.next;
                self.count = 0;
            }
            2 => {
                self.from = self.to;
                self.to = self.next;
                self.count = 0;
                let up = self.flag & 1 == 0;
                self.to.1 = self.from.1 + if up { 2.0 } else { -2.0 };
                self.state = 3;
            }
            _ => {
                let r = self.rng.below(0xb70a7c3) as i32;
                self.state = 0;
                self.delay = r % 5 + 5;
                self.from = self.to;
                self.to = self.next;
                self.count = 0;
            }
        }
    }
}

/// The picture film of type 8: the next picture scales and wipes in over 6
/// frames and out over 6, one after the other, eight pictures round.
#[derive(Debug, Clone)]
pub struct Film {
    index: u32,
    t: i32,
}

/// What an effect does each frame.
#[derive(Debug, Clone)]
pub enum Kind {
    Soot(Soot),
    Pulse(Pulse),
    Lights(Vec<Mover>),
    Film(Film),
}

/// One effect of `Game.1001` on the stage.
#[derive(Debug, Clone)]
pub struct Fx {
    pub owner: u32,
    pub sprite: u32,
    kind: Kind,
}

impl Fx {
    pub fn finish(&mut self, _sprites: &mut [Sprite]) {}

    pub fn tick(&mut self, sprites: &mut [Sprite], f: i32) {
        match &mut self.kind {
            Kind::Soot(s) => s.tick(sprites),
            Kind::Pulse(p) => p.tick(sprites, f),
            Kind::Lights(m) => {
                for l in m.iter_mut() {
                    l.step(sprites);
                }
            }
            Kind::Film(c) => {
                let Some(s) = sprites.iter_mut().find(|s| s.id == self.sprite) else {
                    return;
                };
                if c.t == 0 {
                    s.look = Look::picture(&format!("eff07_{:02}.akb", c.index & 7));
                    s.scale_at = (640.0, 360.0);
                    s.scale = (1.0, 1.0);
                    s.shown = true;
                }
                let k = c.t;
                if k < 6 {
                    let p = (k + 1) as f32 / 6.0;
                    s.scale = (1.0 + p, 1.0 + p);
                    s.wipe = Some((8, p, true));
                } else {
                    s.scale = (2.0, 2.0);
                    s.wipe = Some((8, (k - 5) as f32 / 6.0, false));
                }
                c.t += 1;
                if c.t == 12 {
                    s.wipe = None;
                    s.shown = false;
                    c.t = 0;
                    c.index += 1;
                }
            }
        }
    }
}

/// Plays steps on the stage, with a helper for sprites.
struct Play<'a> {
    st: &'a mut Stage,
    steps: Vec<Step>,
    owner: u32,
}

impl<'a> Play<'a> {
    fn new(st: &'a mut Stage, owner: u32) -> Self {
        Self {
            st,
            steps: Vec::new(),
            owner,
        }
    }

    /// A sprite at (x, y) on the stage, not shown yet.
    fn add(&mut self, look: Look, z: i32, x: f32, y: f32) -> u32 {
        let id = self.st.id();
        self.steps
            .push(Step::Add(Sprite::new(id, self.owner, look, z).at(x, y)));
        id
    }

    fn add_hidden(&mut self, look: Look, z: i32, scale: f32) -> u32 {
        let id = self.st.id();
        let mut s = Sprite::new(id, self.owner, look, z);
        s.scale = (scale, scale);
        self.steps.push(Step::Add(s));
        id
    }

    fn start(&mut self, sprites: Vec<(u32, Vec<Key>)>) -> u32 {
        let g = self.st.id();
        let ids: Vec<u32> = sprites.iter().map(|(id, _)| *id).collect();
        for (id, keys) in sprites {
            self.steps.push(Step::Keys(id, keys));
        }
        self.steps.push(Step::Start(g, ids));
        g
    }

    /// An effect that is stepped every frame, for ever, by `kind`.
    fn run(&mut self, driver: u32, kind: Kind, frames: i32) {
        let u = self.st.id();
        self.st.users.insert(
            u,
            User::K1(Box::new(Fx {
                owner: self.owner,
                sprite: driver,
                kind,
            })),
        );
        self.start(vec![(
            driver,
            vec![Key::new(frames).show(true).user(u), Key::jump(0)],
        )]);
    }

    fn wait(&mut self, g: u32) {
        self.steps.push(Step::Wait(g));
    }

    fn done(self) {
        self.st.play(self.steps);
    }
}

fn black() -> Look {
    Look::Fill {
        color: [0, 0, 0, 255],
        w: 1280,
        h: 720,
    }
}

/// `Game.1001 (type, action)`: action 0 starts the effect (an effect that
/// is running goes first, except that 3 on top of 2 only adds its
/// copies), 1 and 2 stop it.
pub fn effect(st: &mut Stage, ty: i32, action: i32) {
    if action != 0 {
        st.clear(FX);
        return;
    }
    if ty == 3 {
        for u in st.users.values_mut() {
            if let User::K1(k) = u {
                if let Kind::Pulse(p) = &mut k.kind {
                    p.wave = true;
                    return;
                }
            }
        }
    }
    if ty == 6 {
        swell(st);
        return;
    }
    st.clear(FX);
    let mut p = Play::new(st, FX);
    let driver = p.add(Look::Nothing, 0, 0.0, 0.0);
    match ty {
        0 | 1 | 4 => {
            let mode = if ty == 4 { 2 } else { ty };
            let tile = if mode == 2 { 8 } else { 0 };
            let ids: Vec<u32> = (0..128)
                .map(|_| p.add_hidden(Look::part("soot.akb", (0, tile, 8, 8)), 600, 4.0))
                .collect();
            if mode == 1 {
                let haze = p.add(Look::picture("soot2.akb"), 599, 0.0, 0.0);
                p.start(vec![(haze, vec![Key::new(255).wiping(61, true)])]);
            }
            let rng = mt(p.st);
            p.run(
                driver,
                Kind::Soot(Soot {
                    mode,
                    slots: vec![None; 128],
                    ids,
                    counter: 1,
                    made: 0,
                    rng,
                }),
                1,
            );
        }
        2 | 3 => {
            let rng = &mut mt(p.st);
            let peak = [128, 64, 24][rng.below(3) as usize];
            let black = p.add(black(), 595, 0.0, 0.0);
            let waves = [-24.0, 24.0].map(|amp| {
                let id = p.st.id();
                let step = if amp < 0.0 { 1.0 } else { 2.0 };
                let look = Look::Raster {
                    source: Box::new(Look::Screen),
                    amp,
                    step,
                    phase: 0.0,
                };
                p.steps
                    .push(Step::Add(Sprite::new(id, FX, look, 596).at(0.0, 0.0)));
                id
            });
            p.run(
                driver,
                Kind::Pulse(Pulse {
                    black,
                    waves,
                    peak,
                    wave: ty == 3,
                    counter: 0,
                    phase: 0.0,
                }),
                136,
            );
        }
        5 => {
            let bg = p.add(Look::picture("bg005g.akb"), 1, 0.0, 0.0);
            p.steps.push(Step::Keys(bg, vec![Key::new(1).show(true)]));
            let g = p.st.id();
            p.steps.push(Step::Start(g, vec![bg]));
            let starts = [(597.0, 396.0), (663.0, 401.0), (633.0, 419.0)];
            let goals = [(512.0, 366.0), (768.0, 366.0), (640.0, 366.0)];
            let mut lights = Vec::new();
            for i in 0..3 {
                let name = format!("bg005g_{i:02}.akb");
                let id = p.add(Look::picture(&name), 2, starts[i].0, starts[i].1);
                p.steps.push(Step::Keys(id, vec![Key::new(1).show(true)]));
                let g = p.st.id();
                p.steps.push(Step::Start(g, vec![id]));
                let rng = mt(p.st);
                lights.push(Mover {
                    sprite: id,
                    state: 0,
                    count: 0,
                    total: 200,
                    delay: 0,
                    flag: 0,
                    from: starts[i],
                    to: goals[i],
                    next: (0.0, 0.0),
                    rng,
                });
            }
            p.run(driver, Kind::Lights(lights), 1);
        }
        7 => {
            // The screen as it is, turned and doubled in size, with
            // streaks over it that go round.
            let mut screen = Sprite::new(p.st.id(), FX, Look::Screen, 590);
            screen.x = -180.0;
            screen.y = 240.0;
            screen.scale = (2.0, 2.0);
            screen.scale_at = (910.0, 218.0);
            screen.turn = -35.0;
            screen.turn_at = (910.0, 218.0);
            screen.shown = true;
            p.steps.push(Step::Add(screen));
            let lines = p.add(Look::picture("eff04.akb"), 591, 0.0, 0.0);
            p.start(vec![(
                lines,
                vec![
                    Key::new(1).show(true),
                    Key::new(324).turning(0, 324, (640.0, 360.0)),
                    Key::jump(0),
                ],
            )]);
        }
        8 => {
            let rng = &mut mt(p.st);
            let index = rng.below(8);
            let film = p.add(Look::Nothing, 592, 0.0, 0.0);
            p.run(film, Kind::Film(Film { index, t: 0 }), 1);
        }
        _ => {}
    }
    p.done();
}

/// Type 6: the three lights of the red room swell to 48 times their size
/// about their middle over 24 frames, and stay.
fn swell(st: &mut Stage) {
    let ids: Vec<u32> = st
        .users
        .values()
        .filter_map(|u| match u {
            User::K1(k) => match &k.kind {
                Kind::Lights(m) => Some(m.iter().map(|l| l.sprite).collect::<Vec<_>>()),
                _ => None,
            },
            _ => None,
        })
        .flatten()
        .collect();
    if ids.is_empty() {
        return;
    }
    let mut p = Play::new(st, FX);
    let keys = vec![Key::new(24).scaling((1.0, 1.0), (48.0, 48.0), (32.0, 32.0))];
    let g = p.start(ids.iter().map(|&i| (i, keys.clone())).collect());
    p.wait(g);
    p.done();
}

/// A flash picture that goes through a rule picture in and out again.
fn flash_through(p: &mut Play, name: &str, rule_in: u32, rule_out: u32, frames: (i32, i32)) -> u32 {
    let id = p.add(Look::picture(name), 700, 0.0, 0.0);
    let g = p.start(vec![(
        id,
        vec![
            Key::new(frames.0).wiping(rule_in, true),
            Key::new(frames.1).wiping(rule_out, false),
        ],
    )]);
    p.wait(g);
    p.steps.push(Step::Remove(vec![id]));
    g
}

/// A picture that fades in over `frames` frames and then goes into the
/// background.
fn fade_into_background(p: &mut Play, name: &str, frames: i32) {
    let id = p.add(Look::picture(name), 700, 0.0, 0.0);
    let g = p.start(vec![(id, vec![Key::new(frames).show(true).fade(0, 255)])]);
    p.wait(g);
    p.st.to_background
        .push((id, Paint::Picture(name.to_string())));
}

/// `Game.1002 (arg, sub)`: the flashes and fades. The script waits for
/// all of them.
pub fn flash(st: &mut Stage, sub: i32, arg: i32) {
    st.clear(FLASH);
    let mut p = Play::new(st, FLASH);
    match sub {
        // A strip of text shown with a black screen behind it.
        1 => {
            let back = p.add(black(), 700, 0.0, 0.0);
            let text = p.add(Look::picture("st00.akb"), 701, 418.0, 339.0);
            let mut keys = Vec::new();
            for _ in 0..2 {
                keys.push(vec![
                    Key::new(32).show(true).fade(0, 255),
                    Key::new(96),
                    Key::new(32).show(false).fade(255, 0),
                ]);
            }
            let g = p.start(vec![(back, keys[0].clone()), (text, keys[1].clone())]);
            p.wait(g);
            p.steps.push(Step::Remove(vec![back, text]));
        }
        2 => match arg {
            0 => {
                flash_through(&mut p, "eff00.akb", 20, 20, (6, 6));
            }
            1 => {
                flash_through(&mut p, "eff02.akb", 305, 305, (6, 6));
            }
            2 => {
                let id = p.add(Look::picture("eff03.akb"), 700, 0.0, 0.0);
                let g = p.start(vec![(
                    id,
                    vec![Key::new(8).wait(2).show(false).fade(0, 255).scaling(
                        (1.0, 1.0),
                        (4.0, 4.0),
                        (640.0, 360.0),
                    )],
                )]);
                p.wait(g);
                p.steps.push(Step::Remove(vec![id]));
            }
            _ => {
                let id = p.add(Look::picture("eff05.akb"), 700, 0.0, 60.0);
                let g = p.start(vec![(
                    id,
                    vec![
                        Key::new(6).wiping(19, true),
                        Key::new(24).wait(6).wiping(19, false),
                    ],
                )]);
                p.wait(g);
                p.steps.push(Step::Remove(vec![id]));
            }
        },
        // The screen zooms back from a fifth too big.
        3 => {
            let id = p.add(Look::Screen, 700, 0.0, 0.0);
            let g = p.start(vec![(
                id,
                vec![Key::new(8).show(false).fade(0, 255).scaling(
                    (1.2, 1.2),
                    (1.0, 1.0),
                    (640.0, 360.0),
                )],
            )]);
            p.wait(g);
            p.steps.push(Step::Remove(vec![id]));
        }
        // A flash, then a picture fades in and becomes the background.
        4 => {
            match arg {
                0 => {
                    flash_through(&mut p, "eff02.akb", 305, 35, (4, 6));
                    fade_into_background(&mut p, "HG011100_02.akb", 64);
                }
                1 => {
                    flash_through(&mut p, "eff08.akb", 311, 34, (4, 6));
                    fade_into_background(&mut p, "HG011100_03.akb", 64);
                }
                _ => {
                    flash_through(&mut p, "eff08.akb", 311, 34, (4, 6));
                }
            };
        }
        5 => {
            flash_through(&mut p, "eff08.akb", 311, 34, (4, 6));
            fade_into_background(&mut p, "eff09.akb", 64);
        }
        6 => {
            let first = p.add(Look::picture("HG011400_01.akb"), 700, 0.0, 0.0);
            let g = p.start(vec![(first, vec![Key::new(128).show(true).fade(0, 255)])]);
            p.wait(g);
            let second = p.add(Look::picture("HG011400_02.akb"), 701, 0.0, 0.0);
            let g = p.start(vec![(
                second,
                vec![Key::new(12).wait(160).wiping(45, true)],
            )]);
            p.wait(g);
            p.steps.push(Step::Remove(vec![first]));
            p.st.to_background
                .push((second, Paint::Picture("HG011400_02.akb".to_string())));
        }
        _ => {}
    }
    p.done();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(st: &mut Stage, frames: u64) {
        for _ in 0..frames {
            st.advance(16);
        }
    }

    #[test]
    fn mt_matches_the_reference_sequence() {
        let mut m = Mt::new(5489);
        assert_eq!(m.next(), 3_499_211_612);
        assert_eq!(m.next(), 581_869_302);
    }

    #[test]
    fn below_stays_in_range() {
        let mut m = Mt::new(1);
        for n in [1u32, 2, 7, 100, 0xb70a7c3] {
            for _ in 0..50 {
                assert!(m.below(n) < n);
            }
        }
    }

    #[test]
    fn soot_rises_and_stops() {
        let mut st = Stage::default();
        effect(&mut st, 0, 0);
        run(&mut st, 400);
        let shown = st.sprites.iter().filter(|s| s.z == 600 && s.shown).count();
        assert!(shown > 0, "no soot");
        assert!(st
            .sprites
            .iter()
            .filter(|s| s.z == 600 && s.shown)
            .all(|s| s.y < 720.0));
        effect(&mut st, 0, 1);
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn soot_is_the_same_every_run() {
        let positions = || {
            let mut st = Stage::default();
            effect(&mut st, 1, 0);
            run(&mut st, 300);
            st.sprites
                .iter()
                .filter(|s| s.z == 600 && s.shown)
                .map(|s| (s.x as i32, s.y as i32))
                .collect::<Vec<_>>()
        };
        assert_eq!(positions(), positions());
    }

    #[test]
    fn the_darkening_pulses_and_the_waves_join_later() {
        let mut st = Stage::default();
        effect(&mut st, 2, 0);
        run(&mut st, 64);
        let dark = st.sprites.iter().find(|s| s.z == 595).unwrap().alpha;
        assert!(dark > 0);
        assert!(st.sprites.iter().filter(|s| s.z == 596).all(|s| !s.shown));
        effect(&mut st, 3, 0);
        run(&mut st, 10);
        assert!(st.sprites.iter().filter(|s| s.z == 596).all(|s| s.shown));
    }

    #[test]
    fn the_lights_bob_around_their_places() {
        let mut st = Stage::default();
        effect(&mut st, 5, 0);
        run(&mut st, 260);
        let l = st
            .sprites
            .iter()
            .find(|s| s.z == 2 && (s.x - 512.0).abs() < 10.0)
            .unwrap();
        assert!((l.y - 366.0).abs() <= 6.0);
    }

    #[test]
    fn a_flash_blocks_the_script() {
        let mut st = Stage::default();
        flash(&mut st, 2, 0);
        assert_eq!(st.blocking_ms(), Some(12 * 16));
    }
}
