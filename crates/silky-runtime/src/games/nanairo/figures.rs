//! What the standing characters of なないろリンカネーション do besides
//! standing: the order they overlap in (`Game.13.6`), the hops, sways and
//! bows (`13.10` to `13.12`), the slide to another place (`13.8`), and the
//! glow and petals around them (`13.13`, `13.14`, `13.15`).
//!
//! The motions are set up by their calls and run when the scene is
//! committed by `13.2`, which waits until they are over; each frame of
//! 16 ms moves the figure to the next offset of a short table.

use std::collections::{BTreeMap, BTreeSet};

use crate::games::apeiria::anim::{Key, Look, Sprite, Stage, Step};

use super::effects::{scaled, Fx};

/// Owner of the glow sprites on the effect stage.
pub(super) const FX: u32 = 13;
/// Glow sprites below this depth are drawn under the standing characters,
/// the others over them.
const UNDER: i32 = 5;
/// The dark layer behind the glow, the glow, and the petals over all.
const Z_SHADE: i32 = 0;
const Z_GLOW: i32 = 1;
const Z_PETALS: i32 = 8;
/// Frames the glow takes to come and to go.
const FADE_FRAMES: i32 = 24;
/// The dark layer: black at three quarters of full strength, the screen
/// big.
const SHADE: [u8; 4] = [0, 0, 0, 0xc0];
const SCREEN: (u32, u32) = (1280, 720);
/// Glow of the first kind (green light with sparks) and petals.
const GLOW_PICTURE: &str = "teffect.akb";
const PETALS_PICTURE: &str = "tai-effect.akb";
/// The movie that plays for the petals when movies are on.
pub(super) const PETALS_MOVIE: &str = "HG060002_eff.mov";
/// Frames a new face takes to come in over the picture shown: the blend
/// gains 22 of 255 at each frame, so it is whole at the twelfth.
pub(super) const FACE_FRAMES: i32 = 12;
/// Frames a slide to another place takes.
const SLIDE_FRAMES: i32 = 12;

/// Offsets of the 13.10 hop: up and back down over 12 frames.
const HOP: [i32; 12] = [-40, -60, -70, -75, -77, -78, -77, -75, -70, -60, -40, 0];
/// Offsets of the 13.11 bow: down and back up over 12 frames.
const BOW: [i32; 12] = [13, 26, 39, 52, 65, 78, 65, 52, 39, 26, 13, 0];

/// A motion that starts when the scene is committed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Pending {
    /// 13.10 (0) or 13.11 (1).
    bow: Option<i32>,
    /// 13.12: axis and amount in pixels.
    hop: Option<(i32, i32)>,
    /// 13.8: how far the figure was from where it now stands.
    slide: Option<(i32, i32)>,
}

/// What the figures have been told to do.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Figures {
    /// Side code of a slot set by 13.6; higher is in front.
    sides: BTreeMap<i32, i32>,
    /// Place a slot's figure stands at when it is not its own (13.8).
    places: BTreeMap<i32, i32>,
    #[serde(skip)]
    pending: BTreeMap<i32, Pending>,
    /// Slots whose figure was put (13.0) since the last commit.
    #[serde(skip)]
    placed: BTreeSet<i32>,
}

impl Figures {
    /// Depth of a slot's figure when 13.6 changed it: the side code plus
    /// one, the same scale the default order uses (centre 3, left 2,
    /// right 1).
    pub fn depth(&self, slot: i32) -> Option<i32> {
        self.sides.get(&slot).map(|side| side + 1)
    }

    /// 13.6: the figure in a slot goes to a side code.
    pub fn set_side(&mut self, slot: i32, side: i32) {
        self.sides.insert(slot, side);
    }

    /// The place a slot's figure stands at.
    pub fn place(&self, slot: i32) -> i32 {
        self.places.get(&slot).copied().unwrap_or(slot)
    }

