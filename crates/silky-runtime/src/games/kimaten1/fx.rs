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

use crate::games::apeiria::anim::{flag, Key, Look, Sprite, Stage, Step};
use crate::games::apeiria::effects::User;

/// Owner of the sprites of `Game.1001`.
pub const FX: u32 = 1001;
/// Owner of the sprites of `Game.1002`.
pub const FLASH: u32 = 1002;

/// Effects at this depth or below come between the standing models (the
/// models lie at depths 0 to 3, the one in the middle on top); the rest
/// are drawn over all of them.
pub const LOW: i32 = 3;

/// Owner of the red room of type 5. It is the displayed screen in the
/// original, so the stop of the effect leaves it and only a copy into the
/// screen (sub 4, 5 or 6 of [`FLASH`]) takes it away.
pub const ROOM: u32 = 1003;

/// The base background picture as the original copies it once, when an
/// effect starts: the picture only, with no overlays, models, text window
/// or effect sprites (black when there is none).
fn frozen(bg: Option<&str>) -> Look {
    match bg {
        Some(name) => Look::picture(name),
        None => black(),
    }
}

/// Whether an effect sprite is drawn among the models.
pub fn is_low(s: &Sprite) -> bool {
    (s.owner == FX || s.owner == FLASH || s.owner == ROOM) && s.z <= LOW
}

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

/// How dark the screen gets at the top of a lap of the darkening.
const PEAKS: [i32; 3] = [128, 64, 24];

/// The darkening that lifts again, and the waving copies of the screen.
/// Every lap (136 frames) starts over and picks how dark it gets anew, so
/// the laps differ in depth.
#[derive(Debug, Clone)]
pub struct Pulse {
    black: u32,
    waves: [u32; 2],
    peak: i32,
    wave: bool,
    counter: i32,
    phase: f32,
    rng: Mt,
}

