//! Apeiria on a small hologram panel (`Game.1013`), read from the
//! executable: the top or bottom half of `sld_mini.akb` as the panel and,
//! over it, a window of her standing picture at half size. Her coming
//! and going are played as the executable does: she wipes in through
//! rule 47 among rising specks of light (sub 2, way 0) or fades in (way
//! 2) over 12 frames, and fades out with the panel (sub 3).

use std::sync::Arc;

use silky_format::akb::Image;

/// State of the panel, kept with the scene so saves keep it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Holo {
    /// Body picture (a 2048 x 2048 canvas, as for standing characters).
    pub body: String,
    /// Expressions and cheeks drawn over the body, in order (sub 1 draws
    /// each over what is there).
    pub faces: Vec<String>,
    /// Place 0 centre, 1 left, 2 right.
    pub place: i32,
    /// Size in percent (50 in the scripts).
    pub scale: i32,
    /// Panel picture: top (0) or bottom (1) half of `sld_mini.akb`; on
    /// the top half she is drawn at 230 / 255 opacity (sub 6).
    pub panel: i32,
    /// Shift of her picture (sub 4).
    pub offset: (i32, i32),
    /// Shown (sub 2) until removed (subs 3 and 5).
    pub shown: bool,
    /// A fade under way.
    pub fade: Option<Fade>,
    /// Specks of light while she comes (sub 2, way 0).
    pub lights: Option<Lights>,
}

/// Her picture fading in, or her picture and the panel fading out: the
/// opacity goes in a straight line from `from` to `to` over `frames`
/// frames of [`TICK_MS`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Fade {
    pub from: u8,
    pub to: u8,
    pub frames: i32,
    /// The panel fades too.
    pub panel: bool,
    /// She wipes in through this rule picture (`NN.bin`) instead of
    /// fading.
    pub rule: Option<u32>,
    /// Time since the start.
    pub ms: u64,
}

/// Length of a frame of the engine's sprite animation.
pub const TICK_MS: u64 = super::anim::TICK_MS;

/// Frames of her coming (sub 2, ways 0 and 2).
pub const FADE_IN_FRAMES: i32 = 12;
/// Rule picture of her wiping in (way 0).
pub const WIPE_RULE: u32 = 47;
/// The light speck picture, 32 x 32, added to what is under it.
pub const LIGHT: &str = "light.akb";
const LIGHT_COUNT: usize = 50;
/// Frames a speck takes from its start to its end.
const LIGHT_FRAMES: i32 = 4;

/// Specks of light rising from the bottom of the screen into the panel
/// while she comes (`CMiniLightAnime`), stepped once a frame for
/// [`FADE_IN_FRAMES`] frames. Each speck goes from (x, 720) to its end
/// over 4 frames, fading out as it goes, and starts again. The first
/// ones end 16 pixels about the middle at y 549, after a wait of up to 6
/// frames, at 25 to 49 % size; the later ones end anywhere in a band as
/// wide as the panel is times how far the specks are (up to 510 pixels)
/// and between y 154 and 483, after up to one frame. None starts in the
/// last 4 frames.
///
/// The αMAIL panel's specks (`Game.1012` sub 1) work the same way with
/// other numbers ([`Lights::mail`]): 64 frames, each speck 8 frames
/// long, from (640, 720); the first ones end at y 259, the later ones
/// in a band up to 880 pixels wide between y 20 and 479, after up to 6
/// frames each time; none starts in the last 8 frames.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Lights {
    pub specks: Vec<Light>,
    /// Frames stepped.
    pub frame: i32,
    /// Middle of her picture (x).
    pub middle: i32,
    pub rng: u32,
    /// Time not yet stepped.
    pub carry: u64,
    /// The αMAIL panel's specks.
    pub mail: bool,
}

/// The numbers of a kind of [`Lights`].
struct Spec {
    frames: i32,
    speck: i32,
    first_y: i32,
    band: (i32, i32),
    width: f32,
    delay: i32,
}