    pub fn set_place(&mut self, slot: i32, place: i32) {
        if place == slot {
            self.places.remove(&slot);
        } else {
            self.places.insert(slot, place);
        }
    }

    /// A new figure stands in the slot, or the old one went: it has its
    /// own order and place again.
    pub fn forget(&mut self, slot: i32) {
        self.sides.remove(&slot);
        self.places.remove(&slot);
        self.pending.remove(&slot);
        self.placed.remove(&slot);
    }

    pub fn forget_all(&mut self) {
        *self = Self::default();
    }

    /// 13.0: a figure was put in the slot; its face comes in with it.
    pub fn placed(&mut self, slot: i32) {
        self.placed.insert(slot);
    }

    /// Frames a face set by 13.5 takes to fade in. A figure that stands
    /// on the screen blends the new face over what it shows for 12
    /// frames, the first face and a changed one alike. A figure put in
    /// the same commit fades in whole with its body, and a figure with no
    /// picture has nothing to fade.
    pub fn face_frames(&self, slot: i32, standing: bool) -> i32 {
        if standing && !self.placed.contains(&slot) {
            FACE_FRAMES
        } else {
            0
        }
    }

    /// 13.10 and 13.11.
    pub fn bow(&mut self, slot: i32, kind: i32) {
        self.pending.entry(slot).or_default().bow = Some(kind);
    }

    /// 13.12.
    pub fn hop(&mut self, slot: i32, axis: i32, amount: i32) {
        self.pending.entry(slot).or_default().hop = Some((axis, amount));
    }

    /// 13.8: the figure now stands `shift` pixels from where it was.
    pub fn slide(&mut self, slot: i32, shift: (i32, i32)) {
        self.pending.entry(slot).or_default().slide = Some(shift);
    }

    /// The motions to play now, by slot: an offset for each frame. The
    /// last one is back at rest.
    pub fn start(&mut self) -> Vec<(i32, Vec<(i32, i32)>)> {
        self.placed.clear();
        std::mem::take(&mut self.pending)
            .into_iter()
            .map(|(slot, p)| (slot, steps(&p)))
            .filter(|(_, s)| !s.is_empty())
            .collect()
    }
}

/// Offsets for each frame of a figure's motions. A hop or sway takes over
/// from a bow while it lasts; a slide adds to either.
fn steps(p: &Pending) -> Vec<(i32, i32)> {
    let bow: Vec<(i32, i32)> = match p.bow {
        Some(0) => HOP.iter().map(|&y| (0, y)).collect(),
        Some(_) => BOW.iter().map(|&y| (0, y)).collect(),
        None => Vec::new(),
    };
    let hop: Vec<(i32, i32)> = match p.hop {
        Some((0, t)) if t != 0 => vec![(0, -(t / 2)), (0, -t), (0, -(t / 2)), (0, 0)],
        Some((_, t)) if t != 0 => vec![(-t, 0), (t, 0), (0, 0)],
        _ => Vec::new(),
    };
    let slide: Vec<(i32, i32)> = match p.slide {
        Some((dx, dy)) if (dx, dy) != (0, 0) => (1..=SLIDE_FRAMES)
            .map(|i| {
                let rest = SLIDE_FRAMES - i;
                (dx * rest / SLIDE_FRAMES, dy * rest / SLIDE_FRAMES)
            })
            .collect(),
        _ => Vec::new(),
    };
    let n = bow.len().max(hop.len()).max(slide.len());
    (0..n)
        .map(|i| {
            let own = hop.get(i).or_else(|| bow.get(i)).copied().unwrap_or((0, 0));
            let s = slide.get(i).copied().unwrap_or((0, 0));
            (own.0 + s.0, own.1 + s.1)
        })
        .collect()
}

/// Whether a glow sprite lies under the standing characters.
pub fn is_under(s: &crate::games::apeiria::anim::Sprite) -> bool {
    s.owner == FX && s.z < UNDER
}

