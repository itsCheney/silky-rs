//! Standing characters and the face window of なないろリンカネーション.
//!
//! Standing pictures are 1600 by 1600; the figure's left edge is at
//! `(1280 - 16 * size) / 2`, a side slot 345 pixels further out, and above
//! 50 % the figure is lifted by `(2 * size - 100) * K / 100` with K by
//! character. The face window is a 532 by 544 crop of the picture, at an
//! offset by character, drawn at half size.

use crate::assets::Assets;
use crate::games::apeiria::holo::cut;
use crate::render::Canvas;
use crate::scene::Scene;
use crate::text::TextRenderer;
use crate::Result;

/// The two letters after the `t` of a standing picture's name.
fn id(body: &str) -> Option<String> {
    let b = body.to_ascii_lowercase();
    (b.starts_with('t') && b.len() > 3).then(|| b[1..3].to_string())
}

/// Lift of a character at full size.
fn lift_k(body: &str) -> i32 {
    match id(body).as_deref() {
        Some("iy") => 400,
        Some("ai") => 280,
        Some("ki" | "fu" | "az" | "yu") => 140,
        Some("ko" | "ao") => 200,
        _ => 0,
    }
}

/// Shift from where the shared renderer puts a figure (centred on a slot
/// 300 pixels out, top at `300 - 7 * size`) to where this game does.
pub fn offset(slot: i32, size: i32, body: &str) -> (i32, i32) {
    let shared = match slot {
        1 => -300,
        2 => 300,
        _ => 0,
    };
    let own = match slot {
        1 => -345,
        2 => 345,
        _ => 0,
    };
    let lift = if size > 50 {
        (2 * size - 100) * lift_k(body) / 100
    } else {
        0
    };
    (own - shared, -lift - (300 - 7 * size))
}

/// Left and top of the face window on the picture.
fn crop(body: &str) -> (u32, u32) {
    match id(body).as_deref() {
        Some("iy") => (0x214, 0x1c8),
        Some("ai") => (0x260, 0x13c),
        Some("ao") => (0x24c, 0xb8),
        Some("ki") => (0x220, 0x3c),
        Some("ko") => (0x22e, 0xd0),
        Some("az") => (0x204, 0x3e),
        Some("fu") => (0x208, 0x44),
        Some("yu") => (0x236, 0x3e),
        _ => (0, 0),
    }
}

/// Window of the face window on screen and its layer's top.
const PORTRAIT_AT: (f32, f32) = (0.0, 448.0);
/// The name plate on the window sheet.
const PLATE: (u32, u32, u32, u32) = (694, 492, 177, 69);
const PLATE_AT: (i32, i32) = (246, super::window::TOP + 22);

/// Draws the face window and the speaker's name plate.
pub fn draw(
    canvas: &mut Canvas,
    assets: &mut Assets,
    scene: &Scene,
    text: Option<&TextRenderer>,
    rgb: [u8; 3],
    alpha: u8,
) -> Result<()> {
    if let (true, Some((body, face))) = (scene.portrait_visible, &scene.portrait) {
        let (l, t) = crop(body);
        let window = (l, t, 532, 544);
        let img = assets.image(body)?;
        canvas.draw_scaled(&cut(&img, window, 255), PORTRAIT_AT.0, PORTRAIT_AT.1, 0.5);
        if let Some(face) = face {
            let img = assets.image(face)?;
            canvas.draw_scaled(&cut(&img, window, 255), PORTRAIT_AT.0, PORTRAIT_AT.1, 0.5);
        }
    }
    if let Some(name) = &scene.speaker_name {
        let sheet = assets.image(super::window::SHEET)?;
        let plate = cut(&sheet, PLATE, 255);
        let mut tinted = (*plate).clone();
        for px in tinted.rgba.chunks_exact_mut(4) {
            px[..3].copy_from_slice(&rgb);
        }
        let (w, h) = (PLATE.2, PLATE.3);
        let (x, y) = PLATE_AT;
        canvas.draw_part(
            &std::sync::Arc::new(tinted),
            (0, 0, w, h),
            x,
            y,
            false,
            alpha,
        );
        if let Some(tr) = text {
            let size = 22.0;
            let width = name.chars().count() as f32 * size;
            let tx = x as f32 + (w as f32 - width) / 2.0;
            tr.draw_line(canvas, name, tx, (y + 44) as f32, size, [255; 4]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::offset;

    /// The shared renderer's placement plus the shift is the game's.
    #[test]
    fn placement() {
        // Size 50: no lift, top at 0; left side slot 345 out.
        assert_eq!(offset(0, 50, "tiy11.akb"), (0, 50));
        assert_eq!(offset(1, 50, "tki11.akb"), (-45, 50));
        // Size 100 is lifted by K: iy 400, so the top is at -400.
        let (_, dy) = offset(2, 100, "tiy11.akb");
        assert_eq!(300 - 700 + dy, -400);
        let (_, dy) = offset(0, 75, "tai11.akb");
        assert_eq!(300 - 525 + dy, -140);
    }
}
