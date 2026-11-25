//! きまぐれテンプテーション's picture effects, played on the sprite stage of
//! 景の海のアペイリア (the same engine build, see
//! [`crate::games::apeiria::anim`]): the strips of text wiped in through a
//! rule picture (`Game.1000`), the picture laid over the screen and faded
//! (`Game.999`), the evidence cards (`Game.47`), and the fades of E-mote
//! models (companion, CG models, the white flash of a standing model).
//!
//! The runtime applies a scene change at once and lets the frontend mix the
//! screen before and after, while the original loads a picture into a back
//! layer and copies it to the front when told. The strips need the front,
//! so [`applied`] notes which picture it holds each time a change is applied.

use std::collections::BTreeMap;

use silky_vm::Flow;

use crate::games::apeiria::anim::{Key, Look, Sprite, Stage, Step};
use crate::games::apeiria::effects::User;
use crate::host::{Pause, Transition};
use crate::scene::{Emote, Scene};

/// Owners of the sprites on the stage (the command numbers, as in the
/// first game's other effects).
const STRIPS: u32 = 1000;
const OVERLAY: u32 = 999;
const EVIDENCE: u32 = 47;
/// Where the picture of the front screen is noted (a hidden sprite).
const FRONT: u32 = 10_001;
/// Trackers of the fade of an E-mote model, by the key it is drawn under
/// (the slot), and the timeline that fades it.
const FADE: u32 = 4800;
const FADE_GROUP: u32 = 4801;

/// A model that is going: it is drawn under a key of its own while its
/// tracker fades it, and goes with the tracker sprite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ghost {
    pub key: i32,
    /// The sprite whose life it lasts for.
    pub sprite: u32,
    pub emote: Emote,
}

/// What the effects keep on a scene besides sprites. Not saved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fx {
    pub ghosts: Vec<Ghost>,
    /// The model a standing slot had before it was loaded again, for the
    /// white flash that may follow.
    pub before: BTreeMap<i32, Emote>,
}

/// Key a ghost of the model drawn under `key` is drawn under.
pub fn ghost_key(key: i32) -> i32 {
    match key {
        i32::MIN => 302,
        k if k >= 100 => 301,
        k => 200 + k,
    }
}

/// A scene change was applied: what the strips drew belongs to the old
/// front, and `background` is the new one.
pub fn applied(st: &mut Stage, background: Option<&str>) {
    st.clear(STRIPS);
    st.clear(FRONT);
    let id = st.id();
    let look = match background {
        Some(name) => Look::picture(name),
        None => Look::Nothing,
    };
    st.play(vec![Step::Add(Sprite::new(id, FRONT, look, 0))]);
}

fn black() -> Look {
    Look::Fill {
        color: [0, 0, 0, 255],
        w: 1280,
        h: 720,
    }
}

/// Plays steps on the stage and puts sprites on a timeline of their own.
struct Play<'a> {
    st: &'a mut Stage,
    steps: Vec<Step>,
}

impl<'a> Play<'a> {
    fn new(st: &'a mut Stage) -> Self {
        Self {
            st,
            steps: Vec::new(),
        }
    }

    fn add(&mut self, s: Sprite) -> u32 {
        let id = s.id;
        self.steps.push(Step::Add(s));
        id
    }

    /// Starts a timeline of sprites with their keys.
    fn start(&mut self, sprites: Vec<(u32, Vec<Key>)>) -> u32 {
        let g = self.st.id();
        let ids: Vec<u32> = sprites.iter().map(|(id, _)| *id).collect();
        for (id, keys) in sprites {
            self.steps.push(Step::Keys(id, keys));
        }
        self.steps.push(Step::Start(g, ids));
        g
    }

    fn wait(&mut self, g: u32) {
        self.steps.push(Step::Wait(g));
    }

    fn remove(&mut self, ids: Vec<u32>) {
        self.steps.push(Step::Remove(ids));
    }

    fn user(&mut self, u: User) -> u32 {
        let id = self.st.id();
        self.st.users.insert(id, u);
        id
    }

    fn done(self) {
        self.st.play(self.steps);
    }
}