/// Takes the glow away at once (13.14).
pub fn detach(st: &mut Stage) {
    super::effects::clear(st, FX);
}

/// Starts the glow (13.13 with its second value 1, and 13.15): the screen
/// behind the figures darkens, and a picture fades in over 24 frames for
/// each of `parts`, a kind, a slot and the size of the figure there. Kind
/// 0 is a green glow behind the figure, squeezed to the figure's size;
/// kind 1 is petals over the whole screen.
///
/// Approximate: the curve of the fades is not known (linear is used). The
/// original plays a movie for the petals when movies are on (see
/// [`petals_movie`]).
pub fn attach(st: &mut Stage, parts: &[(i32, i32, i32)], factor: f32) {
    detach(st);
    let mut fx = Fx::new(st, FX);
    let frames = scaled(FADE_FRAMES, factor);
    let shade = fx.sprite(
        0,
        Look::Fill {
            color: SHADE,
            w: SCREEN.0,
            h: SCREEN.1,
        },
        Z_SHADE,
    );
    let mut sprites = vec![shade];
    for (n, &(kind, slot, size)) in parts.iter().enumerate() {
        let picture = match kind {
            0 => (GLOW_PICTURE, Z_GLOW),
            1 => (PETALS_PICTURE, Z_PETALS),
            _ => continue,
        };
        let id = fx.sprite(1 + n as u32, Look::picture(picture.0), picture.1);
        sprites.push(id);
        if let (0, Some(Step::Add(sprite))) = (kind, fx.steps.last_mut()) {
            glow_place(sprite, slot, size);
        }
    }
    let keys = sprites
        .into_iter()
        .map(|id| (id, vec![Key::new(frames).show(true).fade(0, 255)]))
        .collect();
    fx.play(keys, false);
    fx.run();
}

/// With the movies on, the petals are the movie's picture (its green is
/// the opacity of white) instead of the still picture.
pub fn petals_movie(st: &mut Stage, name: &str) {
    for s in st.sprites.iter_mut().filter(|s| s.owner == FX) {
        if matches!(&s.look, Look::Picture { name: n, .. } if n == PETALS_PICTURE) {
            s.look = super::video::video(name, super::video::GREEN_AS_OPACITY, super::video::WHITE);
        }
    }
}

/// Where the glow picture lies: its width is the figure's size, kept
/// about the middle of the picture, and it is nudged to the figure's slot.
fn glow_place(sprite: &mut Sprite, slot: i32, size: i32) {
    let s = size as f32 / 100.0;
    sprite.scale = (s, 1.0);
    sprite.scale_at = (640.0, 360.0);
    sprite.x = match slot {
        0 => 80.0 * s,
        2 => 376.0,
        _ => 0.0,
    };
}