const HOLO_SPEC: Spec = Spec {
    frames: FADE_IN_FRAMES,
    speck: LIGHT_FRAMES,
    first_y: 549,
    band: (154, 330),
    width: 510.0,
    delay: 2,
};

/// Frames of the αMAIL panel wiping in.
pub const MAIL_FRAMES: i32 = 64;

const MAIL_SPEC: Spec = Spec {
    frames: MAIL_FRAMES,
    speck: 8,
    first_y: 259,
    band: (20, 460),
    width: 880.0,
    delay: 7,
};

/// One speck of [`Lights`].
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Light {
    pub from: (i32, i32),
    pub to: (i32, i32),
    /// Frame of its way, 0 before it starts.
    pub frame: i32,
    pub delay: i32,
    /// Size in percent.
    pub size: i32,
    /// Where it is drawn and its opacity, once it moved.
    pub at: Option<(i32, i32, u8)>,
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

impl Lights {
    pub fn new(middle: i32, seed: u32) -> Self {
        Self::make(middle, seed, false)
    }

    /// The αMAIL panel's specks.
    pub fn mail(seed: u32) -> Self {
        Self::make(640, seed, true)
    }

    fn spec(&self) -> &'static Spec {
        if self.mail {
            &MAIL_SPEC
        } else {
            &HOLO_SPEC
        }
    }

    fn make(middle: i32, seed: u32, mail: bool) -> Self {
        let mut rng = seed;
        let first_y = if mail {
            MAIL_SPEC.first_y
        } else {
            HOLO_SPEC.first_y
        };
        let specks = (0..LIGHT_COUNT)
            .map(|_| {
                let to = (middle - 16 + rand(&mut rng, 16), first_y);
                let delay = rand(&mut rng, 7);
                let size = rand(&mut rng, 25) + 25;
                Light {
                    from: (middle, 720),
                    to,
                    frame: 0,
                    delay,
                    size,
                    at: None,
                }
            })
            .collect();
        Lights {
            specks,
            frame: 0,
            middle,
            rng,
            carry: 0,
            mail,
        }
    }

    /// Whether it is over.
    pub fn done(&self) -> bool {
        self.frame >= self.spec().frames
    }

    /// One frame: `f` from 1 to the number of frames.
    fn step(&mut self) {
        self.frame += 1;
        let spec = self.spec();
        let (f, n, len) = (self.frame, spec.frames, spec.speck);
        let middle = self.middle;
        for p in &mut self.specks {
            if p.frame == 0 && n - f < len {
                continue;
            }
            if p.delay > 0 {
                p.delay -= 1;
                continue;
            }
            let x = (p.to.0 - p.from.0) * p.frame / len + p.from.0;
            let y = (p.to.1 - p.from.1) * p.frame / len + p.from.1;
            let alpha = 255.0 - p.frame as f32 * 255.0 / len as f32;
            p.at = Some((x, y, alpha as u8));
            p.frame += 1;
            if p.frame > len {
                let end_y = rand(&mut self.rng, spec.band.1) + spec.band.0;
                let w = (f as f32 / n as f32 * spec.width) as i32;
                let dx = rand(&mut self.rng, w);
                p.from = (middle, 720);
                p.to = (middle - w / 2 + dx, end_y);
                p.delay = rand(&mut self.rng, spec.delay);
                p.frame = 0;
            }
        }
    }

    /// Runs for `ms`; true when it moved.
    pub fn advance(&mut self, ms: u64) -> bool {
        self.carry += ms;
        let mut moved = false;
        while self.carry >= TICK_MS && !self.done() {
            self.carry -= TICK_MS;
            self.step();
            moved = true;
        }
        moved
    }
}