/// `Game.1000` (hold, frames, rule, height, width, y, x): the part of the
/// picture just loaded (`background`) that lies in the rectangle is wiped
/// in over the screen through the rule picture, over `frames` frames. The
/// rule is the first `width * height` bytes of the file, a row at a time.
/// The wipe's threshold goes up by 512 / (frames - 1) a frame, to 512, a
/// pixel showing as much as the threshold less its rule value. What was
/// wiped stays on the front until the next scene change. The script waits
/// for the wipe and then for `hold` milliseconds (the original's wait starts
/// that count once the wipe is over); nothing else is waited for. Returns
/// the milliseconds the script waits.
pub fn strip(st: &mut Stage, background: &str, a: [i32; 7]) -> u64 {
    let [hold, frames, rule, h, w, y, x] = a;
    let frames = frames.max(1);
    let (w, h) = (w.max(0) as u32, h.max(0) as u32);
    let mut p = Play::new(st);
    // The front: the picture the last change put up, under the strips.
    let cover = p.st.slots.get(&(STRIPS, 0)).copied();
    if cover.is_none_or(|id| !p.st.sprites.iter().any(|s| s.id == id)) {
        let front =
            p.st.sprites
                .iter()
                .find(|s| s.owner == FRONT)
                .map(|s| s.look.clone());
        let look = match front {
            Some(Look::Nothing) | None => black(),
            Some(l) => l,
        };
        let id = p.st.id();
        p.st.slots.insert((STRIPS, 0), id);
        p.add(Sprite::new(id, STRIPS, look, 0).shown());
    }
    let z = 1 + p.st.sprites.iter().filter(|s| s.owner == STRIPS).count() as i32;
    let id = p.st.id();
    let look = Look::Reveal {
        source: Box::new(Look::part(
            background,
            (x.max(0) as u32, y.max(0) as u32, w, h),
        )),
        rule: rule.max(0) as u32,
        threshold: 0.0,
        erasing: false,
        stride: 0,
    };
    p.add(Sprite::new(id, STRIPS, look, z).at(x as f32, y as f32));
    let step = if frames > 1 {
        512.0 / (frames - 1) as f32
    } else {
        0.0
    };
    let u = p.user(User::Ramp {
        owner: STRIPS,
        sprite: id,
        step,
    });
    let g = p.start(vec![(id, vec![Key::new(frames).show(true).user(u)])]);
    p.wait(g);
    p.done();
    st.blocking_ms().unwrap_or(0) + hold.max(0) as u64
}

/// `Game.999`: a picture over the screen. 0 takes the picture on screen
/// (`background`) and fades it in over 8 frames, 1 fades it out over 8 and
/// puts it away, 2 shows it at once, 3 puts it away at once.
pub fn overlay(st: &mut Stage, sub: i32, background: Option<&str>) {
    const FRAMES: i32 = 8;
    let sprite = |st: &mut Stage, name: &str| {
        let id = st.id();
        Sprite::new(id, OVERLAY, Look::picture(name), 900)
    };
    match sub {
        0 | 2 => {
            st.clear(OVERLAY);
            let Some(name) = background else {
                return;
            };
            let s = sprite(st, name);
            let mut p = Play::new(st);
            if sub == 2 {
                p.add(s.shown());
            } else {
                let id = p.add(s);
                let g = p.start(vec![(id, vec![Key::new(FRAMES).show(true).fade(0, 255)])]);
                p.wait(g);
            }
            p.done();
        }
        1 => {
            let ids: Vec<u32> = st
                .sprites
                .iter()
                .filter(|s| s.owner == OVERLAY)
                .map(|s| s.id)
                .collect();
            if ids.is_empty() {
                return;
            }
            let mut p = Play::new(st);
            let keys = vec![Key::new(FRAMES).show(false).fade(255, 0)];
            let g = p.start(ids.iter().map(|&i| (i, keys.clone())).collect());
            p.wait(g);
            p.remove(ids);
            p.done();
        }
        _ => st.clear(OVERLAY),
    }
}

/// Cell `n` of the evidence sheet: three to a row from 624 down.
fn card(n: i32) -> Look {
    const SIZE: i32 = 410;
    let n = n.max(0);
    Look::part(
        "pursue_p.akb",
        (
            (n % 3 * SIZE) as u32,
            (n / 3 * SIZE + 0x270) as u32,
            SIZE as u32,
            SIZE as u32,
        ),
    )
}