/// Lets the glow go (13.13 with its third value 1): everything of it
/// fades out over 24 frames. It stays on the stage, hidden, until 13.14.
pub fn release(st: &mut Stage, factor: f32) {
    let mut fx = Fx::new(st, FX);
    let frames = scaled(FADE_FRAMES, factor);
    let ids: Vec<u32> = (0..4).filter_map(|slot| fx.alive(slot)).collect();
    if ids.is_empty() {
        return;
    }
    let keys = ids
        .into_iter()
        .map(|id| {
            let from = fx.alpha(id);
            (id, vec![Key::new(frames).show(false).fade(from, 0)])
        })
        .collect();
    fx.play(keys, false);
    fx.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hop_goes_up_by_halves_and_comes_back() {
        let mut f = Figures::default();
        f.hop(0, 0, 14);
        assert_eq!(
            f.start(),
            vec![(0, vec![(0, -7), (0, -14), (0, -7), (0, 0)])]
        );
        // Nothing is left to start.
        assert!(f.start().is_empty());
    }

    #[test]
    fn sway_goes_left_then_right() {
        let mut f = Figures::default();
        f.hop(2, 1, 14);
        assert_eq!(f.start(), vec![(2, vec![(-14, 0), (14, 0), (0, 0)])]);
    }

    #[test]
    fn a_motion_of_nothing_is_not_started() {
        let mut f = Figures::default();
        f.hop(1, 0, 0);
        assert!(f.start().is_empty());
    }

    #[test]
    fn bow_dips_78_pixels_over_twelve_frames() {
        let mut f = Figures::default();
        f.bow(1, 1);
        let s = &f.start()[0].1;
        assert_eq!(s.len(), 12);
        assert_eq!(s[0], (0, 13));
        assert_eq!(s[5], (0, 78));
        assert_eq!(s[11], (0, 0));
        f.bow(1, 0);
        let s = &f.start()[0].1;
        assert_eq!(s[5], (0, -78));
    }

    #[test]
    fn a_hop_takes_over_from_a_bow_while_it_lasts() {
        let p = Pending {
            bow: Some(1),
            hop: Some((1, 10)),
            slide: None,
        };
        let s = steps(&p);
        assert_eq!(s.len(), 12);
        assert_eq!(&s[..3], &[(-10, 0), (10, 0), (0, 0)]);
        assert_eq!(s[3], (0, 52));
    }

    #[test]
    fn slide_ends_where_the_figure_now_stands() {
        let mut f = Figures::default();
        f.slide(0, (345, 0));
        let s = &f.start()[0].1;
        assert_eq!(s.len(), 12);
        assert_eq!(s[0], (345 * 11 / 12, 0));
        assert_eq!(s[11], (0, 0));
    }

    #[test]
    fn a_face_on_a_standing_figure_fades_over_twelve_frames() {
        let mut f = Figures::default();
        assert_eq!(f.face_frames(0, true), 12);
        assert_eq!(f.face_frames(0, false), 0);
        // A figure put in the same commit comes in whole with its body.
        f.placed(0);
        assert_eq!(f.face_frames(0, true), 0);
        assert_eq!(f.face_frames(1, true), 12);
        f.start();
        assert_eq!(f.face_frames(0, true), 12);
        f.placed(0);
        f.forget(0);
        assert_eq!(f.face_frames(0, true), 12);
    }

    #[test]
    fn side_decides_the_depth() {
        let mut f = Figures::default();
        assert_eq!(f.depth(1), None);
        f.set_side(1, 3);
        assert_eq!(f.depth(1), Some(4));
        f.forget(1);
        assert_eq!(f.depth(1), None);
    }

    #[test]
    fn glow_comes_in_under_the_figures_and_petals_over() {
        let mut st = Stage::default();
        attach(&mut st, &[(0, 0, 50)], 1.0);
        st.settle();
        let under: Vec<_> = st.sprites.iter().filter(|s| is_under(s)).collect();
        assert_eq!(under.len(), 2);
        let glow = st.sprites.iter().find(|s| s.z == Z_GLOW).unwrap();
        assert_eq!(glow.scale, (0.5, 1.0));
        assert_eq!(glow.x, 40.0);
        attach(&mut st, &[(1, 0, 50)], 1.0);
        st.settle();
        assert_eq!(st.sprites.iter().filter(|s| is_under(s)).count(), 1);
        assert!(st.sprites.iter().any(|s| s.z == Z_PETALS));
        detach(&mut st);
        assert!(st.sprites.is_empty());
    }

    #[test]
    fn released_glow_fades_to_nothing() {
        let mut st = Stage::default();
        attach(&mut st, &[(0, 2, 50)], 1.0);
        st.settle();
        st.advance(1000);
        assert!(st.sprites.iter().all(|s| s.alpha == 255 && s.shown));
        release(&mut st, 1.0);
        st.advance(1000);
        assert!(st.sprites.iter().all(|s| s.alpha == 0 && !s.shown));
        assert_eq!(st.sprites.iter().find(|s| s.z == Z_GLOW).unwrap().x, 376.0);
    }
}
