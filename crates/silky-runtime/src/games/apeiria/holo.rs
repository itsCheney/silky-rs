//! Apeiria on a small hologram panel (`Game.1013`), read from the
//! executable: the top or bottom half of `sld_mini.akb` as the panel and,
//! over it, a window of her standing picture at half size. The light
//! particles and fades of her coming and going are not done; she appears
//! and goes at once.

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
}

/// Panel picture: 510 x 330 halves of `sld_mini.akb`.
pub const PANEL_SIZE: (u32, u32) = (510, 330);
/// The window of her picture: 442 x 300 at 100 / size times that, taken
/// from the canvas around x = 1024 from y = 662.
const WINDOW: (f32, f32) = (442.0, 300.0);
const WINDOW_TOP: f32 = 662.0;

impl Holo {
    /// Top centre of her picture by place (0x468410).
    fn anchor(&self) -> (i32, i32) {
        match self.place {
            1 => (223, 170),
            2 => (1057, 170),
            _ => (640, 170),
        }
    }

    /// Top left of the panel by place (0x468480).
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