impl Fade {
    pub fn new(from: u8, to: u8, frames: i32, panel: bool) -> Self {
        Fade {
            from,
            to,
            frames: frames.max(1),
            panel,
            rule: None,
            ms: 0,
        }
    }

    /// Her picture wiping in through `rule` over `frames` frames.
    pub fn wipe(rule: u32, frames: i32) -> Self {
        Fade {
            rule: Some(rule),
            ..Fade::new(0, 255, frames, false)
        }
    }

    /// How far the wipe is, 0 to 1.
    pub fn progress(&self) -> f32 {
        self.frame() as f32 / self.frames.max(1) as f32
    }

    /// Frames gone by, up to the length.
    fn frame(&self) -> i32 {
        ((self.ms / TICK_MS) as i32).min(self.frames)
    }

    /// Opacity now, out of 255.
    pub fn alpha(&self) -> u8 {
        let (a, b) = (self.from as i32, self.to as i32);
        ((b - a) * self.frame() / self.frames.max(1) + a) as u8
    }

    pub fn done(&self) -> bool {
        self.frame() >= self.frames
    }

    /// How long it lasts.
    pub fn length_ms(&self) -> u64 {
        self.frames.max(0) as u64 * TICK_MS
    }
}

/// Panel picture: 510 x 330 halves of `sld_mini.akb`.
pub const PANEL_SIZE: (u32, u32) = (510, 330);
/// The window of her picture: 442 x 300 at 100 / size times that, taken
/// from the canvas around x = 1024 from y = 662.
const WINDOW: (f32, f32) = (442.0, 300.0);
const WINDOW_TOP: f32 = 662.0;

impl Holo {
    /// Top centre of her picture by place.
    fn anchor(&self) -> (i32, i32) {
        match self.place {
            1 => (223, 170),
            2 => (1057, 170),
            _ => (640, 170),
        }
    }

    /// Top left of the panel by place.
    pub fn panel_at(&self) -> (i32, i32) {
        match self.place {
            1 => (-32, 154),
            2 => (802, 154),
            _ => (385, 154),
        }
    }

    /// Size as a factor.
    pub fn factor(&self) -> f32 {
        if self.scale > 0 {
            self.scale as f32 / 100.0
        } else {
            0.5
        }
    }

    /// The canvas window (left, top, width, height) of her picture.
    pub fn window(&self) -> (u32, u32, u32, u32) {
        let f = self.factor();
        let (w, h) = ((WINDOW.0 / f) as u32, (WINDOW.1 / f) as u32);
        (1024 - w / 2, WINDOW_TOP as u32, w, h)
    }

    /// Where the window's top left lands on screen.
    pub fn window_at(&self) -> (f32, f32) {
        let (x, y) = self.anchor();
        let w = self.window().2 as f32 * self.factor();
        (
            x as f32 - w / 2.0 + self.offset.0 as f32,
            y as f32 + self.offset.1 as f32,
        )
    }

    /// Runs the fade for `ms`; true when the picture changed. A fade out
    /// takes her and the panel away at its end.
    pub fn advance(&mut self, ms: u64) -> bool {
        let mut lit = false;
        if let Some(l) = &mut self.lights {
            lit = l.advance(ms);
            if l.done() {
                self.lights = None;
                lit = true;
            }
        }
        let Some(f) = &mut self.fade else {
            return lit;
        };
        let before = f.alpha();
        f.ms += ms;
        let changed = f.alpha() != before || f.rule.is_some() || lit;
        if f.done() {
            let gone = f.to == 0;
            self.fade = None;
            if gone {
                self.remove();
            }
            return true;
        }
        changed
    }

    /// Ends the fade under way at once.
    pub fn finish(&mut self) {
        self.lights = None;
        if let Some(f) = self.fade {
            self.advance(f.length_ms());
        }
    }

    /// Takes her and the panel away.
    pub fn remove(&mut self) {
        self.shown = false;
        self.body.clear();
        self.faces.clear();
        self.fade = None;
        self.lights = None;
    }