impl Pulse {
    fn tick(&mut self, sprites: &mut [Sprite], f: i32) {
        // The first frame of a lap (the stage counts a key's frames from 1).
        if f <= 1 {
            self.peak = PEAKS[self.rng.below(3) as usize];
        }
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
        // One accumulator adds each wave's own step in turn (1 for the
        // first copy, 2 for the second), so the pair advances 3 a frame.
        for (id, step) in self.waves.iter().zip([1.0, 2.0]) {
            self.phase += step;
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

/// Frames the streaks of type 7 take to turn once.
pub const TURN_FRAMES: i32 = 10;

/// The picture film of type 8: the next picture scales and wipes in over 6
/// frames and out over 6, one after the other, eight pictures round.
#[derive(Debug, Clone)]
pub struct Film {
    index: u32,
    t: i32,
}

/// Frames the `i`th piece of the text strip takes to fade in: 32 over
/// the number of the piece, at least 1.
pub fn piece_frames(i: usize) -> i32 {
    (32 / (i as i32 + 1)).max(1)
}

/// Size of the text strip picture.
const STRIP: (f32, f32) = (444.0, 41.0);

/// The text strip of `Game.1002` sub 1, laid over the screen piece by
/// piece: each of the 255 pieces is the whole strip, turned a little, put
/// somewhere new and faded in, and the next one starts when it is in.
#[derive(Debug, Clone)]
pub struct Pieces {
    ids: Vec<u32>,
    next: usize,
    left: i32,
    frames: i32,
    cur: usize,
    pattern: i32,
    rng: Mt,
}

impl Pieces {
    fn tick(&mut self, sprites: &mut [Sprite]) {
        if self.left == 0 && self.next < self.ids.len() {
            let i = self.next;
            self.next += 1;
            let (r, r2) = (
                self.rng.below(0xb70a7c3) as i32,
                self.rng.below(0xb70a7c3) as i32,
            );
            let (cx, cy) = match self.pattern % 5 {
                0 | 2 => (640 - r % 640, 360 - r2 % 360),
                3 => (1280 - r % 640, 360 - r2 % 360),
                1 => (1280 - r % 640, 720 - r2 % 360),
                _ => (640 - r % 640, 720 - r2 % 360),
            };
            self.pattern += 1;
            let turn = (self.rng.below(0xb70a7c3) % 44) as f32 - 22.0;
            let big = self.rng.below(0xb70a7c3) % 100 > 50;
            let scale = if big { 1.2 } else { 1.0 };
            if let Some(s) = sprites.iter_mut().find(|s| s.id == self.ids[i]) {
                s.x = cx as f32 - STRIP.0 / 2.0;
                s.y = cy as f32 - STRIP.1 / 2.0;
                s.turn = turn;
                s.scale = (scale, scale);
                s.alpha = 0;
                s.shown = true;
            }
            self.cur = i;
            self.frames = piece_frames(i);
            self.left = self.frames;
        }
        if self.left > 0 {
            let done = self.frames - self.left + 1;
            if let Some(s) = sprites.iter_mut().find(|s| s.id == self.ids[self.cur]) {
                s.alpha = (255 * done / self.frames) as u8;
            }
            self.left -= 1;
        }
    }
}

/// What an effect does each frame.
#[derive(Debug, Clone)]
pub enum Kind {
    Pieces(Pieces),
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
            Kind::Pieces(p) => p.tick(sprites),
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

    /// Like [`Play::add`], laid on the picture under it by adding.
    fn add_lit(&mut self, look: Look, z: i32, x: f32, y: f32) -> u32 {
        let id = self.st.id();
        self.steps.push(Step::Add(
            Sprite::new(id, self.owner, look, z).at(x, y).adding(),
        ));
        id
    }

    fn add_hidden(&mut self, look: Look, z: i32, scale: f32) -> u32 {
        let id = self.st.id();
        let mut s = Sprite::new(id, self.owner, look, z);
        s.scale = (scale, scale);
        self.steps.push(Step::Add(s));
        id
    }

    /// Shows a sprite added in these steps from the start (a timeline
    /// only steps shown sprites).
    fn shown(&mut self, id: u32) {
        for step in self.steps.iter_mut() {
            if let Step::Add(s) = step {
                if s.id == id {
                    s.shown = true;
                }
            }
        }
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
        self.run_keys(
            driver,
            kind,
            vec![Key::new(frames).show(true), Key::jump(0)],
        );
    }

    /// An effect stepped by the user key of `keys`.
    fn run_keys(&mut self, driver: u32, kind: Kind, mut keys: Vec<Key>) {
        let u = self.st.id();
        keys[0] = keys[0].clone().user(u);
        self.st.users.insert(
            u,
            User::K1(Box::new(Fx {
                owner: self.owner,
                sprite: driver,
                kind,
            })),
        );
        self.start(vec![(driver, keys)]);
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

/// Action 2: the effects finish the lap they are on (the keys that go back
/// are no longer taken), the script waits for that, and then they are
/// taken off like with action 1.
fn wind_down(st: &mut Stage) {
    let mut groups: Vec<u32> = Vec::new();
    let mut ids = Vec::new();
    for s in st.sprites.iter_mut().filter(|s| s.owner == FX) {
        for k in s.keys.iter_mut() {
            if k.flags & flag::JUMP != 0 {
                *k = Key::new(1);
            }
        }
        ids.push(s.id);
        if s.group != 0 && !groups.contains(&s.group) {
            groups.push(s.group);
        }
    }
    if ids.is_empty() {
        return;
    }
    let mut steps: Vec<Step> = groups.into_iter().map(Step::Wait).collect();
    steps.push(Step::Remove(ids));
    st.play(steps);
}

/// `Game.1001 (type, action)`: action 0 starts the effect (an effect that
/// is running goes first, except that 3 on top of 2 only adds its
/// copies), 1 and 2 stop it.
pub fn effect(st: &mut Stage, ty: i32, action: i32, bg: Option<&str>) {
    if action == 2 {
        wind_down(st);
        return;
    }
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
                .map(|_| p.add_hidden(Look::part("soot.akb", (0, tile, 8, 8)), 2, 1.0))
                .collect();
            if mode == 1 {
                let haze = p.add(Look::picture("soot2.akb"), 1, 0.0, 0.0);
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
            let rng = mt(p.st);
            let black = p.add(black(), 8, 0.0, 0.0);
            let waves = [-24.0, 24.0].map(|amp| {
                let id = p.st.id();
                let step = if amp < 0.0 { 1.0 } else { 2.0 };
                // The first copy lies over the second.
                let z = if amp < 0.0 { 1 } else { 0 };
                let look = Look::Raster {
                    source: Box::new(frozen(bg)),
                    amp,
                    step,
                    phase: 0.0,
                };
                p.steps
                    .push(Step::Add(Sprite::new(id, FX, look, z).at(0.0, 0.0)));
                id
            });
            p.run(
                driver,
                Kind::Pulse(Pulse {
                    black,
                    waves,
                    peak: 0,
                    wave: ty == 3,
                    counter: 0,
                    phase: 0.0,
                    rng,
                }),
                136,
            );
        }
        5 => {
            // The room is put in the background in the original; it lies
            // under everything here.
            p.st.clear(ROOM);
            let bg = p.st.id();
            p.steps.push(Step::Add(Sprite::new(
                bg,
                ROOM,
                Look::picture("bg005g.akb"),
                -1,
            )));
            p.steps.push(Step::Keys(bg, vec![Key::new(1).show(true)]));
            let g = p.st.id();
            p.steps.push(Step::Start(g, vec![bg]));
            let starts = [(597.0, 396.0), (663.0, 401.0), (633.0, 419.0)];
            let goals = [(512.0, 366.0), (768.0, 366.0), (640.0, 366.0)];
            let mut lights = Vec::new();
            for i in 0..3 {
                let name = format!("bg005g_{i:02}.akb");
                let id = p.add_lit(Look::picture(&name), 0, starts[i].0, starts[i].1);
                p.steps.push(Step::Keys(id, vec![Key::new(1).show(true)]));
                let g = p.st.id();
                p.steps.push(Step::Start(g, vec![id]));
                // All three lights start from the same generator state.
                let rng = Mt::new(5489);
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
            let mut screen = Sprite::new(p.st.id(), FX, frozen(bg), 0);
            screen.x = -180.0;
            screen.y = 240.0;
            screen.scale = (2.0, 2.0);
            screen.scale_at = (910.0, 218.0);
            screen.turn = -35.0;
            screen.turn_at = (640.0, 360.0);
            screen.shown = true;
            p.steps.push(Step::Add(screen));
            let lines = p.st.id();
            let mut streaks = Sprite::new(lines, FX, Look::picture("eff04.akb"), 1);
            streaks.scale = (2.0, 2.0);
            streaks.scale_at = (640.0, 360.0);
            p.steps.push(Step::Add(streaks));
            // Ten frames to turn once round (0 to 324 degrees), over and over.
            p.start(vec![(
                lines,
                vec![
                    Key::new(TURN_FRAMES)
                        .show(true)
                        .turning(0, 324, (640.0, 360.0)),
                    Key::jump(0),
                ],
            )]);
        }
        8 => {
            let rng = &mut mt(p.st);
            // The third number drawn, as the original does, and only its
            // low three bits count.
            rng.below(0xb70a7c3);
            rng.below(0xb70a7c3);
            let index = rng.below(0xb70a7c3) & 7;
            let film = p.add_lit(Look::Nothing, 2, 0.0, 0.0);
            p.run(film, Kind::Film(Film { index, t: 0 }), 1);
        }
        _ => {}
    }
    p.done();
}

/// Type 6: the three lights of the red room swell to 48 times their size
/// about their middle over 24 frames while they drift away, one after the
/// other 12 frames apart, and vanish. The script waits until the last is
/// gone.
fn swell(st: &mut Stage) {
    let lights: Vec<u32> = st
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
    if lights.is_empty() {
        return;
    }
    let at: Vec<(f32, f32)> = lights
        .iter()
        .map(|id| {
            st.sprites
                .iter()
                .find(|s| s.id == *id)
                .map_or((0.0, 0.0), |s| (s.x, s.y))
        })
        .collect();
    let mut p = Play::new(st, FX);
    let sprites = lights
        .iter()
        .zip(at)
        .enumerate()
        .map(|(i, (&id, (x, y)))| (id, swell_keys(i, (x as i32, y as i32))))
        .collect();
    let g = p.start(sprites);
    p.wait(g);
    p.done();
}

/// How far the lights (left, right, middle) drift while they swell.
const DRIFT: [(i32, i32); 3] = [(500, -100), (-700, -100), (0, -50)];

/// The keys of light `i` of type 6, from its place `at`.
fn swell_keys(i: usize, at: (i32, i32)) -> Vec<Key> {
    let d = DRIFT[i % 3];
    vec![Key::new(24)
        .wait(12 * i as i32)
        .show(false)
        .moving(at, (at.0 + d.0, at.1 + d.1))
        .scaling((1.0, 1.0), (48.0, 48.0), (32.0, 32.0))]
}

/// A flash picture that goes through a rule picture in and out again.
fn flash_through(
    p: &mut Play,
    name: &str,
    z: i32,
    rule_in: u32,
    rule_out: u32,
    frames: (i32, i32),
) -> u32 {
    let id = p.add_lit(Look::picture(name), z, 0.0, 0.0);
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

/// Two pictures wipe in together (`under` through its rule over 4
/// frames, `over` through its rule over 6), the first goes, the base
/// background (what the script's `Game.12.0` just loaded) is copied over
/// the screen, and the second fades out over 64 frames.
fn wipe_then_copy(p: &mut Play, under: (&str, u32), over: (&str, u32), room: &[u32]) {
    let a = p.add_lit(Look::picture(under.0), 0, 0.0, 0.0);
    let b = p.add_lit(Look::picture(over.0), 1, 0.0, 0.0);
    let g = p.start(vec![
        (a, vec![Key::new(4).wiping(under.1, true)]),
        (b, vec![Key::new(6).wiping(over.1, true)]),
    ]);
    p.wait(g);
    // The copy replaces the screen, the red room with it; the background
    // is already the picture that was loaded.
    p.steps.push(Step::Remove(
        std::iter::once(a).chain(room.iter().copied()).collect(),
    ));
    let g = p.start(vec![(b, vec![Key::new(64).show(false).fade(255, 0)])]);
    p.wait(g);
    p.steps.push(Step::Remove(vec![b]));
}

/// The eff05 band that opens a flash of sub 5 (sub 2, argument 3).
fn band(p: &mut Play) {
    let id = p.add_lit(Look::picture("eff05.akb"), 12, 0.0, 60.0);
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

/// `Game.1002 (arg, sub)`: the flashes and fades. The script waits for
/// all of them.
pub fn flash(st: &mut Stage, sub: i32, arg: i32, bg: Option<&str>) {
    st.clear(FLASH);
    let room: Vec<u32> = st
        .sprites
        .iter()
        .filter(|s| s.owner == ROOM)
        .map(|s| s.id)
        .collect();
    let mut p = Play::new(st, FLASH);
    match sub {
        // A black screen and the text strip laid piece by piece. The black
        // covers everything on the stage, the text strips (`Game.1000`)
        // that are still up included, as the original's screen goes dark
        // while they stand.
        1 => {
            let back = p.add(black(), STRIP_DEPTH, 0.0, 0.0);
            let ids: Vec<u32> = (0..255)
                .map(|_| {
                    let id = p.st.id();
                    let mut s = Sprite::new(id, FLASH, Look::picture("st00.akb"), STRIP_DEPTH + 1);
                    s.scale_at = (STRIP.0 / 2.0, STRIP.1 / 2.0);
                    s.turn_at = s.scale_at;
                    p.steps.push(Step::Add(s));
                    id
                })
                .collect();
            let driver = p.add(Look::Nothing, 0, 0.0, 0.0);
            let mut rng = mt(p.st);
            let pattern = (rng.below(0xb70a7c3) % 5) as i32;
            let u = p.st.id();
            p.st.users.insert(
                u,
                User::K1(Box::new(Fx {
                    owner: FLASH,
                    sprite: driver,
                    kind: Kind::Pieces(Pieces {
                        ids: ids.clone(),
                        next: 0,
                        left: 0,
                        frames: 1,
                        cur: 0,
                        pattern,
                        rng,
                    }),
                })),
            );
            let g = p.start(vec![
                (back, vec![Key::new(128).wait(128).show(true).fade(0, 255)]),
                (driver, vec![Key::new(PIECE_FRAMES).show(true).user(u)]),
            ]);
            p.wait(g);
            p.steps.push(Step::Remove(
                ids.into_iter().chain([back, driver]).collect(),
            ));
        }
        2 => match arg {
            0 => {
                flash_through(&mut p, "eff00.akb", 12, 20, 20, (6, 6));
            }
            1 => {
                flash_through(&mut p, "eff02.akb", 12, 305, 305, (6, 6));
            }
            2 => {
                let id = p.add_lit(Look::picture("eff03.akb"), 12, 0.0, 0.0);
                p.shown(id);
                let g = p.start(vec![(
                    id,
                    vec![Key::new(8).wait(2).show(false).fade(255, 0).scaling(
                        (1.0, 1.0),
                        (4.0, 4.0),
                        (640.0, 360.0),
                    )],
                )]);
                p.wait(g);
                p.steps.push(Step::Remove(vec![id]));
            }
            _ => band(&mut p),
        },
        // The screen zooms back from a fifth too big, fading away.
        3 => {
            let id = p.add(frozen(bg), 7, 0.0, 0.0);
            p.shown(id);
            let g = p.start(vec![(
                id,
                vec![Key::new(8).show(false).fade(255, 0).scaling(
                    (1.2, 1.2),
                    (1.0, 1.0),
                    (640.0, 360.0),
                )],
            )]);
            p.wait(g);
            p.steps.push(Step::Remove(vec![id]));
        }
        // Two pictures wipe in; arguments 0 and 1 then keep the second as
        // the background.
        4 => match arg {
            0 => wipe_then_copy(&mut p, ("eff02.akb", 305), ("HG011100_02.akb", 35), &room),
            1 => wipe_then_copy(&mut p, ("eff08.akb", 311), ("HG011100_03.akb", 34), &room),
            _ => {
                let id = p.add_lit(Look::picture("eff08.akb"), 0, 0.0, 0.0);
                let g = p.start(vec![(
                    id,
                    vec![Key::new(4).wiping(311, true), Key::new(6).wiping(34, false)],
                )]);
                p.wait(g);
                p.steps.push(Step::Remove(vec![id]));
            }
        },
        5 => {
            band(&mut p);
            wipe_then_copy(&mut p, ("eff08.akb", 311), ("eff09.akb", 34), &room);
        }
        // Two pictures come in one after the other; then the background
        // copy fades in over them from frame 184 and the pictures go.
        6 => {
            let first = p.add_lit(Look::picture("HG011400_01.akb"), 0, 0.0, 0.0);
            let second = p.add_lit(Look::picture("HG011400_02.akb"), 1, 0.0, 0.0);
            let back = p.add(frozen(bg), 2, 0.0, 0.0);
            let g = p.start(vec![
                (first, vec![Key::new(128).show(true).fade(0, 255)]),
                (second, vec![Key::new(12).wait(160).wiping(45, true)]),
                (back, vec![Key::new(128).wait(184).show(true).fade(0, 255)]),
            ]);
            p.wait(g);
            p.steps.push(Step::Remove(
                [first, second, back].into_iter().chain(room).collect(),
            ));
        }
        _ => {}
    }
    p.done();
}

/// Depth of the black of sub 1: over every standing picture and text strip.
const STRIP_DEPTH: i32 = 500;

/// Frames the text strip of sub 1 takes.
pub const PIECE_FRAMES: i32 = 255;

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
        effect(&mut st, 0, 0, None);
        run(&mut st, 400);
        let shown = st.sprites.iter().filter(|s| s.z == 2 && s.shown).count();
        assert!(shown > 0, "no soot");
        assert!(st
            .sprites
            .iter()
            .filter(|s| s.z == 2 && s.shown)
            .all(|s| s.y < 720.0));
        effect(&mut st, 0, 1, None);
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn soot_is_the_same_every_run() {
        let positions = || {
            let mut st = Stage::default();
            effect(&mut st, 1, 0, None);
            run(&mut st, 300);
            st.sprites
                .iter()
                .filter(|s| s.z == 2 && s.shown)
                .map(|s| (s.x as i32, s.y as i32))
                .collect::<Vec<_>>()
        };
        assert_eq!(positions(), positions());
    }

    #[test]
    fn the_darkening_pulses_and_the_waves_join_later() {
        let mut st = Stage::default();
        effect(&mut st, 2, 0, None);
        run(&mut st, 64);
        let dark = st.sprites.iter().find(|s| s.z == 8).unwrap().alpha;
        assert!(dark > 0);
        let waves = |st: &Stage| {
            st.sprites
                .iter()
                .filter(|s| matches!(s.look, Look::Raster { .. }))
                .map(|s| (s.z, s.shown, s.add))
                .collect::<Vec<_>>()
        };
        assert_eq!(waves(&st), [(1, false, false), (0, false, false)]);
        effect(&mut st, 3, 0, None);
        run(&mut st, 10);
        assert_eq!(waves(&st), [(1, true, false), (0, true, false)]);
    }

    #[test]
    fn every_lap_of_the_darkening_picks_its_depth_anew() {
        let mut st = Stage::default();
        effect(&mut st, 2, 0, None);
        let mut peaks = Vec::new();
        for _ in 0..12 {
            let mut top = 0;
            for _ in 0..136 {
                st.advance(16);
                if let Some(s) = st.sprites.iter().find(|s| s.z == 8) {
                    top = top.max(s.alpha);
                }
            }
            peaks.push(top);
        }
        assert!(peaks.iter().all(|p| [128, 64, 24].contains(p)), "{peaks:?}");
        assert!(peaks.windows(2).any(|w| w[0] != w[1]), "{peaks:?}");
    }

    #[test]
    fn the_black_of_the_strip_lies_over_the_standing_text_strips() {
        let mut st = Stage::default();
        for _ in 0..3 {
            crate::games::kimaten1::effects::strip(&mut st, "bg.akb", [0, 4, 20, 100, 100, 0, 0]);
        }
        run(&mut st, 20);
        flash(&mut st, 1, 0, None);
        run(&mut st, 2);
        let back = st
            .sprites
            .iter()
            .find(|s| matches!(s.look, Look::Fill { .. }) && s.owner == FLASH)
            .unwrap();
        assert!(back.z > LOW);
        assert!(st
            .sprites
            .iter()
            .filter(|s| s.owner == 1000)
            .all(|s| s.z < back.z));
    }

    #[test]
    fn the_lights_bob_around_their_places() {
        let mut st = Stage::default();
        effect(&mut st, 5, 0, None);
        run(&mut st, 260);
        let l = st
            .sprites
            .iter()
            .find(|s| s.add && (s.x - 512.0).abs() < 10.0)
            .unwrap();
        assert!((l.y - 366.0).abs() <= 6.0);
    }

    #[test]
    fn a_flash_blocks_the_script() {
        let mut st = Stage::default();
        flash(&mut st, 2, 0, None);
        assert_eq!(st.blocking_ms(), Some(12 * 16));
    }

    #[test]
    fn the_lights_add_and_the_room_lies_under_everything() {
        let mut st = Stage::default();
        effect(&mut st, 5, 0, None);
        let bg = st.sprites.iter().find(|s| s.z == -1).unwrap();
        assert!(!bg.add && is_low(bg));
        assert_eq!(st.sprites.iter().filter(|s| s.add && s.z == 0).count(), 3);
    }

    #[test]
    fn the_lights_swell_one_after_the_other_and_go() {
        assert_eq!(
            swell_keys(1, (10, 20)),
            vec![Key::new(24)
                .wait(12)
                .show(false)
                .moving((10, 20), (-690, -80))
                .scaling((1.0, 1.0), (48.0, 48.0), (32.0, 32.0))]
        );
        assert_eq!(swell_keys(0, (0, 0))[0].to, (500, -100));
        assert_eq!(swell_keys(2, (0, 0))[0].to, (0, -50));
        let mut st = Stage::default();
        effect(&mut st, 5, 0, None);
        run(&mut st, 10);
        effect(&mut st, 6, 0, None);
        // The last light waits 24 frames, then swells for 24.
        assert_eq!(st.blocking_ms(), Some(48 * 16));
        run(&mut st, 60);
        assert!(st.sprites.iter().filter(|s| s.add).all(|s| !s.shown));
    }

    #[test]
    fn the_streaks_turn_every_ten_frames() {
        let mut st = Stage::default();
        effect(&mut st, 7, 0, None);
        let streaks = st.sprites.iter().find(|s| s.z == 1).unwrap();
        assert_eq!(streaks.scale, (2.0, 2.0));
        assert_eq!(streaks.keys[0].frames, TURN_FRAMES);
        assert_eq!(streaks.keys[0].turn, (0, 324));
        let screen = st.sprites.iter().find(|s| s.z == 0 && s.shown).unwrap();
        assert_eq!(screen.scale_at, (910.0, 218.0));
        assert_eq!(screen.turn_at, (640.0, 360.0));
    }

    #[test]
    fn the_film_is_added_and_runs_twelve_frames_a_picture() {
        let mut st = Stage::default();
        effect(&mut st, 8, 0, None);
        let film = st.sprites.iter().find(|s| s.z == 2).unwrap();
        assert!(film.add);
    }

    #[test]
    fn a_pieces_fade_takes_32_over_its_number() {
        let v: Vec<i32> = (0..17).map(piece_frames).collect();
        assert_eq!(v, [32, 16, 10, 8, 6, 5, 4, 4, 3, 3, 2, 2, 2, 2, 2, 2, 1]);
        assert_eq!(piece_frames(254), 1);
    }

    #[test]
    fn the_strip_is_laid_piece_by_piece_for_255_frames() {
        let mut st = Stage::default();
        flash(&mut st, 1, 0, None);
        assert_eq!(st.blocking_ms(), Some(256 * 16));
        run(&mut st, 40);
        let shown = |st: &Stage| {
            st.sprites
                .iter()
                .filter(|s| matches!(s.look, Look::Picture { .. }) && s.shown)
                .count()
        };
        // The first piece takes 32 frames, the second 16 more.
        assert_eq!(shown(&st), 2);
        run(&mut st, 9);
        assert_eq!(shown(&st), 3);
        run(&mut st, 400);
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn the_flashes_add_and_sub_6_lasts_312_frames() {
        let mut st = Stage::default();
        flash(&mut st, 6, 0, None);
        assert!(st.sprites.iter().filter(|s| s.z < 2).all(|s| s.add));
        assert_eq!(st.blocking_ms(), Some(312 * 16));
        let mut st = Stage::default();
        flash(&mut st, 3, 0, None);
        assert!(st.sprites.iter().all(|s| !s.add && s.z == 7));
        assert_eq!(st.blocking_ms(), Some(9 * 16));
        let mut st = Stage::default();
        flash(&mut st, 2, 2, None);
        assert_eq!(st.blocking_ms(), Some(10 * 16));
    }

    #[test]
    fn sub_4_wipes_in_copies_the_background_and_fades_the_picture_out() {
        let mut st = Stage::default();
        flash(&mut st, 4, 0, Some("HG011100_01.akb"));
        assert_eq!(st.blocking_ms(), Some(72 * 16));
        // The under picture wipes in over 4 frames, the over one over 6.
        let wipes = |st: &Stage| st.sprites.iter().map(|s| (s.z, s.add)).collect::<Vec<_>>();
        assert_eq!(wipes(&st), [(0, true), (1, true)]);
        run(&mut st, 8);
        // Nothing is baked: the loaded background is already the copy.
        assert!(st.take_painted().is_empty());
        run(&mut st, 70);
        assert!(st.sprites.is_empty());
        flash(&mut st, 4, 2, None);
        run(&mut st, 20);
        assert!(st.take_painted().is_empty());
    }

    #[test]
    fn the_screen_is_copied_once_as_the_background_picture() {
        let mut st = Stage::default();
        flash(&mut st, 3, 0, Some("bg1.akb"));
        assert!(st
            .sprites
            .iter()
            .all(|s| s.look == Look::picture("bg1.akb")));
        let mut st = Stage::default();
        effect(&mut st, 7, 0, Some("bg1.akb"));
        assert!(st.sprites.iter().all(|s| s.look != Look::Screen));
        let mut st = Stage::default();
        effect(&mut st, 2, 0, Some("bg1.akb"));
        assert!(st.sprites.iter().any(|s| matches!(
            &s.look,
            Look::Raster { source, .. } if **source == Look::picture("bg1.akb")
        )));
        let mut st = Stage::default();
        flash(&mut st, 6, 0, None);
        assert!(st.sprites.iter().all(|s| s.look != Look::Screen));
        assert!(st.sprites.iter().any(|s| s.z == 2 && !s.add));
    }

    #[test]
    fn the_strip_starts_over_a_black_that_fades_in_after_128_frames() {
        let mut st = Stage::default();
        flash(&mut st, 1, 0, None);
        let back = st
            .sprites
            .iter()
            .find(|s| matches!(s.look, Look::Fill { .. }));
        assert_eq!(back.map(|s| s.z), Some(STRIP_DEPTH));
        run(&mut st, 120);
        assert_eq!(
            st.sprites
                .iter()
                .find(|s| matches!(s.look, Look::Fill { .. }))
                .map(|s| s.alpha),
            Some(0)
        );
        run(&mut st, 100);
        assert!(
            st.sprites
                .iter()
                .find(|s| matches!(s.look, Look::Fill { .. }))
                .map(|s| s.alpha)
                .unwrap()
                > 0
        );
    }

    #[test]
    fn the_room_outlives_the_stop_and_goes_with_the_copy() {
        let mut st = Stage::default();
        effect(&mut st, 5, 0, None);
        effect(&mut st, 5, 1, None);
        assert!(st.sprites.iter().any(|s| s.owner == ROOM));
        flash(&mut st, 4, 0, Some("bg1.akb"));
        run(&mut st, 20);
        assert!(!st.sprites.iter().any(|s| s.owner == ROOM));
    }

    #[test]
    fn the_waves_advance_by_their_own_steps() {
        let mut st = Stage::default();
        effect(&mut st, 3, 0, Some("bg1.akb"));
        run(&mut st, 2);
        let phases: Vec<f32> = st
            .sprites
            .iter()
            .filter_map(|s| match &s.look {
                Look::Raster { phase, .. } => Some(*phase),
                _ => None,
            })
            .collect();
        // First frame: 0 + 1 and 1 + 2; second: 3 + 1 and 4 + 2.
        assert_eq!(phases, [4.0, 6.0]);
    }

    #[test]
    fn action_2_finishes_the_lap_before_the_effects_go() {
        let mut st = Stage::default();
        effect(&mut st, 2, 0, None);
        run(&mut st, 20);
        effect(&mut st, 2, 2, None);
        assert!(st.blocking_ms().is_some());
        run(&mut st, 140);
        assert!(st.sprites.is_empty());
    }
}
