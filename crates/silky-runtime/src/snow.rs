//! Falling snow of ふゆから、くるる。 (`Game.115`: 0 starts it, 1 stops
//! it), read from the executable.
//!
//! The engine keeps 100 flakes of `SnowBall.akb` (64 x 64, drawn from its
//! centre). At most one flake starts per frame, and only once a random
//! wait of up to 200 ms since the last one has passed: the first free flake
//! starts 64 pixels above the screen at a random x from 120 to 1119. Each
//! frame it moves by its speed and leaves a trail of nine copies at its
//! last positions, each fainter (opacity 128 for the flake, 10 less per
//! copy) and smaller (0.01 less scale per copy). It stops once it leaves
//! the screen (x <= -100, x >= 1380 or y >= 820). Four flakes (numbers 4,
//! 34, 64, 94) are larger and faster.

use std::sync::Arc;

use silky_format::akb::Image;

use crate::assets::Assets;
use crate::canvas::Canvas;

const FLAKES: usize = 100;
const TRAIL: usize = 10;
/// Frames of the engine, 60 a second.
const FRAME_MS: f32 = 1000.0 / 60.0;

#[derive(Debug, Default, Clone)]
struct Flake {
    active: bool,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    scale: f32,
    /// Last positions, newest first.
    trail: Vec<(f32, f32)>,
}

/// The snow, while it falls.
#[derive(Debug)]
pub struct Snow {
    flakes: Vec<Flake>,
    clock: f32,
    next: f32,
    left: f32,
    seed: u32,
    /// The flake picture at the opacity of each trail copy.
    images: Option<Vec<Arc<Image>>>,
}

impl Default for Snow {
    fn default() -> Self {
        Self {
            flakes: vec![Flake::default(); FLAKES],
            clock: 0.0,
            next: 0.0,
            left: 0.0,
            seed: 0x9e37_79b9,
            images: None,
        }
    }
}

impl Snow {
    /// Random number from 0 to `n - 1`.
    fn random(&mut self, n: u32) -> u32 {
        // xorshift32
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        x % n
    }

    fn start_flake(&mut self) {
        let Some(i) = self.flakes.iter().position(|f| !f.active) else {
            return;
        };
        let x = (self.random(1000) + 120) as f32;
        let dx = self.random(100) as f32 / 100.0 - 0.5;
        let (dy, scale) = if i % 30 == 4 {
            (
                self.random(220) as f32 / 100.0 + 1.3,
                self.random(30) as f32 / 100.0 + 0.5,
            )
        } else {
            (
                self.random(100) as f32 / 100.0 + 0.3,
                self.random(20) as f32 / 100.0 + 0.2,
            )
        };
        self.flakes[i] = Flake {
            active: true,
            x,
            y: -64.0,
            dx,
            dy,
            scale,
            trail: Vec::new(),
        };
        self.next = self.clock + self.random(200) as f32;
    }

    /// Runs the snow for `ms`; true when it moved.
    pub fn progress(&mut self, ms: f32) -> bool {
        self.left += ms;
        let mut moved = false;
        while self.left >= FRAME_MS {
            self.left -= FRAME_MS;
            self.clock += FRAME_MS;
            if self.clock >= self.next {
                self.start_flake();
            }
            for f in self.flakes.iter_mut().filter(|f| f.active) {
                f.trail.insert(0, (f.x, f.y));
                f.trail.truncate(TRAIL + 1);
                f.x += f.dx;
                f.y += f.dy;
                if f.x <= -100.0 || f.x >= 1380.0 || f.y >= 820.0 {
                    f.active = false;
                }
                moved = true;
            }
        }
        moved
    }

    /// Draws the flakes and their trails.
    pub fn draw(&mut self, canvas: &mut Canvas, assets: &mut Assets) {
        if self.images.is_none() {
            let Ok(img) = assets.image("SnowBall.akb") else {
                return;
            };
            self.images = Some(
                (0..TRAIL)
                    .map(|i| {
                        let a = (128 - 10 * i) as u32;
                        let mut copy = (*img).clone();
                        for px in copy.rgba.chunks_exact_mut(4) {
                            px[3] = (px[3] as u32 * a / 255) as u8;
                        }
                        Arc::new(copy)
                    })
                    .collect(),
            );
        }
        let Some(images) = &self.images else {
            return;
        };
        for f in self.flakes.iter().filter(|f| f.active) {
            let positions: Vec<_> = std::iter::once((f.x, f.y))
                .chain(f.trail.iter().copied())
                .take(TRAIL)
                .collect();
            // The faintest copy first, the flake on top.
            for (i, &(x, y)) in positions.iter().enumerate().rev() {
                let img = &images[i];
                let s = f.scale - 0.01 * i as f32;
                let (w, h) = (img.width as f32 * s, img.height as f32 * s);
                canvas.draw_scaled(img, x - w / 2.0, y - h / 2.0, s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flakes_fall_and_leave() {
        let mut snow = Snow::default();
        assert!(snow.progress(1000.0));
        let active: Vec<_> = snow.flakes.iter().filter(|f| f.active).cloned().collect();
        assert!(!active.is_empty());
        assert!(active
            .iter()
            .all(|f| f.y > -64.0 && f.trail.len() <= TRAIL + 1));
        // Flakes leave the screen and free their place for new ones.
        snow.progress(60_000.0);
        assert!(snow
            .flakes
            .iter()
            .filter(|f| f.active)
            .all(|f| f.y < 820.0 && f.x > -100.0 && f.x < 1380.0));
    }
}