/// `Game.47`: evidence cards over the screen. Sub 0 (card) shows one at
/// (435, 80); sub 1 (first, second) two side by side, the second at 210 and
/// the first at 640. They fade in over 12 frames, stay 128 and fade out over
/// 12, the script waiting all along.
pub fn evidence(st: &mut Stage, cards: &[i32]) {
    let places: Vec<(i32, i32)> = match cards {
        [one] => vec![(*one, 435)],
        [first, second] => vec![(*second, 210), (*first, 640)],
        _ => return,
    };
    st.clear(EVIDENCE);
    let mut p = Play::new(st);
    let mut ids = Vec::new();
    for (n, x) in places {
        let id = p.st.id();
        p.add(Sprite::new(id, EVIDENCE, card(n), 800).at(x as f32, 80.0));
        ids.push(id);
    }
    let keys = vec![
        Key::new(12).show(true).fade(0, 255),
        Key::new(12).wait(128).show(false).fade(255, 0),
    ];
    let g = p.start(ids.iter().map(|&i| (i, keys.clone())).collect());
    p.wait(g);
    p.remove(ids);
    p.done();
}

/// How the model under `key` is drawn now: its opacity (0 to 255) and its
/// colour byte (0x80 leaves the model as it is, 0xff doubles its colours).
/// `None` when nothing changes it.
pub fn emote_fx(scene: &Scene, key: i32) -> Option<(u8, u8)> {
    let id = scene.effects.slots.get(&(FADE, key as u32))?;
    let s = scene.effects.sprites.iter().find(|s| s.id == *id)?;
    Some((s.alpha, s.tint[0]))
}

/// The colour byte of a model that is not flashed.
const NEUTRAL: u8 = 0x80;

/// How many pixels the model under `key` is moved sideways by its tracker
/// (a standing model that leaves slides while it fades).
pub fn emote_shift(scene: &Scene, key: i32) -> i32 {
    scene
        .effects
        .slots
        .get(&(FADE, key as u32))
        .and_then(|id| scene.effects.sprites.iter().find(|s| s.id == *id))
        .map_or(0, |s| s.x.round() as i32)
}

/// The models that are going, with the keys they are drawn under, those
/// whose tracker is still there.
pub fn ghosts(scene: &Scene) -> impl Iterator<Item = (i32, &Emote)> {
    scene
        .k1fx
        .ghosts
        .iter()
        .filter(|g| {
            scene
                .effects
                .sprites
                .iter()
                .any(|s| s.id == g.sprite && s.busy())
        })
        .map(|g| (g.key, &g.emote))
}

/// What a fade of a model does to its tracker sprite.
pub enum Fade {
    /// Opacity from 0 to 255 over the frames.
    In(i32),
    /// Opacity from 255 to 0 over the frames, then it is gone.
    Out(i32),
    /// The white flash of the model that goes (`Game.40` sub 100): whiter
    /// and whiter over the frames, then fading out over as many.
    WhiteOut(i32),
    /// The white flash of the model that comes: it fades in white over the
    /// frames and loses its white over as many.
    WhiteIn(i32),
    /// Fully there for the frames, then gone (a model that stays under
    /// the one fading in over it).
    Hold(i32),
    /// Fades out over the frames while sliding that many pixels sideways
    /// (a standing model that leaves).
    Leave(i32, i32),
}

/// Puts a tracker on the model drawn under `key`, replacing the one it had.
/// Returns the timeline, which the script can wait for ([`wait_for`]).
pub fn fade(st: &mut Stage, key: i32, how: Fade) -> u32 {
    let mut p = Play::new(st);
    if let Some(old) = p.st.slots.get(&(FADE, key as u32)).copied() {
        p.remove(vec![old]);
    }
    let id = p.st.id();
    let mut s = Sprite::new(id, FADE, Look::Nothing, 0);
    // The tracker's tint is the model's colour byte.
    s.tint = [NEUTRAL; 3];
    let tint = |p: &mut Play, owner: u32, from: u8, to: u8| {
        p.user(User::Tint {
            owner,
            sprite: id,
            from,
            to,
        })
    };
    let keys = match how {
        Fade::In(n) => {
            s.alpha = 0;
            vec![Key::new(n).show(true).fade(0, 255)]
        }
        Fade::Hold(n) => {
            s = s.shown();
            vec![Key::new(n).show(true)]
        }
        Fade::Out(n) => {
            s = s.shown();
            vec![Key::new(n).show(false).fade(255, 0)]
        }
        Fade::Leave(n, dx) => {
            s = s.shown();
            vec![Key::new(n).show(false).fade(255, 0).moving((0, 0), (dx, 0))]
        }
        Fade::WhiteOut(n) => {
            s = s.shown();
            let u = tint(&mut p, FADE, NEUTRAL, 255);
            vec![
                Key::new(n).show(true).user(u),
                Key::new(n).show(false).fade(255, 0),
            ]
        }
        Fade::WhiteIn(n) => {
            s.alpha = 0;
            s.tint = [255; 3];
            let u = tint(&mut p, FADE, 255, NEUTRAL);
            vec![
                Key::new(n).show(true).fade(0, 255),
                Key::new(n).show(true).user(u),
            ]
        }
    };
    p.st.slots.insert((FADE, key as u32), id);
    p.add(s);
    let g = p.start(vec![(id, keys)]);
    p.st.slots.insert((FADE_GROUP, key as u32), g);
    p.done();
    g
}

