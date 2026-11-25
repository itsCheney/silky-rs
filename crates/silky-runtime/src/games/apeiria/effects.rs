//! 景の海のアペイリア's attack and spell effects (`Game.998`, `1000`,
//! `1001`, `1002`), built the way the executable builds them: sprites
//! with keyframes on [`Stage`] timelines, and a few hand-written steps
//! ([`User`]) that move or redraw a sprite every frame.
//!
//! See `docs/games/apeiria.md` for what each one shows.

use super::anim::{Key, Look, Sprite, Stage, Step};
use super::panels::Paint;

/// The commands, which own the sprites they put up.
pub const ATTACK: u32 = 998;
pub const SPRITE_ANIME: u32 = 1000;
pub const SCREEN_EFFECT: u32 = 1001;
pub const SPELL: u32 = 1002;

const W: u32 = 1280;
const H: u32 = 720;

/// An ARGB colour as RGBA.
fn argb(c: u32) -> [u8; 4] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8]
}

fn fill(c: u32) -> Look {
    Look::Fill {
        color: argb(c),
        w: W,
        h: H,
    }
}

/// A picture with its rows moved along a sine wave.
fn raster(source: Look, amp: f32) -> Look {
    Look::Raster {
        source: Box::new(source),
        amp,
        step: 1.0,
        phase: 0.0,
    }
}

/// A hand-written step an effect takes every frame of a keyframe.
#[derive(Debug, Clone)]
pub enum User {
    /// `CCosmos`: the starry sky of a spell wavers row by row, six pixels
    /// at most, a degree further each frame and each row. It fades out
    /// over 8 frames when told to (`fading`), and cross-fades to another
    /// picture over 16 (`mixing`).
    Cosmos {
        owner: u32,
        sprite: u32,
        fading: Option<i32>,
        mixing: Option<i32>,
    },
    /// `CRasterAnime`: a picture or the screen wavers row by row. Mode 3
    /// raises the swing to `next_amp` after the first frame and settles
    /// after 64 frames.
    Raster {
        owner: u32,
        sprite: u32,
        mode: i32,
        count: i32,
        next_amp: f32,
    },
    /// `CAttackBullet`: up to four streaks of light (`len` pixels long)
    /// shoot out from about a point to the edge of the screen (`inward`:
    /// in from the edge), each over 4 frames and again on the next frame.
    Bullets {
        owner: u32,
        sprite: u32,
        inward: bool,
        /// Streaks start anywhere in 640 x 200 about the point, not 64 x 64.
        wide: bool,
        len: f32,
        at: (i32, i32),
        streaks: Vec<Streak>,
    },
    /// `CAttackBless`: a picture spins 45 degrees a frame flying from the
    /// middle of the screen to `to`, growing from nothing (or shrinking
    /// to nothing), and fades out over the last 16 frames.
    Bless {
        owner: u32,
        sprite: u32,
        to: (i32, i32),
        shrink: bool,
        angle: i32,
    },
    /// `CRandomSlash`: a slash (one of two pictures) somewhere in the
    /// middle of the screen shows through rule 00 over 5 frames, goes the
    /// same way over 5, and comes back elsewhere after up to 63 frames.
    Slash {
        owner: u32,
        sprite: u32,
        slash: u32,
        delay: i32,
        threshold: f32,
        erasing: bool,
    },
    /// `CLoginAnime`: 150 specks of light drift down from above the top
    /// of the screen, each to a place of its own over 32 to 159 frames,
    /// and start again; only the first 50 move until `all`, when they fly
    /// on to new places instead of starting again.
    Specks {
        owner: u32,
        sprite: u32,
        all: bool,
        specks: Vec<Speck>,
    },
    /// Spell 1: the screen in blocks growing or shrinking from `from` to
    /// `to` pixels over `frames` frames.
    Mosaic {
        owner: u32,
        sprite: u32,
        from: f32,
        to: f32,
        frames: i32,
    },
    /// Spell 6: the picture changes to another over the keyframe.
    Morph { owner: u32, sprite: u32 },
    /// Spell 0: white bars, 1 to 10 a frame, gather on the screen about
    /// its top left corner, wrapping round.
    Bars { owner: u32, sprite: u32 },
    /// `CRandomSlash2`: swords, one after another, fly from the middle of
    /// the screen to their places shrinking to a sixth over 6 frames.
    Swords {
        owner: u32,
        sprite: u32,
        swords: Vec<Sword>,
    },
    /// A wipe through a rule picture stepped by hand (きまぐれテンプテー
    /// ション's picture strips): the threshold of the sprite's
    /// [`Look::Reveal`] is `step` times the frame, and everything shows at
    /// the end.
    Ramp { owner: u32, sprite: u32, step: f32 },
    /// A tint going from `from` to `to` over the keyframe (255 is no
    /// tint), in [`Sprite::tint`].
    Tint {
        owner: u32,
        sprite: u32,
        from: u8,
        to: u8,
    },
    /// 先輩が私の妄想にドージンする's puff of breath: the picture fades in
    /// to 196 over the first half of the keyframe and out over the rest.
    Breath { owner: u32, sprite: u32 },
    /// One of きまぐれテンプテーション's own effects (see
    /// `kimaten1::fx`).
    K1(Box<crate::games::kimaten1::fx::Fx>),
}

/// One streak of [`User::Bullets`].
#[derive(Debug, Clone)]
pub struct Streak {
    sprite: u32,
    from: (i32, i32),
    to: (i32, i32),
    frame: i32,
}

/// One speck of [`User::Specks`].
#[derive(Debug, Clone)]
pub struct Speck {
    sprite: u32,
    from: (i32, i32),
    to: (i32, i32),
    frame: i32,
    frames: i32,
    delay: i32,
    again: bool,
}

/// One sword of [`User::Swords`].
#[derive(Debug, Clone)]
pub struct Sword {
    sprite: u32,
    to: (i32, i32),
    delay: i32,
    step: i32,
}

/// Starts again the specks of [`User::Specks`] that are not shown.
fn launch_specks(sprites: &mut [Sprite], rng: &mut u32, all: bool, specks: &mut [Speck]) {
    for p in specks.iter_mut() {
        let Some(s) = sprites.iter_mut().find(|s| s.id == p.sprite) else {
            continue;
        };
        if s.shown {
            continue;
        }
        p.from = (640, -64);
        p.to = (rand(rng, 320) + 480, rand(rng, 120));
        p.frames = rand(rng, 128) + 32;
        p.delay = rand(rng, 32);
        p.frame = 0;
        let k = (rand(rng, 25) + 25) as f32 / 100.0;
        if all {
            p.again = true;
        }
        s.scale = (k, k);
        s.alpha = 0;
        s.x = p.from.0 as f32;
        s.y = p.from.1 as f32;
        s.shown = true;
    }
}

/// A normally distributed random number (Box-Muller, as the executable).
fn gauss(rng: &mut u32) -> f64 {
    let u1 = (rand(rng, 32768) as f64 + 1.0) / 32769.0;
    let u2 = (rand(rng, 32768) as f64 + 1.0) / 32769.0 * 6.283184;
    (-2.0 * u1.ln()).sqrt() * u2.cos()
}

/// A random number below `n`.
fn rand(rng: &mut u32, n: i32) -> i32 {
    if *rng == 0 {
        *rng = 0x2545_f491;
    }
    *rng ^= *rng << 13;
    *rng ^= *rng >> 17;
    *rng ^= *rng << 5;
    ((*rng >> 1) % n.max(1) as u32) as i32
}

impl User {
    pub fn owner(&self) -> u32 {
        match self {
            User::Cosmos { owner, .. }
            | User::Raster { owner, .. }
            | User::Bullets { owner, .. }
            | User::Bless { owner, .. }
            | User::Slash { owner, .. }
            | User::Specks { owner, .. }
            | User::Mosaic { owner, .. }
            | User::Morph { owner, .. }
            | User::Bars { owner, .. }
            | User::Swords { owner, .. }
            | User::Ramp { owner, .. }
            | User::Tint { owner, .. }
            | User::Breath { owner, .. } => *owner,
            User::K1(k) => k.owner,
        }
    }

    /// The sprite it changes.
    pub fn sprite(&self) -> u32 {
        match self {
            User::Cosmos { sprite, .. }
            | User::Raster { sprite, .. }
            | User::Bullets { sprite, .. }
            | User::Bless { sprite, .. }
            | User::Slash { sprite, .. }
            | User::Specks { sprite, .. }
            | User::Mosaic { sprite, .. }
            | User::Morph { sprite, .. }
            | User::Bars { sprite, .. }
            | User::Swords { sprite, .. }
            | User::Ramp { sprite, .. }
            | User::Tint { sprite, .. }
            | User::Breath { sprite, .. } => *sprite,
            User::K1(k) => k.sprite,
        }
    }

    pub fn start(&mut self, sprites: &mut [Sprite]) {
        if let User::Breath { sprite, .. } = self {
            if let Some(s) = sprites.iter_mut().find(|s| s.id == *sprite) {
                s.alpha = 0;
            }
        }
    }

    pub fn finish(&mut self, sprites: &mut [Sprite]) {
        match self {
            User::Morph { sprite, .. } => {
                if let Some(s) = sprites.iter_mut().find(|s| s.id == *sprite) {
                    if let Look::Mix { to, .. } = &s.look {
                        s.look = (**to).clone();
                    }
                }
            }
            User::Ramp { sprite, .. } => {
                if let Some(s) = sprites.iter_mut().find(|s| s.id == *sprite) {
                    if let Look::Reveal { threshold, .. } = &mut s.look {
                        *threshold = 512.0;
                    }
                }
            }
            User::Tint { sprite, to, .. } => {
                if let Some(s) = sprites.iter_mut().find(|s| s.id == *sprite) {
                    s.tint = [*to; 3];
                }
            }
            User::Breath { sprite, .. } => {
                if let Some(s) = sprites.iter_mut().find(|s| s.id == *sprite) {
                    s.alpha = 0;
                }
            }
            User::K1(k) => k.finish(sprites),
            _ => {}
        }
    }

