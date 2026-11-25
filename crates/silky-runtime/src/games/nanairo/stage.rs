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

/// Sprite that event pictures and scene stills are loaded into over the
/// background; it only shows while it is copied onto the screen.
const OVERLAY_SPRITE: i32 = 6;

/// `Sprite.3(flag, source, y, x, target, height, width, target y, target
/// x)`: the screen is the target sprite 0. A whole picture copied onto it
/// from the background replaces what an earlier copy of the overlay
/// sprite showed; the overlay's picture is kept in its sprite but is gone
/// from the screen.
pub fn copied(scene: &mut Scene, a: &[i32]) {
    if let [_, source, 0, 0, 0, 720, 1280, 0, 0] = *a {
        if source == super::BACKGROUND_SPRITE {
            scene.layers.remove(&OVERLAY_SPRITE);
        }
    }
}

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

/// Where the shared renderer puts the middle of a slot's figure, from the
/// middle of the screen.
fn shared_x(slot: i32) -> i32 {
    match slot {
        1 => -300,
        2 => 300,
        _ => 0,
    }
}

/// Where this game puts the left edge of a slot's figure, from the same
/// place.
fn own_x(slot: i32) -> i32 {
    match slot {
        1 => -345,
        2 => 345,
        _ => 0,
    }
}

/// Shift from where the shared renderer puts a figure (centred on a slot
/// 300 pixels out, top at `300 - 7 * size`) to where this game does, for
/// the figure of `slot` standing at the place of `place` (the same slot
/// unless `Game.13.8` moved it).
pub fn shifted(slot: i32, place: i32, size: i32, body: &str) -> (i32, i32) {
    let lift = if size > 50 {
        (2 * size - 100) * lift_k(body) / 100
    } else {
        0
    };
    (own_x(place) - shared_x(slot), -lift - (300 - 7 * size))
}

/// Whether two pictures are of the same character.
pub fn same_character(a: &str, b: &str) -> bool {
    id(a).is_some() && id(a) == id(b)
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
/// The speaker's name: white, size 22, from the text's left edge, its
/// baseline 80 below the window top, with no plate behind it (measured
/// under Wine).
const NAME_AT: (f32, f32) = (super::window::TEXT_LEFT, (super::window::TOP + 80) as f32);
const NAME_SIZE: f32 = 22.0;

/// Draws the face window and the speaker's name.
pub fn draw(
    canvas: &mut Canvas,
    assets: &mut Assets,
    scene: &Scene,
    text: Option<&TextRenderer>,
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
    if let (Some(name), Some(tr)) = (&scene.speaker_name, text) {
        tr.draw_line(canvas, name, NAME_AT.0, NAME_AT.1, NAME_SIZE, [255; 4]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::shifted;

    fn offset(slot: i32, size: i32, body: &str) -> (i32, i32) {
        shifted(slot, slot, size, body)
    }

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

    /// A figure of the centre slot moved to the left place is 345 pixels
    /// out, one of the left slot moved to the centre is 345 pixels in.
    #[test]
    fn places() {
        assert_eq!(shifted(0, 1, 50, "tiy11.akb"), (-345, 50));
        assert_eq!(shifted(1, 0, 50, "tiy11.akb"), (300, 50));
        assert_eq!(shifted(2, 2, 50, "tiy11.akb"), offset(2, 50, "tiy11.akb"));
    }

    /// A still loaded into the overlay sprite shows once it is copied onto
    /// the screen, and goes when the next background is copied there.
    #[test]
    fn overlay_picture_goes_with_the_next_background() {
        let mut scene = crate::scene::Scene::default();
        scene.layers.insert(2, "HG040003_h01.akb".into());
        scene.layers.insert(6, "HG040002_h01.akb".into());
        // A part of the picture does not replace the screen.
        super::copied(&mut scene, &[1, 2, 0, 0, 0, 100, 100, 0, 0]);
        assert!(scene.layers.contains_key(&6));
        // The overlay itself copied to the screen stays.
        super::copied(&mut scene, &[0, 6, 0, 0, 0, 720, 1280, 0, 0]);
        assert!(scene.layers.contains_key(&6));
        super::copied(&mut scene, &[1, 2, 0, 0, 0, 720, 1280, 0, 0]);
        assert!(!scene.layers.contains_key(&6));
        assert!(scene.layers.contains_key(&2));
    }

    #[test]
    fn characters() {
        assert!(super::same_character("tiy11.akb", "tiy12_h03.akb"));
        assert!(!super::same_character("tiy11.akb", "tai11.akb"));
    }
}