/// Takes the tracker of the model under `key` away.
pub fn drop_fade(st: &mut Stage, key: i32) {
    if let Some(id) = st.slots.remove(&(FADE, key as u32)) {
        st.sprites.retain(|s| s.id != id);
    }
    st.slots.remove(&(FADE_GROUP, key as u32));
}

/// Makes the script wait for the fade of the model under `key`.
pub fn wait_for(st: &mut Stage, key: i32) {
    if let Some(g) = st.slots.get(&(FADE_GROUP, key as u32)).copied() {
        st.play(vec![Step::Wait(g)]);
    }
}

/// Makes the script wait for a timeline.
pub fn wait_group(st: &mut Stage, g: u32) {
    st.play(vec![Step::Wait(g)]);
}

/// Keeps `emote` on screen under a key of its own while the tracker made
/// by the next [`fade`] on that key lasts; returns that key.
pub fn leave(scene: &mut Scene, key: i32, emote: Emote) -> i32 {
    let gk = ghost_key(key);
    scene.k1fx.ghosts.retain(|g| g.key != gk);
    drop_fade(&mut scene.effects, gk);
    scene.k1fx.ghosts.push(Ghost {
        key: gk,
        sprite: 0,
        emote,
    });
    gk
}

/// The ghost of `key`'s tracker is the one `fade` made; points the ghost at
/// it so that it goes with it.
pub fn bind(scene: &mut Scene, ghost: i32) {
    if let Some(id) = scene.effects.slots.get(&(FADE, ghost as u32)).copied() {
        if let Some(g) = scene.k1fx.ghosts.iter_mut().find(|g| g.key == ghost) {
            g.sprite = id;
        }
    }
}

/// Takes away the ghosts whose trackers have ended, and the trackers of
/// models that are gone.
pub fn sweep(scene: &mut Scene) {
    let busy =
        |scene: &Scene, id: u32| scene.effects.sprites.iter().any(|s| s.id == id && s.busy());
    let gone: Vec<i32> = scene
        .k1fx
        .ghosts
        .iter()
        .filter(|g| !busy(scene, g.sprite))
        .map(|g| g.key)
        .collect();
    for k in gone {
        scene.k1fx.ghosts.retain(|g| g.key != k);
        drop_fade(&mut scene.effects, k);
    }
    let stale: Vec<i32> = scene
        .effects
        .slots
        .keys()
        .filter(|(o, _)| *o == FADE)
        .map(|(_, k)| *k as i32)
        .filter(|k| {
            !scene.k1fx.ghosts.iter().any(|g| g.key == *k)
                && if *k == i32::MIN {
                    scene.emote.is_none()
                } else {
                    !scene.emotes.contains_key(k)
                }
        })
        .collect();
    for k in stale {
        drop_fade(&mut scene.effects, k);
    }
}

impl crate::host::Runtime {
    /// Makes the script wait for what the stage was just given to wait
    /// for, `extra` milliseconds more.
    pub(crate) fn k1_block(&mut self, extra: u64) {
        let ms = self.scene.effects.blocking_ms().unwrap_or(0) + extra;
        if ms > 0 {
            self.pause = Some(Pause::Transition(Transition {
                ms: ms as u32,
                rule: None,
                spin: None,
                live: true,
            }));
        }
    }

