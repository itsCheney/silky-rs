//! The pan over a tall picture (`Game.33`), used by `E09.MES` and
//! `KAORI_X.MES`.
//!
//! Read from the executable. The script loads a 1920 x 2160 picture into
//! sprite layer 1 and then calls `Game.33` with, in order: the start
//! position (x, y) of the window in the picture, the end position, the
//! delay of a step in milliseconds and the step in pixels. Each step shows
//! the 1920 x 1080 window at the current position and moves it on by
//! (distance / steps); the call waits for the delay after each step and
//! ends with the window exactly at the end position. The number of steps
//! is the longer distance divided by the step (rounded down). When the
//! screen speed setting is not the first one, the step is four times
//! larger. See `docs/games/doukyuusei.md`.
//!
//! The "skip scenes" setting, which makes the original do nothing, is not
//! in the runtime.

/// The sprite layer the picture is in.
pub const LAYER: i32 = 1;

/// A pan that is running (the script waits for it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pan {
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub steps: i32,
    pub delay_ms: i32,
    /// Step the wait has got to.
    pub frame: i32,
}

impl Pan {
    /// From the call's values in the order the script pushes them: step,
    /// delay, end y, end x, start y, start x. `None` without all six.
    pub fn new(values: &[i32], fast: bool) -> Option<Pan> {
        let [step, delay, end_y, end_x, start_y, start_x] = *values.last_chunk::<6>()?;
        let step = step.max(1) * if fast { 4 } else { 1 };
        let dist = (end_x - start_x).abs().max((end_y - start_y).abs());
        Some(Pan {
            from: (start_x, start_y),
            to: (end_x, end_y),
            steps: dist / step,
            delay_ms: delay.max(0),
            frame: 0,
        })
    }

    /// How long the wait takes.
    pub fn total_ms(&self) -> u32 {
        (self.steps as i64 * self.delay_ms as i64).clamp(0, u32::MAX as i64) as u32
    }

    /// The window's corner at step `k`; the end position from the last
    /// step on.
    pub fn at(&self, k: i32) -> (i32, i32) {
        if k >= self.steps {
            return self.to;
        }
        let k = k.max(0) as i64;
        let s = self.steps.max(1) as i64;
        let f = |a: i32, b: i32| a + ((b - a) as i64 * k / s) as i32;
        (f(self.from.0, self.to.0), f(self.from.1, self.to.1))
    }

    /// The wait has gone `elapsed` of `total`.
    pub fn progress(&mut self, elapsed: u32, total: u32) {
        let f = elapsed as u64 * self.steps as u64 / total.max(1) as u64;
        self.frame = f.min(self.steps as u64) as i32;
    }

    /// The window's corner now.
    pub fn shown(&self) -> (i32, i32) {
        self.at(self.frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The values as the scripts push them: step, delay, end y, end x,
    // start y, start x.
    const E09: [i32; 6] = [4, 15, 0, 0, 1080, 0];
    const KAORI: [i32; 6] = [16, 15, 0, 0, 1080, 0];

    #[test]
    fn step_counts() {
        assert_eq!(Pan::new(&E09, false).unwrap().steps, 270);
        assert_eq!(Pan::new(&KAORI, false).unwrap().steps, 67);
        // Four times the step: a quarter of the steps.
        assert_eq!(Pan::new(&E09, true).unwrap().steps, 67);
        assert_eq!(Pan::new(&E09, false).unwrap().total_ms(), 4050);
        assert!(Pan::new(&E09[1..], false).is_none());
    }

    #[test]
    fn positions() {
        let p = Pan::new(&E09, false).unwrap();
        assert_eq!(p.at(0), (0, 1080));
        assert_eq!(p.at(1), (0, 1076));
        assert_eq!(p.at(135), (0, 540));
        assert_eq!(p.at(270), (0, 0));
        assert_eq!(p.at(400), (0, 0));
        // 67 steps do not divide 1080: the last one still lands on the end.
        let p = Pan::new(&KAORI, false).unwrap();
        assert_eq!(p.at(1), (0, 1080 - 16));
        assert_eq!(p.at(66).1, 1080 - 1080 * 66 / 67);
        assert_eq!(p.at(67), (0, 0));
    }

    #[test]
    fn progress_follows_the_wait() {
        let mut p = Pan::new(&E09, false).unwrap();
        let total = p.total_ms();
        assert_eq!(p.shown(), (0, 1080));
        p.progress(total / 2, total);
        assert_eq!(p.frame, 135);
        assert_eq!(p.shown(), (0, 540));
        p.progress(total, total);
        assert_eq!(p.shown(), (0, 0));
    }

    #[test]
    fn diagonal_and_no_distance() {
        let p = Pan::new(&[10, 20, 100, 200, 0, 0], false).unwrap();
        assert_eq!(p.steps, 20);
        assert_eq!(p.at(10), (100, 50));
        let p = Pan::new(&[4, 15, 5, 5, 5, 5], false).unwrap();
        assert_eq!((p.steps, p.total_ms()), (0, 0));
        assert_eq!(p.shown(), (5, 5));
    }

    #[test]
    fn scene_shows_the_pan_then_the_end() {
        let mut scene = crate::scene::Scene::default();
        assert_eq!(scene.layer_offset(LAYER), (0, 0));
        scene.layer_view.insert(LAYER, (0, 0));
        let mut p = Pan::new(&E09, false).unwrap();
        p.progress(0, p.total_ms());
        scene.pan = Some(p);
        assert_eq!(scene.layer_offset(LAYER), (0, 1080));
        assert_eq!(scene.layer_offset(2), (0, 0));
        scene.pan = None;
        assert_eq!(scene.layer_offset(LAYER), (0, 0));
    }
}
