//! The explanation slides of 景の海のアペイリア (`Game.991`): lecture
//! pictures (`tNN_NN_NN.akb` over `tbg.akb`, `wNN_NN_NN.akb` over
//! `wbg.akb`) that fade in one over the other and scroll.
//!
//! The original keeps a background picture and a tall sprite layer
//! (1280 x 3880) that the script moves up and down. A picture is copied
//! into the layer's top left corner, replacing what was there (its
//! transparent pixels too), so the first lines of a picture are what shows
//! when the layer sits at (0, 0); a moving layer shows a window of them.
//! A picture shorter than the one before leaves the old one's lower
//! lines. A second picture over the layer (subs 5 and 9) is drawn on the
//! screen and does not move with it.

use serde::{Deserialize, Serialize};

use super::panels::{Tween, TICK_MS};

/// Background pictures: kind 0 and kind 1.
pub const BACKGROUNDS: [&str; 2] = ["tbg.akb", "wbg.akb"];
/// Lines of the screen.
pub const VIEW_HEIGHT: i32 = 720;
/// Frames of the layer's fade-out (sub 7).
pub const OUT_FRAMES: i32 = 12;

/// The slides on screen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slides {
    /// 0 for `tbg.akb`, 1 for `wbg.akb`.
    pub kind: i32,
    /// Shown (from sub 2, 3 or 4 until the fade-out ends).
    pub shown: bool,
    /// Pictures copied into the layer with their heights, the oldest
    /// first; only those taller than every later one are kept.
    pub pictures: Vec<(String, u32)>,
    /// Where the layer is, when no scroll is running.
    pub pos: (i32, i32),
    /// A picture fading in (subs 3 and 4).
    pub fade_in: Option<FadeIn>,
    /// The layer moving (subs 10 and 12).
    pub scroll: Option<Scroll>,
    /// The picture over the layer, with its opacity (subs 5 and 9).
    pub over: Option<(String, Tween)>,
    /// The layer's opacity while it fades out (sub 7).
    pub out: Option<Tween>,
}

/// A picture coming into the layer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FadeIn {
    pub picture: String,
    pub height: u32,
    pub frames: i32,
    pub ms: u64,
    /// Fades on the screen's top left corner (sub 3) instead of inside
    /// the layer (sub 4).
    pub on_screen: bool,
}

/// The layer moving from one place to another, in a straight line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scroll {
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub frames: i32,
    pub ms: u64,
    /// The script waits for it (sub 10) or goes on (sub 12).
    pub waited: bool,
}

/// What a script that drops the slides leaves in the scene (sub 8): the
/// background and the window of the layer that showed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlideShot {
    pub kind: i32,
    pub pictures: Vec<(String, u32)>,
    pub pos: (i32, i32),
}

impl FadeIn {
    /// Opacity now: 0 to 255 over the frames.
    pub fn alpha(&self) -> u8 {
        let n = self.frames.max(1) as u64;
        let f = (self.ms / TICK_MS).min(n);
        (255 * f / n) as u8
    }

    fn done(&self) -> bool {
        self.ms >= self.frames.max(1) as u64 * TICK_MS
    }
}

impl Scroll {
    /// Where the layer is now.
    pub fn at(&self) -> (i32, i32) {
        let n = self.frames.max(1) as i64;
        let f = (self.ms / TICK_MS) as i64;
        if f >= n {
            return self.to;
        }
        let v = |a: i32, b: i32| ((b - a) as i64 * f / n) as i32 + a;
        (v(self.from.0, self.to.0), v(self.from.1, self.to.1))
    }

    fn done(&self) -> bool {
        self.ms >= self.frames.max(1) as u64 * TICK_MS
    }
}

impl Slides {
    /// Sub 0: the layer and its background, nothing drawn yet.
    pub fn new(kind: i32) -> Self {
        Self {
            kind: kind.clamp(0, 1),
            ..Self::default()
        }
    }