    /// Ends a call that may have set a pause (see [`Self::k1_block`]).
    pub(crate) fn k1_done(&mut self) -> Option<silky_vm::Result<Flow>> {
        self.handled += 1;
        Some(Ok(if self.pause.is_some() {
            Flow::Yield
        } else {
            Flow::Continue
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::frames_ms;

    fn threshold(st: &Stage) -> f32 {
        st.sprites
            .iter()
            .filter(|s| s.owner == STRIPS)
            .find_map(|s| match &s.look {
                Look::Reveal { threshold, .. } => Some(*threshold),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn strip_ramps_to_the_full_range_and_stays() {
        let mut st = Stage::default();
        applied(&mut st, Some("front"));
        let ms = strip(&mut st, "back", [100, 33, 300, 42, 1280, 140, 0]);
        // The wipe and then the hold, nothing more.
        assert!(ms + 16 >= frames_ms(33) as u64 + 100, "{ms}");
        assert!(ms < frames_ms(33) as u64 + 100 + 40, "{ms}");
        st.advance(16 * 16);
        let mid = threshold(&st);
        assert!(mid > 200.0 && mid < 300.0, "{mid}");
        st.advance(2000);
        assert_eq!(threshold(&st), 512.0);
        // The cover and the strip are still there.
        assert_eq!(st.sprites.iter().filter(|s| s.owner == STRIPS).count(), 2);
        // The next scene change takes them.
        applied(&mut st, Some("back"));
        assert_eq!(st.sprites.iter().filter(|s| s.owner == STRIPS).count(), 0);
    }

    #[test]
    fn overlay_fades_in_and_out() {
        let mut st = Stage::default();
        overlay(&mut st, 0, Some("pic"));
        assert!(st.blocking_ms().unwrap() >= 8 * 1000 / 60 - 16);
        st.settle();
        let s = st.sprites.iter().find(|s| s.owner == OVERLAY).unwrap();
        assert!(s.shown && s.alpha == 255);
        overlay(&mut st, 1, None);
        assert!(st.blocking_ms().unwrap() > 0);
        st.settle();
        assert!(st.sprites.iter().all(|s| s.owner != OVERLAY));
    }

    #[test]
    fn evidence_takes_its_time() {
        let mut st = Stage::default();
        evidence(&mut st, &[1, 2]);
        let ms = st.blocking_ms().unwrap() as i64;
        assert!((2400..=2700).contains(&ms), "{ms}");
        assert_eq!(st.sprites.iter().filter(|s| s.owner == EVIDENCE).count(), 2);
        st.settle();
        assert!(st.sprites.iter().all(|s| s.owner != EVIDENCE));
    }

    #[test]
    fn leaving_model_slides_while_it_fades() {
        let mut scene = Scene::default();
        let gk = ghost_key(1);
        fade(&mut scene.effects, gk, Fade::Leave(16, -48));
        scene.effects.advance(frames_ms(8) as u64);
        let shift = emote_shift(&scene, gk);
        assert!((-40..=-8).contains(&shift), "{shift}");
        scene.effects.advance(2000);
        assert_eq!(emote_shift(&scene, gk), -48);
    }

    #[test]
    fn white_out_raises_the_colour_byte_from_0x80_to_0xff() {
        let mut scene = Scene::default();
        fade(&mut scene.effects, 0, Fade::WhiteOut(16));
        assert_eq!(emote_fx(&scene, 0), Some((255, NEUTRAL)));
        scene.effects.advance(frames_ms(8) as u64);
        let (_, byte) = emote_fx(&scene, 0).unwrap();
        assert!((185..=197).contains(&byte), "{byte}");
        scene.effects.advance(frames_ms(8) as u64);
        // Then it fades out, fully white.
        let (alpha, byte) = emote_fx(&scene, 0).unwrap();
        assert!(alpha < 255, "{alpha}");
        assert_eq!(byte, 255);
    }

    #[test]
    fn white_flash_values() {
        let mut scene = Scene::default();
        fade(&mut scene.effects, 0, Fade::WhiteIn(16));
        assert_eq!(emote_fx(&scene, 0), Some((0, 255)));
        scene.effects.advance(2000);
        assert_eq!(emote_fx(&scene, 0), Some((255, NEUTRAL)));
        // A model going leaves a ghost that ends with its fade.
        leave(&mut scene, 0, Emote::default());
        let gk = ghost_key(0);
        fade(&mut scene.effects, gk, Fade::Out(8));
        bind(&mut scene, gk);
        assert_eq!(ghosts(&scene).count(), 1);
        scene.effects.advance(1000);
        sweep(&mut scene);
        assert_eq!(ghosts(&scene).count(), 0);
        assert!(emote_fx(&scene, gk).is_none());
    }
}