    /// Frame `f` of `n` of the keyframe that runs it.
    pub fn tick(&mut self, sprites: &mut [Sprite], rng: &mut u32, f: i32, n: i32) {
        if let User::K1(k) = self {
            k.tick(sprites, f);
            return;
        }
        let find = |sprites: &mut [Sprite], id: u32| sprites.iter().position(|s| s.id == id);
        match self {
            User::Bullets {
                inward,
                wide,
                len,
                at,
                streaks,
                ..
            } => {
                for b in streaks.iter_mut() {
                    let Some(i) = find(sprites, b.sprite) else {
                        continue;
                    };
                    let s = &mut sprites[i];
                    if !s.shown {
                        let start = if *wide {
                            (at.0 - 320 + rand(rng, 640), at.1 - 100 + rand(rng, 200))
                        } else {
                            (at.0 - 32 + rand(rng, 64), at.1 - 32 + rand(rng, 64))
                        };
                        let mut d = (rand(rng, 320) - 159, rand(rng, 320) - 159);
                        if d == (0, 0) {
                            d.1 = 1;
                        }
                        let mut edge = *at;
                        while (0..=1280).contains(&edge.0) && (0..=720).contains(&edge.1) {
                            edge = (edge.0 + d.0, edge.1 + d.1);
                        }
                        (b.from, b.to) = if *inward {
                            (edge, start)
                        } else {
                            (start, edge)
                        };
                        let (dx, dy) = ((b.to.0 - b.from.0) as f64, (b.to.1 - b.from.1) as f64);
                        s.turn = (dy.atan2(dx) * 180.0 / 3.141592) as f32 + 90.0;
                        s.turn_at = (8.0, 32.0);
                        s.x = b.from.0 as f32;
                        s.y = b.from.1 as f32;
                        s.scale = (1.0, 0.0);
                        s.alpha = 0;
                        s.shown = true;
                        b.frame = 0;
                    }
                    if b.frame == 4 {
                        s.shown = false;
                    }
                    b.frame += 1;
                    s.alpha = 255;
                    s.x = ((b.to.0 - b.from.0) * b.frame / 4 + b.from.0) as f32;
                    s.y = ((b.to.1 - b.from.1) * b.frame / 4 + b.from.1) as f32;
                    // The streak grows from its tail for 2 frames, then
                    // shrinks into its head.
                    let (h, tail_first) = if b.frame < 2 {
                        (b.frame as f32 * 0.5 * *len, true)
                    } else {
                        (*len - (b.frame - 2) as f32 * 0.5 * *len, false)
                    };
                    let bottom = tail_first != *inward;
                    s.scale = (1.0, h);
                    s.scale_at = (8.0, if bottom { 64.0 } else { 0.0 });
                }
                return;
            }
            User::Mosaic {
                sprite,
                from,
                to,
                frames,
                ..
            } => {
                if let Some(i) = find(sprites, *sprite) {
                    let b = *from + (f - 1) as f32 * (*to - *from) / *frames as f32;
                    if let Look::Mosaic { block, .. } = &mut sprites[i].look {
                        *block = (b as i32).max(1) as u32;
                    }
                }
                return;
            }
            User::Morph { sprite, .. } => {
                if let Some(i) = find(sprites, *sprite) {
                    if let Look::Mix { mix, .. } = &mut sprites[i].look {
                        *mix = (f * 255 / n.max(1)).clamp(0, 255) as u8;
                    }
                }
                return;
            }
            User::Bars { sprite, .. } => {
                let Some(i) = find(sprites, *sprite) else {
                    return;
                };
                let Look::Bars { bars } = &mut sprites[i].look else {
                    return;
                };
                let count = rand(rng, 10) + 1;
                let (mut drawn, mut tries) = (0, 0);
                while drawn < count || tries < count * 2 {
                    tries += 1;
                    let x = (gauss(rng) * 0.5 * 1280.0 + 1280.0) as i32 % 1280;
                    let y = (gauss(rng) * 0.5 * 720.0 + 720.0) as i32 % 720;
                    let mut w = (gauss(rng) * 0.2 * 320.0 + 320.0) as i32 % 320;
                    let mut h = (gauss(rng) * 0.2 * 4.0 + 4.0) as i32 & 3;
                    if w < 101 {
                        w = 100;
                    }
                    if h < 2 {
                        h = 1;
                    }
                    if x < 0 || y < 0 {
                        continue;
                    }
                    w = w.min(1280 - x);
                    h = h.min(720 - y);
                    bars.push([x, y, w, h]);
                    drawn += 1;
                }
                return;
            }
            User::Slash {
                slash,
                delay,
                threshold,
                erasing,
                ..
            } => {
                let Some(i) = find(sprites, *slash) else {
                    return;
                };
                let s = &mut sprites[i];
                if !s.shown {
                    let name = if rand(rng, 166) >= 100 {
                        "attack_03_17.akb"
                    } else {
                        "attack_03_16.akb"
                    };
                    s.x = (rand(rng, 640) - 320) as f32;
                    s.y = (rand(rng, 130) - 60) as f32;
                    s.look = Look::Reveal {
                        source: Box::new(Look::picture(name)),
                        rule: 0,
                        threshold: 0.0,
                        erasing: false,
                        stride: 0,
                    };
                    s.shown = true;
                    *delay = rand(rng, 64);
                    *threshold = 102.4;
                    *erasing = false;
                }
                if *delay > 0 {
                    *delay -= 1;
                    return;
                }
                if let Look::Reveal {
                    threshold: t,
                    erasing: e,
                    ..
                } = &mut s.look
                {
                    *t = *threshold;
                    *e = *erasing;
                }
                *threshold += 102.4;
                if *threshold > 512.0 {
                    if *erasing {
                        s.shown = false;
                    } else {
                        *threshold = 102.4;
                        *erasing = true;
                    }
                }
                return;
            }
            User::Specks { all, specks, .. } => {
                let n = if *all {
                    specks.len()
                } else {
                    50.min(specks.len())
                };
                for p in specks.iter_mut().take(n) {
                    let Some(i) = find(sprites, p.sprite) else {
                        continue;
                    };
                    let s = &mut sprites[i];
                    if p.delay > 0 {
                        p.delay -= 1;
                        continue;
                    }
                    s.alpha = if p.frame < 8 && s.alpha < 255 {
                        (p.frame as f32 * 255.0 * 0.125) as u8
                    } else if p.frame < p.frames - 8 {
                        255
                    } else {
                        // From 510: they stay lit to the end.
                        (510.0 - (p.frame - p.frames + 8) as f32 * 255.0 * 0.125).min(255.0) as u8
                    };
                    s.x = ((p.to.0 - p.from.0) * p.frame / p.frames + p.from.0) as f32;
                    s.y = ((p.to.1 - p.from.1) * p.frame / p.frames + p.from.1) as f32;
                    p.frame += 1;
                    if p.frame == p.frames {
                        if p.again {
                            p.to = (rand(rng, 128) + 576, rand(rng, 360));
                            p.frame = 0;
                        } else {
                            s.shown = false;
                        }
                    }
                }
                launch_specks(sprites, rng, *all, specks);
                return;
            }
            User::Swords { swords, .. } => {
                for w in swords.iter_mut() {
                    let Some(i) = find(sprites, w.sprite) else {
                        continue;
                    };
                    let s = &mut sprites[i];
                    if !s.shown {
                        continue;
                    }
                    if w.delay > 0 {
                        w.delay -= 1;
                    } else if w.step == 6 {
                        s.shown = false;
                    } else {
                        s.alpha = 255;
                        let k = 1.0 - w.step as f32 / 6.0;
                        s.scale = (k, k);
                        s.x = ((w.to.0 + 100) * w.step / 6 - 100) as f32;
                        s.y = ((w.to.1 + 300) * w.step / 6 - 300) as f32;
                        w.step += 1;
                    }
                }
                return;
            }
            User::Ramp { sprite, step, .. } => {
                if let Some(i) = find(sprites, *sprite) {
                    if let Look::Reveal { threshold, .. } = &mut sprites[i].look {
                        *threshold = f as f32 * *step;
                    }
                }
                return;
            }
            User::Breath { sprite, .. } => {
                if let Some(i) = find(sprites, *sprite) {
                    let half = n / 2;
                    if half > 0 {
                        sprites[i].alpha = if f <= half {
                            f * 196 / half
                        } else {
                            196 + (half - f) * 196 / (half + (n & 1))
                        }
                        .clamp(0, 255) as u8;
                    }
                }
                return;
            }
            User::Tint {
                sprite, from, to, ..
            } => {
                if let Some(i) = find(sprites, *sprite) {
                    let (a, b) = (*from as i32, *to as i32);
                    sprites[i].tint = [(a + (b - a) * f / n.max(1)) as u8; 3];
                }
                return;
            }
            User::Bless {
                sprite,
                to,
                shrink,
                angle,
                ..
            } => {
                let Some(i) = find(sprites, *sprite) else {
                    return;
                };
                let s = &mut sprites[i];
                let t = f as f32 / n as f32;
                let k = if *shrink { 1.0 - t } else { t };
                s.scale = (k, k);
                s.x = ((to.0 + 100) * f / n - 100) as f32;
                s.y = ((to.1 + 250) * f / n - 250) as f32;
                *angle += 45;
                s.turn = (*angle % 360) as f32;
                if f > 47 {
                    s.alpha = (255.0 - (f - 48) as f32 * 255.0 * 0.0625) as u8;
                }
                return;
            }
            _ => {}
        }
        let id = self.sprite();
        let Some(s) = sprites.iter_mut().find(|s| s.id == id) else {
            return;
        };
        match self {
            User::Cosmos { fading, mixing, .. } => {
                if let Some(n) = fading {
                    *n += 1;
                    let a = 255.0 - *n as f32 * 255.0 * 0.125;
                    s.alpha = a.max(0.0) as u8;
                    if a <= 0.0 {
                        s.shown = false;
                        return;
                    }
                } else if let Some(n) = mixing {
                    *n += 1;
                    let mix = (*n * 255 / 16).min(255) as u8;
                    if let Look::Raster { source, .. } = &mut s.look {
                        if let Look::Mix { mix: m, to, .. } = source.as_mut() {
                            *m = mix;
                            if mix == 255 {
                                **source = (**to).clone();
                            }
                        }
                    }
                    if mix == 255 {
                        *mixing = None;
                    }
                }
                if let Look::Raster { phase, .. } = &mut s.look {
                    *phase += 1.0;
                }
            }
            User::Raster {
                mode,
                count,
                next_amp,
                ..
            } => {
                if let Look::Raster { phase, amp, .. } = &mut s.look {
                    *phase += 1.0;
                    if *mode == 3 {
                        *amp = *next_amp;
                        if *count == 0x40 {
                            *mode = -1;
                        }
                        *count += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Collects the steps of an effect as its builder in the executable runs.
struct Build<'a> {
    st: &'a mut Stage,
    owner: u32,
    steps: Vec<Step>,
}

impl<'a> Build<'a> {
    fn new(st: &'a mut Stage, owner: u32) -> Self {
        Self {
            st,
            owner,
            steps: Vec::new(),
        }
    }

    /// Puts up a sprite (hidden until a keyframe shows it).
    fn sprite(&mut self, look: Look, z: i32) -> u32 {
        let id = self.st.id();
        self.steps
            .push(Step::Add(Sprite::new(id, self.owner, look, z)));
        id
    }

    /// Changes a sprite put up by this build.
    fn edit(&mut self, id: u32, f: impl FnOnce(&mut Sprite)) {
        for step in self.steps.iter_mut().rev() {
            if let Step::Add(s) = step {
                if s.id == id {
                    f(s);
                    return;
                }
            }
        }
    }

    fn adding(&mut self, id: u32) {
        self.edit(id, |s| s.add = true);
    }

    fn user(&mut self, u: User) -> u32 {
        let id = self.st.id();
        self.st.users.insert(id, u);
        id
    }

    /// Gives sprites their keyframes and starts a timeline of them;
    /// `block` makes the script wait for it.
    fn play(&mut self, sprites: Vec<(u32, Vec<Key>)>, block: bool) -> u32 {
        let g = self.st.id();
        let ids: Vec<u32> = sprites.iter().map(|(id, _)| *id).collect();
        for (id, keys) in sprites {
            self.steps.push(Step::Keys(id, keys));
        }
        self.steps.push(Step::Start(g, ids));
        if block {
            self.steps.push(Step::Wait(g));
        }
        g
    }

    /// Gives a sprite a new look when the steps before are taken.
    fn edit_later(&mut self, id: u32, look: Look) {
        self.steps.push(Step::Look(id, look));
    }

    /// The sprites put up by this build.
    fn added(&self) -> Vec<u32> {
        self.steps
            .iter()
            .filter_map(|s| match s {
                Step::Add(s) => Some(s.id),
                _ => None,
            })
            .collect()
    }

    fn remove(&mut self, ids: &[u32]) {
        self.steps.push(Step::Remove(ids.to_vec()));
    }

    fn done(self) {
        let steps = self.steps;
        self.st.play(steps);
    }
}

/// Where a command keeps a sprite, timeline or step it comes back to.
mod slot {
    pub const COSMOS: u32 = 1;
    pub const COSMOS_STEP: u32 = 2;
    pub const COSMOS_LINE: u32 = 3;
    pub const SCREEN: u32 = 4;
    pub const SPRITE: u32 = 5;
    pub const RASTER: u32 = 6;
    pub const SPOT: u32 = 7;
}

/// `Game.1001` (`sub`, arguments): screen effects of spells and attacks.
/// The script waits while [`Stage::blocking_ms`] says.
pub fn screen_effect(st: &mut Stage, sub: i32, ints: &[i32], strs: &[String]) {
    const O: u32 = SCREEN_EFFECT;
    let mut b = Build::new(st, O);
    let casa = b.st.casablanca;
    match sub {
        // A red light pulses over the screen: up to half over 32 frames,
        // 12 frames held, down over 32, for ever.
        0 => {
            let s = b.sprite(fill(0xffcc0000), 5);
            b.adding(s);
            b.play(
                vec![(
                    s,
                    vec![
                        Key::new(32).show(true).fade(0, 128),
                        Key::new(1).wait(11),
                        Key::new(32).fade(128, 0),
                        Key::jump(0),
                    ],
                )],
                false,
            );
        }
        // The screen goes white over 64 frames and comes back over 32 on
        // a starry sky that wavers on until sub 2 (sub 3: another sky,
        // or the sky changing to it over 16 frames).
        1 | 3 => {
            let picture = if sub == 1 {
                "cosmos00.akb"
            } else {
                "cosmos01.akb"
            };
            if sub == 3 {
                if let Some(u) = b.st.slots.get(&(O, slot::COSMOS_STEP)).copied() {
                    let sky = b.st.slots.get(&(O, slot::COSMOS)).copied();
                    if let (Some(User::Cosmos { mixing, .. }), Some(sky)) =
                        (b.st.users.get_mut(&u), sky)
                    {
                        *mixing = Some(0);
                        if let Some(s) = b.st.sprite(sky) {
                            if let Look::Raster { source, .. } = &mut s.look {
                                let from = std::mem::take(source.as_mut());
                                **source = Look::Mix {
                                    from: Box::new(from),
                                    to: Box::new(Look::picture(picture)),
                                    mix: 0,
                                };
                            }
                        }
                        return;
                    }
                }
            }
            let white = b.sprite(fill(0xffffffff), 5);
            b.adding(white);
            let sky = b.sprite(raster(Look::picture(picture), 6.0), 0);
            b.adding(sky);
            let u = b.user(User::Cosmos {
                owner: O,
                sprite: sky,
                fading: None,
                mixing: None,
            });
            // Casablanca wipes the white in through rule 8 and out through
            // rule 26 where Apeiria fades it.
            let (up, down) = (Key::new(64).show(true), Key::new(32).show(false));
            let (up, down) = if casa {
                (up.wiping(8, true), down.wiping(26, false))
            } else {
                (up.fade(0, 255), down.fade(255, 0))
            };
            b.play(vec![(white, vec![up])], true);
            let line = b.play(
                vec![(sky, vec![Key::new(2).show(true).user(u), Key::jump(0)])],
                false,
            );
            b.play(vec![(white, vec![down])], true);
            b.remove(&[white]);
            b.st.slots.insert((O, slot::COSMOS), sky);
            b.st.slots.insert((O, slot::COSMOS_STEP), u);
            b.st.slots.insert((O, slot::COSMOS_LINE), line);
        }
        // The sky fades out over 8 frames; the script waits for it.
        2 => {
            let u = b.st.slots.get(&(O, slot::COSMOS_STEP)).copied();
            if let Some(User::Cosmos { fading, .. }) = u.and_then(|u| b.st.users.get_mut(&u)) {
                *fading = Some(0);
            }
            if let Some(line) = b.st.slots.get(&(O, slot::COSMOS_LINE)).copied() {
                b.steps.push(Step::Wait(line));
            }
        }
        // The screen wipes to black through rule 60 over 128 frames:
        // it stays black (4), or the script waits and the screen comes
        // back at once (5).
        4 | 5 => {
            let s = b.sprite(fill(0xff000000), 5);
            let keys = vec![Key::new(128).show(true).wiping(60, true)];
            if sub == 4 {
                b.play(vec![(s, keys)], false);
            } else {
                b.play(vec![(s, keys)], true);
                b.remove(&[s]);
            }
        }
        // Slashes (attack_03_16, 03_17) show and go one after another
        // somewhere in the middle of the screen, until sub 255.
        6 => {
            let holder = b.sprite(Look::Nothing, 0);
            let slash = b.sprite(Look::Nothing, 1);
            let u = b.user(User::Slash {
                owner: O,
                sprite: holder,
                slash,
                delay: 0,
                threshold: 0.0,
                erasing: false,
            });
            b.play(
                vec![(holder, vec![Key::new(6).show(true).user(u), Key::jump(0)])],
                false,
            );
        }
        // A burst (attack_09_00) grows from nothing over 16 frames about
        // (640, 545), then grows on to twice its size while it fades.
        7 => {
            let s = b.sprite(Look::picture("attack_09_00.akb"), 0);
            b.adding(s);
            let at = (640.0, 545.0);
            b.play(
                vec![(
                    s,
                    vec![
                        Key::new(16).show(true).scaling((0.0, 0.0), (1.0, 1.0), at),
                        Key::new(16)
                            .fade(255, 0)
                            .scaling((1.0, 1.0), (2.0, 2.0), at),
                    ],
                )],
                true,
            );
            b.remove(&[s]);
        }
        // The screen zooms in to twice its size about a point and moves
        // left over 16 frames (8: about (899, 388) by 300 pixels; 18:
        // about (600, 320), not moving; 21: to 1.5 times about (830, 370)
        // by 150 pixels over 300 frames); called again, it zooms back
        // and goes.
        8 | 18 | 21 => {
            let (frames, to, at, dx) = match sub {
                8 => (16, 2.0, (899.0, 388.0), -300),
                18 => (16, 2.0, (600.0, 320.0), 0),
                _ => (300, 1.5, (830.0, 370.0), -150),
            };
            let back = if sub == 21 { 2.0 } else { to };
            match b.st.slots.get(&(O, slot::SCREEN)).copied() {
                None => {
                    let s = b.sprite(Look::Screen, 5);
                    b.st.slots.insert((O, slot::SCREEN), s);
                    let mut k = Key::new(frames)
                        .show(true)
                        .scaling((1.0, 1.0), (to, to), at);
                    if sub != 18 {
                        k = k.moving((0, 0), (dx, 0));
                    }
                    b.play(vec![(s, vec![k])], true);
                }
                Some(s) => {
                    let mut k = Key::new(frames)
                        .show(false)
                        .scaling((back, back), (1.0, 1.0), at);
                    if sub != 18 {
                        k = k.moving((dx, 0), (0, 0));
                    }
                    b.play(vec![(s, vec![k])], true);
                }
            }
        }
        // Noise wavering ten pixels row by row, enlarged 1.2 times about
        // the middle, until sub 255.
        9 => {
            let s = b.sprite(raster(Look::picture("noise.akb"), 10.0), 0);
            b.edit(s, |s| {
                s.scale = (1.2, 1.2);
                s.scale_at = (640.0, 360.0);
            });
            let u = b.user(User::Raster {
                owner: O,
                sprite: s,
                mode: 1,
                count: 0,
                next_amp: 10.0,
            });
            b.play(
                vec![(s, vec![Key::new(2).show(true).user(u), Key::jump(0)])],
                false,
            );
        }
        // The screen as it is now blinks out for 8 frames after 42 and
        // back after 128, for ever.
        10 => {
            let s = b.sprite(Look::Screen, 0);
            b.play(
                vec![(
                    s,
                    vec![
                        Key::new(1).show(true),
                        Key::new(8).wait(42).fade(255, 0),
                        Key::new(8).wait(128).fade(0, 255),
                        Key::jump(0),
                    ],
                )],
                false,
            );
        }
        // Pictures (name, frames, rule) wipe in one after another, each
        // when the one before is in.
        11 => {
            let mut wait = 0;
            let mut list = vec![];
            for (k, name) in strs.iter().enumerate() {
                let frames = ints.get(k * 2).copied().unwrap_or(1);
                let rule = ints.get(k * 2 + 1).copied().unwrap_or(0).max(0) as u32;
                let s = b.sprite(Look::picture(name), 0);
                let mut key = Key::new(frames.max(1)).show(true).wiping(rule, true);
                if wait != 0 {
                    key = key.wait(wait);
                }
                wait += frames;
                list.push((s, vec![key]));
            }
            b.play(list, false);
        }
        // The screen wavers 80 pixels row by row, enlarged 1.2 times
        // about the middle, until sub 255; 13 makes it 150.
        12 | 13 => {
            let next = if sub == 12 { 80.0 } else { 150.0 };
            if sub == 13 {
                if let Some(u) = b.st.slots.get(&(O, slot::RASTER)).copied() {
                    if let Some(User::Raster {
                        mode,
                        count,
                        next_amp,
                        ..
                    }) = b.st.users.get_mut(&u)
                    {
                        *mode = 3;
                        *count = 0;
                        *next_amp = next;
                        return;
                    }
                }
            }
            let s = b.sprite(raster(Look::Screen, 0.0), 0);
            b.edit(s, |s| {
                s.scale = (1.2, 1.2);
                s.scale_at = (640.0, 360.0);
            });
            let u = b.user(User::Raster {
                owner: O,
                sprite: s,
                mode: 3,
                count: 0,
                next_amp: next,
            });
            b.st.slots.insert((O, slot::RASTER), u);
            b.play(
                vec![(s, vec![Key::new(2).show(true).user(u), Key::jump(0)])],
                false,
            );
        }
        // A pale green light comes up over 12 frames and stays (15: the
        // same light again; Casablanca wipes it in through rule 18, and
        // the second time wipes it out).
        14 | 15 => {
            let s = match b.st.slots.get(&(O, slot::SPRITE)).copied() {
                Some(s) if sub == 15 => s,
                _ => {
                    let s = b.sprite(fill(0xff88ff88), 15);
                    b.adding(s);
                    b.st.slots.insert((O, slot::SPRITE), s);
                    s
                }
            };
            let key = Key::new(12).show(true);
            let key = match (casa, sub) {
                (false, _) => key.fade(0, 255),
                (true, 14) => key.wiping(18, true),
                (true, _) => key.wiping(18, false),
            };
            b.play(vec![(s, vec![key])], true);
        }
        // A sky picture (attack_14_01, 1480 x 920 from (-100, -100))
        // fades in over 12 frames wavering 30 pixels row by row, until
        // sub 255.
        17 => {
            let s = b.sprite(raster(Look::picture("attack_14_01.akb"), 30.0), 0);
            b.edit(s, |s| {
                s.x = -100.0;
                s.y = -100.0;
            });
            let u = b.user(User::Raster {
                owner: O,
                sprite: s,
                mode: -1,
                count: 0,
                next_amp: 30.0,
            });
            b.play(
                vec![(
                    s,
                    vec![
                        Key::new(12).show(true).fade(0, 255).user(u),
                        Key::new(2).show(true).user(u),
                        Key::jump(1),
                    ],
                )],
                false,
            );
        }
        // A spotlight (spot.akb) is laid on; called again, it goes.
        19 => match b.st.slots.remove(&(O, slot::SPOT)) {
            None => {
                let s = b.sprite(Look::picture("spot.akb"), 0);
                b.adding(s);
                b.edit(s, |s| s.shown = true);
                b.st.slots.insert((O, slot::SPOT), s);
            }
            Some(s) => b.remove(&[s]),
        },
        // A white flash: up over 128 frames (Casablanca wipes it in through
        // rule 8), down over 128.
        20 => {
            let s = b.sprite(fill(0xffffffff), 5);
            b.adding(s);
            let up = Key::new(128).show(true);
            let up = if casa {
                up.wiping(8, true)
            } else {
                up.fade(0, 255)
            };
            b.play(vec![(s, vec![up, Key::new(128).fade(255, 0)])], true);
        }
        // Everything of this command goes.
        255 => {
            b.st.clear(O);
            b.st.slots.retain(|k, _| k.0 != O);
        }
        _ => {}
    }
    b.done();
}

/// `Game.1002` (`sub`, numbers, words): spells, each played to its end
/// (the script waits). What the executable draws into the background
/// is drawn into [`Panels::painted`](super::panels::Panels) as the
/// sprite ends; a screen left covered in a colour (spell 0, an
/// increasing spell 1) is a sprite over everything. Both stay until the
/// background changes. Gives back `Some(None)` when the screen is no longer
/// covered.
pub fn spell(st: &mut Stage, sub: i32, ints: &[i32], strs: &[String]) -> Option<Option<u32>> {
    const O: u32 = SPELL;
    let arg = |k: usize| ints.get(k).copied().unwrap_or(0);
    let mut cover = None;
    let mut b = Build::new(st, O);
    let mid = (640.0, 360.0);
    match sub {
        // The screen goes white over 180 frames while white bars gather
        // on it; it stays covered in white.
        0 => {
            let bars = b.sprite(Look::Bars { bars: vec![] }, 7);
            let white = b.sprite(fill(0xffffffff), 8);
            let u = b.user(User::Bars {
                owner: O,
                sprite: bars,
            });
            b.play(
                vec![
                    (bars, vec![Key::new(180).show(true).user(u)]),
                    (white, vec![Key::new(180).show(true).fade(0, 255)]),
                ],
                true,
            );
            b.remove(&[bars]);
            b.st.until_background.push(white);
        }
        // (from, to, frames): the screen in blocks growing from `from`
        // pixels to `to` as it goes black, or shrinking as it comes back
        // from black.
        1 => {
            let (from, to, frames) = (arg(0), arg(1), arg(2).max(1));
            let s = b.sprite(
                Look::Mosaic {
                    source: Box::new(Look::Screen),
                    block: from.max(1) as u32,
                },
                8,
            );
            let black = b.sprite(fill(0xff000000), 9);
            let u = b.user(User::Mosaic {
                owner: O,
                sprite: s,
                from: from as f32,
                to: to as f32,
                frames,
            });
            let (a0, a1) = if from < to { (0, 255) } else { (255, 0) };
            b.play(
                vec![
                    (s, vec![Key::new(frames).show(true).user(u)]),
                    (black, vec![Key::new(frames).show(true).fade(a0, a1)]),
                ],
                true,
            );
            if from < to {
                b.remove(&[s]);
                b.st.until_background.push(black);
            } else {
                b.remove(&[s, black]);
                cover = Some(None);
            }
        }
        // A band of light (scan.akb) sweeps across over 32 frames.
        2 => {
            let s = b.sprite(Look::picture("scan.akb"), 8);
            b.adding(s);
            b.play(
                vec![(
                    s,
                    vec![Key::new(32).show(true).moving((-200, 0), (1680, 0))],
                )],
                true,
            );
            b.remove(&[s]);
        }
        // The screen, held 1.2 times its size for 20 frames, comes back
        // to its size over 8 as it goes.
        3 => {
            let s = b.sprite(Look::Screen, 10);
            b.edit(s, |s| {
                s.scale = (1.2, 1.2);
                s.scale_at = mid;
                s.shown = true;
            });
            b.play(
                vec![(
                    s,
                    vec![Key::new(8).wait(20).show(false).fade(255, 0).scaling(
                        (1.2, 1.2),
                        (1.0, 1.0),
                        mid,
                    )],
                )],
                true,
            );
            b.remove(&[s]);
        }
        // The screen goes dark red over 128 frames, the background
        // painted so.
        4 => {
            let s = b.sprite(fill(0xff3b010c), 8);
            let mut key = Key::new(128).show(true).fade(0, 255);
            if b.st.casablanca {
                key = key.wiping(0, true);
            }
            b.play(vec![(s, vec![key])], true);
            b.st.to_background.push((s, Paint::Fill(0x3b010c)));
        }
        // (frames, colour), 10 (picture, frames): a light up over the
        // frames, and down after 4 more.
        5 | 10 => {
            let (look, frames) = if sub == 5 {
                (fill(arg(1) as u32 | 0xff000000), arg(0))
            } else {
                let name = strs.first().cloned().unwrap_or_default();
                (Look::picture(&name), arg(0))
            };
            let s = b.sprite(look, 15);
            b.adding(s);
            let frames = frames.max(1);
            b.play(
                vec![(
                    s,
                    vec![
                        Key::new(frames).show(true).fade(0, 255),
                        Key::new(frames).wait(4).show(true).fade(255, 0),
                    ],
                )],
                true,
            );
            b.remove(&[s]);
        }
        // (picture, second picture, x, y, frames): the picture grows from
        // nothing about (x, y) over the frames, then turns once about it
        // changing to the second, which is painted into the background.
        6 => {
            let first = strs.first().cloned().unwrap_or_default();
            let second = strs.get(1).cloned().unwrap_or_default();
            let at = (arg(0) as f32, arg(1) as f32);
            let frames = arg(2).max(1);
            let s = b.sprite(Look::picture(&first), 8);
            let u = b.user(User::Morph {
                owner: O,
                sprite: s,
            });
            b.play(
                vec![(
                    s,
                    vec![Key::new(frames).show(true).fade(0, 255).scaling(
                        (0.0, 0.0),
                        (1.0, 1.0),
                        at,
                    )],
                )],
                true,
            );
            b.edit_later(
                s,
                Look::Mix {
                    from: Box::new(Look::picture(&first)),
                    to: Box::new(Look::picture(&second)),
                    mix: 0,
                },
            );
            b.play(
                vec![(s, vec![Key::new(frames).turning(0, 360, at).user(u)])],
                true,
            );
            b.st.to_background.push((s, Paint::Picture(second)));
        }
        // (0, 1): a burst grows to 2.5 times its size over 24 frames; two
        // more pictures come up over 32 frames after 32 and 64.
        9 => {
            let (a, c) = if arg(0) == 0 {
                ("attack_14_00.akb", "attack_14_01.akb")
            } else {
                ("attack_14_02.akb", "attack_14_03.akb")
            };
            let s0 = b.sprite(Look::picture("attack_14_04.akb"), 8);
            let s1 = b.sprite(Look::picture(a), 8);
            let s2 = b.sprite(Look::picture(c), 9);
            // Casablanca wipes the two pictures in (rule 0, or 18 for the
            // second pair); other values just put them up.
            let casa = b.st.casablanca;
            let wipe = |key: Key| match (casa, arg(0)) {
                (false, _) => key.fade(0, 255),
                (true, 0) => key.wiping(0, true),
                (true, 1) => key.wiping(18, true),
                (true, _) => key,
            };
            b.play(
                vec![
                    (
                        s0,
                        vec![Key::new(24).show(true).fade(0, 255).scaling(
                            (0.0, 0.0),
                            (2.5, 2.5),
                            mid,
                        )],
                    ),
                    (s1, vec![wipe(Key::new(32).wait(32).show(true))]),
                    (s2, vec![wipe(Key::new(32).wait(64).show(true))]),
                ],
                true,
            );
            b.remove(&[s0, s1, s2]);
        }
        _ => {}
    }
    b.done();
    cover
}

/// `Game.1000` (`sub`): 0 puts up the specks of light of the title of a
/// log-in scene, 1 sets all of them going, 2 takes them off.
pub fn sprite_anime(st: &mut Stage, sub: i32) {
    const O: u32 = SPRITE_ANIME;
    match sub {
        0 => {
            st.clear(O);
            let mut b = Build::new(st, O);
            let holder = b.sprite(Look::Nothing, 0);
            let mut specks = vec![];
            for _ in 0..150 {
                let s = b.sprite(Look::picture("light.akb"), 8);
                b.adding(s);
                specks.push(Speck {
                    sprite: s,
                    from: (640, 720),
                    to: (640, 720),
                    frame: 0,
                    frames: 1,
                    delay: 0,
                    again: false,
                });
            }
            let u = b.user(User::Specks {
                owner: O,
                sprite: holder,
                all: false,
                specks,
            });
            b.play(
                vec![(holder, vec![Key::new(2).show(true).user(u), Key::jump(0)])],
                false,
            );
            b.done();
            // They start at once, not on the first frame.
            if let Some(User::Specks { specks, .. }) = st.users.get_mut(&u) {
                launch_specks(&mut st.sprites, &mut st.rng, false, specks);
            }
            st.slots.insert((O, 0), u);
        }
        1 => {
            let u = st.slots.get(&(O, 0)).copied();
            if let Some(User::Specks { all, .. }) = u.and_then(|u| st.users.get_mut(&u)) {
                *all = true;
            }
        }
        2 => {
            st.clear(O);
            st.slots.retain(|k, _| k.0 != O);
        }
        _ => {}
    }
}

/// `Game.998` (`sub`, arguments): attacks. Sub 0 (kind, ...) plays one
/// and waits for it; 1 (number, kind, ...) builds one, 2 (number) starts
/// it without waiting, 3 (number) takes it off, 4 takes all off.
pub fn attack(st: &mut Stage, sub: i32, ints: &[i32]) {
    const O: u32 = ATTACK;
    let arg = |k: usize| ints.get(k).copied().unwrap_or(0);
    match sub {
        0 => {
            let mut b = Build::new(st, O);
            let keys = build_attack(&mut b, arg(0), &ints[ints.len().min(1)..], false);
            let ids = b.added();
            b.play(keys, true);
            b.remove(&ids);
            b.done();
        }
        1 => {
            let id = arg(0) as u32;
            drop_attack(st, id);
            let mut b = Build::new(st, O);
            let keys = build_attack(&mut b, arg(1), &ints[ints.len().min(2)..], true);
            b.done();
            st.sets.insert((O, id), keys);
        }
        2 => {
            let id = arg(0) as u32;
            if let Some(keys) = st.sets.get(&(O, id)).cloned() {
                let mut b = Build::new(st, O);
                let g = b.play(keys, false);
                b.done();
                st.slots.insert((O, id), g);
            }
        }
        3 => drop_attack(st, arg(0) as u32),
        4 => {
            let ids: Vec<u32> = st
                .sprites
                .iter()
                .filter(|s| s.owner == O)
                .map(|s| s.id)
                .collect();
            st.to_background.retain(|(s, _)| !ids.contains(s));
            st.play(vec![Step::Remove(ids)]);
            st.sets.retain(|k, _| k.0 != O);
            st.slots.retain(|k, _| k.0 != O);
        }
        _ => {}
    }
}

/// Takes off a built attack.
fn drop_attack(st: &mut Stage, id: u32) {
    st.slots.remove(&(ATTACK, id));
    if let Some(keys) = st.sets.remove(&(ATTACK, id)) {
        let ids: Vec<u32> = keys.iter().map(|(s, _)| *s).collect();
        st.to_background.retain(|(s, _)| !ids.contains(s));
        st.play(vec![Step::Remove(ids)]);
    }
}

/// Puts up the sprites of an attack of `kind` and gives back their
/// keyframes. `kept`: built to be started later, so it loops where the
/// executable makes it loop.
fn build_attack(b: &mut Build, kind: i32, a: &[i32], kept: bool) -> Vec<(u32, Vec<Key>)> {
    let arg = |k: usize| a.get(k).copied().unwrap_or(0);
    // Two fades in from nothing, one after the other (the executable
    // gives most attack pictures these two keyframes).
    let fades = |frames: i32| {
        vec![
            Key::new(frames).show(true).fade(0, 255),
            Key::new(frames).show(true).fade(0, 255),
        ]
    };
    // Casablanca wipes its pictures in and out where Apeiria fades them
    // in: shown at the end of the first keyframe, hidden at the end of
    // the second.
    let casa = b.st.casablanca;
    let wipes = |frames: i32, rule: u32| {
        vec![
            Key::new(frames).show(true).wiping(rule, true),
            Key::new(frames).show(false).wiping(rule, false),
        ]
    };
    let twice = |frames: i32| {
        if casa {
            wipes(frames, 0)
        } else {
            fades(frames)
        }
    };
    let mid = (640.0, 360.0);
    match kind {
        // Streaks of light (direction, length, picture and spread, count,
        // x, y, ...).
        0 => {
            let (inward, len, wide, x, y, count) =
                (arg(0) == 1, arg(1), arg(2) == 1, arg(3), arg(4), arg(7));
            let picture = if wide {
                "attack_01_01.akb"
            } else {
                "attack_01_00.akb"
            };
            let holder = b.sprite(Look::Nothing, 0);
            let n = if count == -1 { 4 } else { count.clamp(0, 4) };
            let mut streaks = vec![];
            for _ in 0..n {
                let s = b.sprite(Look::picture(picture), 1);
                b.adding(s);
                streaks.push(Streak {
                    sprite: s,
                    from: (0, 0),
                    to: (0, 0),
                    frame: 0,
                });
            }
            let u = b.user(User::Bullets {
                owner: ATTACK,
                sprite: holder,
                inward,
                wide,
                len: len as f32 / 64.0,
                at: (x, y),
                streaks,
            });
            let keys = if kept {
                vec![Key::new(2).show(true).user(u), Key::jump(0)]
            } else {
                let mut frames = count * 5 - 4;
                if frames == 1 {
                    frames = 4;
                }
                vec![Key::new(frames.max(1)).show(true).user(u)]
            };
            vec![(holder, keys)]
        }
        // A slash over the screen (0, 1).
        1 => {
            let name = ["attack_02_00.akb", "attack_02_01.akb"];
            let s = b.sprite(Look::picture(name[arg(0).clamp(0, 1) as usize]), 12);
            vec![(s, twice(6))]
        }
        // Slashes and bursts (0 to 12); 9 and 10 grow to twice their size
        // fading out, 11 and 12 grow from nothing first.
        2 => {
            const NAMES: [&str; 13] = [
                "attack_03_00.akb",
                "attack_03_09.akb",
                "attack_03_06.akb",
                "attack_03_10.akb",
                "attack_03_08.akb",
                "attack_03_01.akb",
                "attack_03_07.akb",
                "attack_03_11.akb",
                "attack_03_12.akb",
                "attack_03_13.akb",
                "attack_03_18.akb",
                "attack_03_21.akb",
                "attack_03_22.akb",
            ];
            let k = arg(0).clamp(0, 12);
            let s = b.sprite(Look::picture(NAMES[k as usize]), 12);
            b.adding(s);
            let keys = match k {
                9 | 10 => vec![
                    if casa {
                        Key::new(6)
                            .show(true)
                            .wiping(0, true)
                            .scaling((1.0, 1.0), (1.0, 1.0), mid)
                    } else {
                        Key::new(6)
                            .show(true)
                            .fade(0, 255)
                            .scaling((1.0, 1.0), (1.0, 1.0), mid)
                    },
                    Key::new(6)
                        .show(true)
                        .fade(255, 0)
                        .scaling((1.0, 1.0), (2.0, 2.0), mid),
                ],
                11 | 12 => vec![
                    if casa {
                        Key::new(4)
                            .show(true)
                            .wiping(0, true)
                            .scaling((0.0, 0.0), (1.0, 1.0), mid)
                    } else {
                        Key::new(4)
                            .show(true)
                            .fade(0, 255)
                            .scaling((0.0, 0.0), (1.0, 1.0), mid)
                    },
                    Key::new(4)
                        .show(true)
                        .fade(255, 0)
                        .scaling((1.0, 1.0), (2.0, 2.0), mid),
                ],
                0 | 4 if casa => wipes(6, 1),
                _ => twice(6),
            };
            vec![(s, keys)]
        }
        // Light and a rift (0, 1: added; 2: laid over).
        3 => {
            let name = ["attack_04_00.akb", "attack_04_01.akb", "attack_12_00.akb"];
            let k = arg(0).clamp(0, 2);
            let s = b.sprite(Look::picture(name[k as usize]), 1);
            if k != 2 {
                b.adding(s);
            }
            vec![(s, twice(6))]
        }
        // A blast about (x, y), up and down over 6 frames each.
        4 => {
            let name = ["attack_05_00.akb", "attack_05_01.akb"];
            let s = b.sprite(Look::picture(name[arg(0).clamp(0, 1) as usize]), 1);
            b.adding(s);
            b.edit(s, |s| {
                s.x = (arg(1) - 640) as f32;
                s.y = (arg(2) - 360) as f32;
            });
            vec![(
                s,
                vec![
                    if casa {
                        Key::new(6).show(true).wiping(8, true)
                    } else {
                        Key::new(6).show(true).fade(0, 255)
                    },
                    Key::new(6).show(true).fade(255, 0),
                ],
            )]
        }
        // A pillar of light (2: 100 pixels left; 3: 200 right).
        5 => {
            let s = b.sprite(Look::picture("attack_06_00.akb"), 1);
            b.adding(s);
            let dx = match arg(0) {
                2 => -100.0,
                3 => 200.0,
                _ => 0.0,
            };
            b.edit(s, |s| s.x = dx);
            if casa {
                let rule = if arg(0) == 1 { 0 } else { 18 };
                return vec![(s, wipes(6, rule))];
            }
            vec![(s, twice(6))]
        }
        // Casablanca: a pillar of fire drops in from above over 12 frames,
        // then wipes out after 4 more.
        18 if casa => {
            let s = b.sprite(Look::picture("attack_17_00.akb"), 0);
            b.adding(s);
            vec![(
                s,
                vec![
                    Key::new(12).show(true).moving((0, -1480), (0, 0)),
                    Key::new(24).wait(4).show(false).wiping(0, false),
                ],
            )]
        }
        // A blessing flying to (x, y) over 64 frames (shrinking when the
        // third argument is not 0).
        6 => {
            let s = b.sprite(Look::picture("attack_07_00.akb"), 1);
            b.adding(s);
            let at = (740.0, 610.0);
            let shrink = arg(2) != 0;
            b.edit(s, |s| {
                s.turn_at = at;
                s.scale_at = at;
                s.x = -100.0;
                s.y = -250.0;
                let k = if shrink { 1.0 } else { 0.0 };
                s.scale = (k, k);
            });
            let u = b.user(User::Bless {
                owner: ATTACK,
                sprite: s,
                to: (arg(0) - 740, arg(1) - 610),
                shrink,
                angle: 0,
            });
            vec![(s, vec![Key::new(64).show(true).user(u)])]
        }
        // A flash growing to twice its size over 12 frames, then back
        // over 4 while it goes.
        7 => {
            let s = b.sprite(Look::picture("attack_08_00.akb"), 12);
            vec![(
                s,
                vec![
                    Key::new(12)
                        .show(true)
                        .fade(0, 255)
                        .scaling((1.0, 1.0), (2.0, 2.0), mid),
                    Key::new(4)
                        .show(false)
                        .fade(255, 0)
                        .scaling((2.0, 2.0), (1.0, 1.0), mid),
                ],
            )]
        }
        // A pale yellow light up over 32 frames, down over 12.
        8 => {
            let s = b.sprite(fill(0xffffffc0), 12);
            b.adding(s);
            vec![(
                s,
                vec![
                    Key::new(32).show(true).fade(0, 255),
                    Key::new(12).show(false).fade(255, 0),
                ],
            )]
        }
        // A burst about (x, y) wavering 50 pixels row by row, growing
        // from nothing to twice its size over 16 frames, then fading out
        // over 32.
        9 => {
            let s = b.sprite(raster(Look::picture("attack_11_00.akb"), 50.0), 12);
            b.adding(s);
            b.edit(s, |s| {
                s.x = (arg(0) - 640) as f32;
                s.y = (arg(1) - 360) as f32;
            });
            let u = b.user(User::Raster {
                owner: ATTACK,
                sprite: s,
                mode: 1,
                count: 0,
                next_amp: 50.0,
            });
            vec![(
                s,
                vec![
                    Key::new(16)
                        .show(true)
                        .fade(0, 255)
                        .scaling((0.0, 0.0), (2.0, 2.0), (640.0, 720.0))
                        .user(u),
                    Key::new(32).show(true).fade(255, 0),
                ],
            )]
        }
        // Swords (x1, y1, x2, y2, count) landing one after another in the
        // box, until taken off (11: other swords).
        10 | 11 => {
            let names = if kind == 10 {
                ["attack_03_02.akb", "attack_03_03.akb"]
            } else {
                ["attack_03_14.akb", "attack_03_15.akb"]
            };
            let (x1, y1, x2, y2, count) = (arg(0), arg(1), arg(2), arg(3), arg(4));
            let holder = b.sprite(Look::Nothing, 0);
            let mut swords = vec![];
            let mut delay = 1;
            for k in 0..count.max(0) {
                let pick = rand(&mut b.st.rng, 100) & 1;
                let s = b.sprite(Look::picture(names[pick as usize]), 1);
                // The executable never sets up the first.
                if k == 0 {
                    continue;
                }
                let turn = rand(&mut b.st.rng, 360) as f32;
                let at = (740.0, 660.0);
                b.edit(s, |s| {
                    s.x = -100.0;
                    s.y = -300.0;
                    s.scale_at = at;
                    s.turn_at = at;
                    s.turn = turn;
                    s.alpha = 0;
                    s.shown = true;
                });
                let d = delay + (rand(&mut b.st.rng, 2) & 1);
                let to = (
                    x1 + rand(&mut b.st.rng, (x2 - x1).max(1)) - 740,
                    y1 + rand(&mut b.st.rng, (y2 - y1).max(1)) - 660,
                );
                swords.push(Sword {
                    sprite: s,
                    to,
                    delay: d,
                    step: 0,
                });
                delay = d + 1;
            }
            let u = b.user(User::Swords {
                owner: ATTACK,
                sprite: holder,
                swords,
            });
            vec![(holder, vec![Key::new(2).show(true).user(u), Key::jump(0)])]
        }
        // Eight swords appear in a ring one after another, 2 frames apart,
        // growing to half size over 4 frames and on to full size over 4
        // while they fade (17: other swords).
        12 | 17 => {
            let name = if kind == 12 {
                "attack_03_14.akb"
            } else {
                "attack_03_19.akb"
            };
            const PLACES: [(i32, i32, f32); 8] = [
                (394, -490, 0.0),
                (-594, -79, 0.0),
                (-462, -490, 53.0),
                (276, -116, 53.0),
                (-301, -517, 108.0),
                (-76, -143, 7.0),
                (131, -291, 57.0),
                (-259, -362, 138.0),
            ];
            let at = (740.0, 660.0);
            let mut out = vec![];
            for (k, (x, y, turn)) in PLACES.into_iter().enumerate() {
                let s = b.sprite(Look::picture(name), 12);
                b.edit(s, |s| {
                    s.x = x as f32;
                    s.y = y as f32;
                    s.turn = turn;
                    s.turn_at = at;
                });
                let mut first = Key::new(4).show(true).scaling((0.0, 0.0), (0.5, 0.5), at);
                if k > 0 {
                    first = first.wait(2 * k as i32);
                }
                out.push((
                    s,
                    vec![
                        first,
                        Key::new(4)
                            .show(false)
                            .fade(255, 0)
                            .scaling((0.5, 0.5), (1.0, 1.0), at),
                    ],
                ));
            }
            out
        }
        // Light over the screen (0 to 4).
        13 => {
            let name = [
                "attack_13_00.akb",
                "attack_13_01.akb",
                "attack_13_02.akb",
                "attack_15_00.akb",
                "attack_15_01.akb",
            ];
            let s = b.sprite(Look::picture(name[arg(0).clamp(0, 4) as usize]), 2);
            b.adding(s);
            if casa {
                return vec![(s, wipes(8, 18))];
            }
            vec![(s, twice(8))]
        }
        // An event picture changing nine times, each over 48 frames. Each
        // is drawn into the background as it ends, so they stay when the
        // attack is taken off. Casablanca: 0 shows all ten pictures, 1 the
        // first five, 2 the last five.
        14 => {
            let mut out = vec![];
            let all = (1..10)
                .map(|k| format!("ev1701_h{k:02}.akb"))
                .chain(std::iter::once("ev1701.akb".to_string()));
            let (skip, count) = match arg(0) {
                _ if !casa => (0, 10),
                1 => (0, 5),
                2 => (5, 5),
                _ => (0, 10),
            };
            let names = all.skip(skip).take(count);
            for (k, name) in names.enumerate() {
                let s = b.sprite(Look::picture(&name), 12 + k as i32);
                let mut key = Key::new(48).show(true).fade(0, 255);
                if k > 0 {
                    key = key.wait(48 * k as i32);
                }
                out.push((s, vec![key]));
                b.st.to_background.push((s, Paint::Picture(name)));
            }
            let holder = b.sprite(Look::Nothing, 0);
            let hold = if count == 10 { 624 } else { 312 };
            out.push((holder, vec![Key::new(hold).show(true)]));
            out
        }
        // A red light after 18 frames, up over 8 frames twice (looping
        // when built to be started later).
        15 => {
            let s = b.sprite(fill(0xffff0000), 1);
            b.adding(s);
            let mut keys = vec![
                Key::new(4).wait(18).show(true).fade(0, 0),
                Key::new(8).show(true).fade(0, 255),
                Key::new(8).show(true).fade(0, 255),
            ];
            if casa {
                keys.truncate(1);
                keys.extend(wipes(8, 18));
            }
            if kept {
                keys.push(Key::jump(0));
            }
            vec![(s, keys)]
        }
        // A blessing somewhere at random growing to half size over 16
        // frames, then going over 4.
        16 => {
            let s = b.sprite(Look::picture("attack_07_00.akb"), 12);
            b.adding(s);
            let x = rand(&mut b.st.rng, 640) - 520;
            let y = rand(&mut b.st.rng, 200) - 350;
            b.edit(s, |s| {
                s.x = x as f32;
                s.y = y as f32;
            });
            let at = (740.0, 610.0);
            vec![(
                s,
                vec![
                    Key::new(16).show(true).scaling((0.0, 0.0), (0.5, 0.5), at),
                    Key::new(4).show(false).fade(255, 0),
                ],
            )]
        }
        _ => vec![],
    }
}

/// Pictures the effects made, kept between frames.
#[derive(Default)]
pub struct Drawn {
    /// Wavering and mixed pictures by sprite: what they were made of, and
    /// the picture.
    made: std::collections::HashMap<u32, (String, std::sync::Arc<Image>)>,
    /// Rule pictures of the wipes.
    rules: std::collections::HashMap<u32, Option<Arc<Vec<u8>>>>,
}

use crate::assets::Assets;
use crate::canvas::Canvas;
use silky_format::akb::Image;
use std::sync::Arc;

impl Drawn {
    /// Draws the effect sprites over `canvas`, lowest `z` first.
    pub fn draw(&mut self, canvas: &mut Canvas, stage: &Stage, assets: &mut Assets) {
        self.draw_where(canvas, stage, assets, |_| true);
    }

    /// Draws only the sprites `pick` takes, lowest `z` first (a game that
    /// puts effects between its characters draws them in several goes).
    pub fn draw_where(
        &mut self,
        canvas: &mut Canvas,
        stage: &Stage,
        assets: &mut Assets,
        pick: impl Fn(&Sprite) -> bool,
    ) {
        self.made
            .retain(|id, _| stage.sprites.iter().any(|s| s.id == *id));
        let mut order: Vec<&Sprite> = stage
            .sprites
            .iter()
            .filter(|s| s.shown && pick(s))
            .collect();
        order.sort_by_key(|s| s.z);
        let mut screen: Option<Arc<Image>> = None;
        for s in order {
            let alpha = s.alpha;
            if alpha == 0 && s.wipe.is_none() {
                continue;
            }
            let (image, part, mut color) = match &s.look {
                Look::Nothing => continue,
                Look::Fill { color, w, h } => (None, (0, 0, *w, *h), *color),
                Look::Picture { name, part } => match assets.image(name) {
                    Ok(img) => {
                        let part = part.unwrap_or((0, 0, img.width, img.height));
                        (Some(img), part, [255; 4])
                    }
                    Err(_) => continue,
                },
                look => {
                    let Some(img) = self.make(s.id, look, canvas, &mut screen, assets) else {
                        continue;
                    };
                    let part = (0, 0, img.width, img.height);
                    (Some(img), part, [255; 4])
                }
            };
            for k in 0..3 {
                color[k] = ((color[k] as u32 * s.tint[k] as u32 + 127) / 255) as u8;
            }
            color[3] = ((color[3] as u32 * alpha as u32 + 127) / 255) as u8;
            let mut part = part;
            let mut corners = corners(s, part.2 as f32, part.3 as f32);
            if let Some((cx, cy, cw, ch)) = s.clip {
                let (sx, sy) = (s.x as i32, s.y as i32);
                let (x0, y0) = (sx.max(cx), sy.max(cy));
                let x1 = (sx + part.2 as i32).min(cx + cw);
                let y1 = (sy + part.3 as i32).min(cy + ch);
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }
                part = (
                    part.0 + (x0 - sx) as u32,
                    part.1 + (y0 - sy) as u32,
                    (x1 - x0) as u32,
                    (y1 - y0) as u32,
                );
                let (fx, fy, fw, fh) = (x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32);
                corners = [[fx, fy], [fx + fw, fy], [fx, fy + fh]];
            }
            match s.wipe {
                Some((rule, t, shown)) => {
                    let from = canvas.clone();
                    canvas.draw_sprite(image.as_ref(), part, corners, color, s.add);
                    let rule = self
                        .rules
                        .entry(rule)
                        .or_insert_with(|| {
                            assets.data(&format!("{rule:02}.bin")).ok().map(Arc::new)
                        })
                        .clone();
                    // Wiping out takes the sprite away in the order a
                    // wipe in would show it.
                    *canvas = if shown {
                        Canvas::blend_edge(&from, canvas, t, rule, 255.0)
                    } else {
                        Canvas::blend_edge(canvas, &from, t, rule, 255.0)
                    };
                }
                None => canvas.draw_sprite(image.as_ref(), part, corners, color, s.add),
            }
        }
    }

    /// The picture of a wavering or mixed look, made again when it changed.
    fn make(
        &mut self,
        id: u32,
        look: &Look,
        canvas: &Canvas,
        screen: &mut Option<Arc<Image>>,
        assets: &mut Assets,
    ) -> Option<Arc<Image>> {
        let key = format!("{look:?}");
        // A look made of the screen changes with it.
        let live = key.contains("Screen");
        if let Some((k, img)) = self.made.get(&id) {
            if *k == key && !live {
                return Some(img.clone());
            }
        }
        let img = Arc::new(picture(look, canvas, screen, assets)?);
        self.made.insert(id, (key, img.clone()));
        Some(img)
    }
}

/// The picture of a look, at its own size.
fn picture(
    look: &Look,
    canvas: &Canvas,
    screen: &mut Option<Arc<Image>>,
    assets: &mut Assets,
) -> Option<Image> {
    Some(match look {
        Look::Nothing => return None,
        Look::Fill { color, w, h } => Image {
            width: *w,
            height: *h,
            rgba: color.repeat((*w * *h) as usize),
        },
        Look::Picture { name, part } => {
            let img = assets.image(name).ok()?;
            match part {
                None => (*img).clone(),
                Some(p) => crate::games::apeiria::holo::cut(&img, *p, 255)
                    .as_ref()
                    .clone(),
            }
        }
        Look::Pieces { name, w, h, pieces } => {
            let src = assets.image(name).ok()?;
            let mut rgba = vec![0u8; (*w * *h) as usize * 4];
            for &((pl, pt, pw, ph), (dx, dy)) in pieces {
                for row in 0..ph as i32 {
                    let (sy, ty) = (pt as i32 + row, dy + row);
                    if sy >= src.height as i32 || ty < 0 || ty >= *h as i32 {
                        continue;
                    }
                    for col in 0..pw as i32 {
                        let (sx, tx) = (pl as i32 + col, dx + col);
                        if sx >= src.width as i32 || tx < 0 || tx >= *w as i32 {
                            continue;
                        }
                        let from = (sy as usize * src.width as usize + sx as usize) * 4;
                        let to = (ty as usize * *w as usize + tx as usize) * 4;
                        rgba[to..to + 4].copy_from_slice(&src.rgba[from..from + 4]);
                    }
                }
            }
            Image {
                width: *w,
                height: *h,
                rgba,
            }
        }
        Look::Screen => {
            let img = screen.get_or_insert_with(|| {
                let c = canvas.rasterize();
                Arc::new(Image {
                    width: c.width,
                    height: c.height,
                    rgba: c.rgba,
                })
            });
            (**img).clone()
        }
        Look::Mix { from, to, mix } => {
            let a = picture(from, canvas, screen, assets)?;
            let b = picture(to, canvas, screen, assets)?;
            if a.width != b.width || a.height != b.height {
                return Some(b);
            }
            let t = *mix as u32;
            let rgba = a
                .rgba
                .iter()
                .zip(&b.rgba)
                .map(|(&x, &y)| ((x as u32 * (255 - t) + y as u32 * t + 127) / 255) as u8)
                .collect();
            Image {
                width: a.width,
                height: a.height,
                rgba,
            }
        }
        Look::Mosaic { source, block } => {
            let mut img = picture(source, canvas, screen, assets)?;
            let (w, h, b) = (
                img.width as usize,
                img.height as usize,
                (*block).max(1) as usize,
            );
            if b > 1 {
                for by in (0..h).step_by(b) {
                    for bx in (0..w).step_by(b) {
                        let (x1, y1) = ((bx + b).min(w), (by + b).min(h));
                        let mut sum = [0u32; 3];
                        for y in by..y1 {
                            for x in bx..x1 {
                                let p = (y * w + x) * 4;
                                for c in 0..3 {
                                    sum[c] += img.rgba[p + c] as u32;
                                }
                            }
                        }
                        let n = ((x1 - bx) * (y1 - by)) as u32;
                        for y in by..y1 {
                            for x in bx..x1 {
                                let p = (y * w + x) * 4;
                                for c in 0..3 {
                                    img.rgba[p + c] = (sum[c] / n) as u8;
                                }
                                img.rgba[p + 3] = 255;
                            }
                        }
                    }
                }
            }
            img
        }
        Look::Bars { bars } => {
            let (w, h) = (W as usize, H as usize);
            let mut rgba = vec![0u8; w * h * 4];
            for &[x, y, bw, bh] in bars {
                for yy in y.max(0)..(y + bh).min(h as i32) {
                    for xx in x.max(0)..(x + bw).min(w as i32) {
                        let p = (yy as usize * w + xx as usize) * 4;
                        rgba[p..p + 4].copy_from_slice(&[255; 4]);
                    }
                }
            }
            Image {
                width: W,
                height: H,
                rgba,
            }
        }
        Look::Reveal {
            source,
            rule,
            threshold,
            erasing,
            stride,
        } => {
            let mut img = picture(source, canvas, screen, assets)?;
            let rule = assets.data(&format!("{rule:02}.bin")).ok()?;
            let thr = *threshold as i32;
            let width = img.width.max(1) as usize;
            for (k, px) in img.rgba.chunks_exact_mut(4).enumerate() {
                let at = match *stride as usize {
                    0 => k,
                    row => (k / width) * row + k % width,
                };
                let r = rule.get(at).copied().unwrap_or(255) as i32;
                let shown = (thr - r).clamp(0, px[3] as i32);
                px[3] = if *erasing {
                    px[3] - shown as u8
                } else {
                    shown as u8
                };
            }
            img
        }
        Look::Raster {
            source,
            amp,
            step,
            phase,
        } => {
            let src = picture(source, canvas, screen, assets)?;
            rastered(&src, *amp, *step, *phase)
        }
    })
}

/// `src` with its rows moved sideways along a sine wave, as
/// [`Look::Raster`] says.
pub fn rastered(src: &Image, amp: f32, step: f32, phase: f32) -> Image {
    let (w, h) = (src.width as usize, src.height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    let mut angle = phase as f64;
    for y in 0..h {
        angle += step as f64;
        let dx = ((angle * 3.141592 / 180.0).sin() * amp as f64) as i64;
        let (to, from) = if dx >= 0 {
            (dx as usize, 0)
        } else {
            (0, (-dx) as usize)
        };
        let n = w.saturating_sub(to.max(from));
        let row = y * w * 4;
        rgba[row + to * 4..row + (to + n) * 4]
            .copy_from_slice(&src.rgba[row + from * 4..row + (from + n) * 4]);
    }
    Image {
        width: src.width,
        height: src.height,
        rgba,
    }
}

/// Top left, top right and bottom left corners of a sprite of `w` x `h`
/// on screen: scaled about `scale_at`, turned about `turn_at`, moved to
/// its place.
fn corners(s: &Sprite, w: f32, h: f32) -> [[f32; 2]; 3] {
    let (sx, sy) = s.scale;
    let (ax, ay) = s.scale_at;
    let (tx, ty) = s.turn_at;
    let (sin, cos) = s.turn.to_radians().sin_cos();
    let at = |px: f32, py: f32| {
        let (qx, qy) = ((px - ax) * sx + ax, (py - ay) * sy + ay);
        let (dx, dy) = (qx - tx, qy - ty);
        [
            dx * cos - dy * sin + tx + s.x,
            dx * sin + dy * cos + ty + s.y,
        ]
    };
    [at(0.0, 0.0), at(w, 0.0), at(0.0, h)]
}

#[cfg(test)]
mod paint_tests {
    use super::super::anim::TICK_MS;
    use super::*;

    fn painted(st: &mut Stage) -> Vec<Paint> {
        st.settle();
        st.take_painted()
    }

    #[test]
    fn spells_paint_the_background_and_leave_no_sprite() {
        let mut st = Stage::default();
        spell(&mut st, 4, &[], &[]);
        assert_eq!(painted(&mut st), [Paint::Fill(0x3b010c)]);
        assert!(st.sprites.is_empty());

        // Spells 0 and 1 leave a cover sprite until the background
        // changes.
        spell(&mut st, 0, &[], &[]);
        assert!(painted(&mut st).is_empty());
        assert_eq!(st.sprites.len(), 1);
        spell(&mut st, 1, &[1, 64, 10], &[]);
        assert!(painted(&mut st).is_empty());
        assert_eq!(st.sprites.len(), 2);
        st.background_changed();
        assert!(st.sprites.is_empty());

        // Coming back from black draws nothing.
        spell(&mut st, 1, &[64, 1, 10], &[]);
        assert!(painted(&mut st).is_empty());
    }

    #[test]
    fn spell_six_paints_its_second_picture() {
        let mut st = Stage::default();
        let strs = ["a.akb".to_string(), "b.akb".to_string()];
        spell(&mut st, 6, &[640, 360, 10], &strs);
        assert_eq!(painted(&mut st), [Paint::Picture("b.akb".into())]);
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn kind_fourteen_paints_each_picture_in_turn() {
        let mut st = Stage::default();
        attack(&mut st, 1, &[1, 14]);
        attack(&mut st, 2, &[1]);
        // Not played yet: nothing painted.
        assert!(st.take_painted().is_empty());
        st.advance(48 * 3 * TICK_MS);
        let first = st.take_painted();
        assert!(!first.is_empty() && first.len() < 10);
        assert_eq!(first[0], Paint::Picture("ev1701_h01.akb".into()));
        st.advance(1000 * TICK_MS);
        let rest = st.take_painted();
        assert_eq!(first.len() + rest.len(), 10);
        assert_eq!(rest.last(), Some(&Paint::Picture("ev1701.akb".into())));
        // Taking the attack off leaves what was painted.
        attack(&mut st, 4, &[]);
        st.settle();
        assert!(st.take_painted().is_empty());
    }

    #[test]
    fn casablanca_kind_fourteen_takes_its_half() {
        let mut st = Stage::default();
        st.casablanca = true;
        attack(&mut st, 1, &[1, 14, 2]);
        attack(&mut st, 2, &[1]);
        st.advance(1000 * TICK_MS);
        let all = st.take_painted();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], Paint::Picture("ev1701_h06.akb".into()));
        assert_eq!(all[4], Paint::Picture("ev1701.akb".into()));
    }

    #[test]
    fn casablanca_pillar_of_fire_drops_in_and_wipes_out() {
        let mut st = Stage::default();
        st.casablanca = true;
        attack(&mut st, 1, &[1, 18]);
        attack(&mut st, 2, &[1]);
        st.advance(6 * TICK_MS);
        let s = &st.sprites[0];
        assert!(s.y < 0.0 && s.shown);
        st.advance(20 * TICK_MS);
        assert!(st.sprites[0].wipe.is_some());
        st.advance(100 * TICK_MS);
        assert!(!st.sprites[0].shown);
    }

    #[test]
    fn only_casablanca_wipes_a_slash() {
        let mut st = Stage::default();
        attack(&mut st, 1, &[1, 1, 0]);
        assert!(st.sets[&(ATTACK, 1)][0].1[0].flags & super::super::anim::flag::WIPE == 0);
        let mut st = Stage::default();
        st.casablanca = true;
        attack(&mut st, 1, &[1, 1, 0]);
        assert!(st.sets[&(ATTACK, 1)][0].1[0].flags & super::super::anim::flag::WIPE != 0);
    }

    fn wipes_now(st: &Stage) -> Vec<(u32, bool)> {
        st.sprites
            .iter()
            .filter_map(|s| s.wipe.map(|(rule, _, shown)| (rule, shown)))
            .collect()
    }

    #[test]
    fn casablanca_screen_effects_wipe_where_apeiria_fades() {
        // The green light: in through rule 18, out through it the second
        // time.
        let mut st = Stage::default();
        st.casablanca = true;
        screen_effect(&mut st, 14, &[], &[]);
        st.advance(6 * TICK_MS);
        assert_eq!(wipes_now(&st), vec![(18, true)]);
        st.advance(20 * TICK_MS);
        assert!(wipes_now(&st).is_empty() && st.sprites[0].shown);
        screen_effect(&mut st, 15, &[], &[]);
        st.advance(6 * TICK_MS);
        assert_eq!(wipes_now(&st), vec![(18, false)]);
        st.advance(20 * TICK_MS);
        assert!(!st.sprites[0].shown);

        let mut st = Stage::default();
        screen_effect(&mut st, 14, &[], &[]);
        st.advance(6 * TICK_MS);
        assert!(wipes_now(&st).is_empty());

        // The white flash wipes in through rule 8, then fades out.
        let mut st = Stage::default();
        st.casablanca = true;
        screen_effect(&mut st, 20, &[], &[]);
        st.advance(10 * TICK_MS);
        assert_eq!(wipes_now(&st), vec![(8, true)]);
        st.advance(200 * TICK_MS);
        assert!(wipes_now(&st).is_empty());

        // The white of the starry sky: in through rule 8, out through 26.
        let mut st = Stage::default();
        st.casablanca = true;
        screen_effect(&mut st, 1, &[], &[]);
        st.advance(10 * TICK_MS);
        assert_eq!(wipes_now(&st), vec![(8, true)]);
        st.advance(70 * TICK_MS);
        assert_eq!(wipes_now(&st), vec![(26, false)]);
        let mut st = Stage::default();
        screen_effect(&mut st, 1, &[], &[]);
        st.advance(10 * TICK_MS);
        assert!(wipes_now(&st).is_empty());
    }
}