    /// The background picture.
    pub fn background(&self) -> &'static str {
        BACKGROUNDS[self.kind.clamp(0, 1) as usize]
    }

    /// Where the layer is now.
    pub fn position(&self) -> (i32, i32) {
        self.scroll.as_ref().map_or(self.pos, Scroll::at)
    }

    /// A picture copied into the layer: what it covers is gone.
    fn put(&mut self, file: String, height: u32) {
        self.pictures.retain(|(_, h)| *h > height);
        self.pictures.push((file, height));
    }

    /// Sub 2: a picture copied into the layer at once; everything shows.
    pub fn show(&mut self, file: String, height: u32) {
        self.put(file, height);
        self.shown = true;
    }

    /// Subs 3 and 4: a picture fades in over `frames` frames. Returns how
    /// long the script waits (ms).
    pub fn fade(&mut self, file: String, height: u32, frames: i32, on_screen: bool) -> u64 {
        let frames = frames.max(1);
        self.shown = true;
        self.fade_in = Some(FadeIn {
            picture: file,
            height,
            frames,
            ms: 0,
            on_screen,
        });
        frames as u64 * TICK_MS
    }

    /// Subs 10 and 12: the layer moves from where it is to `to` over
    /// `frames` frames. Returns how long the script waits (ms).
    pub fn scroll_to(&mut self, to: (i32, i32), frames: i32, waited: bool) -> u64 {
        let from = self.position();
        let frames = frames.max(1);
        self.scroll = Some(Scroll {
            from,
            to,
            frames,
            ms: 0,
            waited,
        });
        if waited {
            frames as u64 * TICK_MS
        } else {
            0
        }
    }

    /// Sub 11: the layer is put there at once.
    pub fn set_position(&mut self, at: (i32, i32)) {
        self.scroll = None;
        self.pos = at;
    }

    /// Sub 5: a picture over the layer that fades out and in again for
    /// ever: shown, after 42 frames out over 8, after 128 more in over 8.
    pub fn blink(&mut self, file: String) {
        let keys = vec![(0, 1, 255, 255), (42, 8, 255, 0), (128, 8, 0, 255)];
        self.over = Some((file, Tween::looping(keys)));
    }

    /// Sub 9: a picture over the layer that fades in over `frames` frames
    /// again and again.
    pub fn repeat(&mut self, file: String, frames: i32) {
        let keys = vec![(0, 1, 255, 255), (0, frames, 0, 255)];
        self.over = Some((file, Tween::looping(keys)));
    }

    /// Subs 6, 7 and 8: the running animation stops where it is.
    pub fn stop(&mut self) {
        self.pos = self.position();
        self.scroll = None;
        if let Some((_, t)) = &mut self.over {
            t.stop();
        }
    }

    /// Sub 7: the layer fades out over 12 frames and goes. Returns how
    /// long the script waits (ms).
    pub fn fade_out(&mut self) -> u64 {
        self.stop();
        let t = Tween::once(OUT_FRAMES, 255, 0);
        let ms = t.length_ms();
        self.out = Some(t);
        ms
    }

    /// Sub 8: what shows of the layer, to be drawn into the scene.
    pub fn shot(&self) -> SlideShot {
        SlideShot {
            kind: self.kind,
            pictures: self.pictures.clone(),
            pos: self.position(),
        }
    }

    /// Steps the animations; whether the screen changed.
    pub fn advance(&mut self, ms: u64) -> bool {
        let mut moved = false;
        if let Some(f) = &mut self.fade_in {
            f.ms += ms;
            moved = true;
        }
        if let Some(s) = &mut self.scroll {
            s.ms += ms;
            moved = true;
        }
        if let Some(t) = &mut self.out {
            moved |= t.advance(ms);
        }
        if let Some((_, t)) = &mut self.over {
            moved |= t.advance(ms);
        }
        self.settle();
        moved
    }

    /// Ends what the script waits for.
    pub fn finish(&mut self) {
        if let Some(f) = &mut self.fade_in {
            f.ms = f.frames.max(1) as u64 * TICK_MS;
        }
        if let Some(s) = self.scroll.as_mut().filter(|s| s.waited) {
            s.ms = s.frames.max(1) as u64 * TICK_MS;
        }
        if let Some(t) = &mut self.out {
            t.ms = t.length_ms();
        }
        self.settle();
    }

    /// What came is part of the layer; where it went is where it is.
    fn settle(&mut self) {
        if self.fade_in.as_ref().is_some_and(FadeIn::done) {
            if let Some(f) = self.fade_in.take() {
                self.put(f.picture, f.height);
            }
        }
        if self.scroll.as_ref().is_some_and(Scroll::done) {
            if let Some(s) = self.scroll.take() {
                self.pos = s.to;
            }
        }
        if self.out.as_ref().is_some_and(Tween::done) {
            self.out = None;
            self.shown = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_fades_into_the_layer() {
        let mut s = Slides::new(0);
        s.show("a".into(), 720);
        assert_eq!(s.fade("b".into(), 1720, 24, false), 24 * TICK_MS);
        s.advance(12 * TICK_MS);
        assert_eq!(s.fade_in.as_ref().unwrap().alpha(), 127);
        assert_eq!(s.pictures.len(), 1);
        s.advance(12 * TICK_MS);
        assert!(s.fade_in.is_none());
        // The taller picture covers the first; a shorter one leaves it.
        assert_eq!(s.pictures, [("b".to_string(), 1720)]);
        s.show("c".into(), 720);
        assert_eq!(s.pictures.len(), 2);
    }

    #[test]
    fn the_layer_scrolls_in_a_straight_line_and_from_where_it_is() {
        let mut s = Slides::new(1);
        s.show("a".into(), 720);
        // A scroll the script does not wait for.
        assert_eq!(s.scroll_to((0, -500), 150, false), 0);
        s.advance(75 * TICK_MS);
        assert_eq!(s.position(), (0, -250));
        // The next one starts from there; the script waits for it.
        assert_eq!(s.scroll_to((0, -350), 10, true), 10 * TICK_MS);
        s.advance(5 * TICK_MS);
        assert_eq!(s.position(), (0, -300));
        s.finish();
        assert_eq!((s.position(), s.scroll.is_none()), ((0, -350), true));
        // Stopped, it stays; a waited scroll is the only one `finish` ends.
        s.scroll_to((0, 0), 100, false);
        s.advance(50 * TICK_MS);
        s.finish();
        assert!(s.scroll.is_some());
        s.stop();
        assert_eq!(s.pos, (0, -175));
    }

    #[test]
    fn the_layer_fades_out_and_goes_but_not_the_background() {
        let mut s = Slides::new(0);
        s.show("a".into(), 720);
        assert_eq!(s.fade_out(), OUT_FRAMES as u64 * TICK_MS);
        s.advance(6 * TICK_MS);
        assert_eq!(s.out.as_ref().unwrap().value(), 128);
        assert!(s.shown);
        s.finish();
        assert!(!s.shown && s.out.is_none());
        assert_eq!(s.background(), "tbg.akb");
    }
}