    /// Opacity of her picture and of the panel under the fade, out of
    /// 255 (a wipe leaves both whole).
    pub fn fade_alpha(&self) -> (u8, u8) {
        match &self.fade {
            Some(f) if f.rule.is_none() => (f.alpha(), if f.panel { f.alpha() } else { 255 }),
            _ => (255, 255),
        }
    }

    /// The wipe under way: rule and progress.
    pub fn wipe(&self) -> Option<(u32, f32)> {
        let f = self.fade.as_ref()?;
        Some((f.rule?, f.progress()))
    }

    /// Middle of her picture (x) by place.
    pub fn middle(&self) -> i32 {
        self.anchor().0
    }

    /// Opacity of her picture.
    pub fn alpha(&self) -> u8 {
        if self.panel == 0 {
            230
        } else {
            255
        }
    }
}

/// The window of `picture` with its alpha scaled by `alpha`.
pub fn cut(picture: &Image, window: (u32, u32, u32, u32), alpha: u8) -> Arc<Image> {
    let (l, t, w, h) = window;
    let mut rgba = vec![0; (w * h * 4) as usize];
    for y in 0..h {
        let sy = t + y;
        if sy >= picture.height {
            break;
        }
        for x in 0..w {
            let sx = l + x;
            if sx >= picture.width {
                break;
            }
            let s = ((sy * picture.width + sx) * 4) as usize;
            let d = ((y * w + x) * 4) as usize;
            rgba[d..d + 3].copy_from_slice(&picture.rgba[s..s + 3]);
            rgba[d + 3] = (picture.rgba[s + 3] as u32 * alpha as u32 / 255) as u8;
        }
    }
    Arc::new(Image {
        width: w,
        height: h,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_out_takes_her_away_at_the_end() {
        let mut h = Holo {
            body: "ch01_01_00.akb".into(),
            shown: true,
            fade: Some(Fade::new(255, 0, 12, true)),
            ..Holo::default()
        };
        assert_eq!(h.fade_alpha(), (255, 255));
        assert!(h.advance(6 * TICK_MS));
        assert_eq!(h.fade_alpha(), (128, 128));
        assert!(h.shown);
        assert!(h.advance(6 * TICK_MS));
        assert!(!h.shown && h.body.is_empty() && h.fade.is_none());
    }

    #[test]
    fn lights_rise_into_the_panel_and_end() {
        let mut l = Lights::new(640, 7);
        assert!(l
            .specks
            .iter()
            .all(|p| p.to.1 == 549 && (624..640).contains(&p.to.0)));
        assert!(l
            .specks
            .iter()
            .all(|p| (25..50).contains(&p.size) && p.delay < 7));
        l.advance(TICK_MS);
        // Specks without a wait are at their start, whole.
        assert!(l
            .specks
            .iter()
            .filter_map(|p| p.at)
            .all(|(x, y, a)| (x, y, a) == (640, 720, 255)));
        l.advance(4 * TICK_MS);
        // Those that went their way start again, ending in the panel.
        let later: Vec<_> = l.specks.iter().filter(|p| p.to.1 != 549).collect();
        assert!(!later.is_empty());
        assert!(later
            .iter()
            .all(|p| (154..484).contains(&p.to.1) && p.delay < 2));
        assert!(!l.advance(100 * TICK_MS) || l.done());
        assert!(l.done());
    }

    #[test]
    fn fade_in_leaves_the_panel() {
        let mut h = Holo {
            shown: true,
            fade: Some(Fade::new(0, 255, FADE_IN_FRAMES, false)),
            ..Holo::default()
        };
        assert_eq!(h.fade_alpha(), (0, 255));
        h.advance(3 * TICK_MS);
        assert_eq!(h.fade_alpha(), (63, 255));
        h.advance(20 * TICK_MS);
        assert!(h.shown && h.fade.is_none());
    }
}
