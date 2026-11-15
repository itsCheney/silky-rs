//! Where standing characters (`Game.13`) stand in 景の海のアペイリア,
//! read from the executable: the canvas (2048 pixels wide) is scaled
//! about its top centre, which goes to the slot's x; the top edge depends
//! on the character and the size.

/// Shift of the canvas centre from the screen centre by slot (0x44c690).
fn slot_x(slot: i32) -> f32 {
    match slot {
        1 => -400.0,
        2 | 3 => 400.0,
        _ => 0.0,
    }
}

/// Top edge on screen at size `s`: `(1 - s) * a - s * b` with (a, b) by
/// character (0x44c710); -120 for those without an entry.
fn top(body: &str, s: f32) -> f32 {
    let b = body.as_bytes();
    let id = match (b.first(), b.get(1), b.get(2), b.get(3)) {
        (Some(b'c' | b'C'), Some(b'h' | b'H'), Some(&t), Some(&u)) => {
            Some((t.wrapping_sub(b'0'), u.wrapping_sub(b'0')))
        }
        _ => None,
    };
    let (a, k) = match id {
        Some((0, 1)) => (270.0, 550.0),
        Some((0, 2)) => (170.0, 400.0),
        Some((0, 3)) => (250.0, 430.0),
        Some((0, 4)) => (160.0, 380.0),
        Some((0, 5)) | Some((0, 6)) => (120.0, 200.0),
        Some((0, 7)) => (100.0, 100.0),
        Some((0, 9)) => (130.0, 266.0),
        Some((1, 0)) => (160.0, 350.0),
        _ => return -120.0,
    };
    (1.0 - s) * a - s * k
}

/// Shift of the canvas centre from the screen centre and the top edge.
pub fn place(slot: i32, body: &str, s: f32) -> (f32, f32) {
    (slot_x(slot), top(body, s))
}

/// Message window portrait (`Game.10`): a 1024 x 1024 window of the
/// standing picture (left, top by character, 0x4625b0) at 0.45 of its
/// size with its top left at (-144, 440) (0x461ca0).
pub const PORTRAIT: (f32, f32, f32) = (-144.0, 440.0, 0.45);

/// The window of the portrait on the canvas.
pub fn portrait_window(body: &str) -> (u32, u32, u32, u32) {
    let b = body.as_bytes();
    let (l, t) = match (b.get(2), b.get(3)) {
        (Some(b'0'), Some(b'1')) => (542, 632),
        (Some(b'0'), Some(b'2')) => (482, 381),
        (Some(b'0'), Some(b'3')) => (542, 452),
        (Some(b'0'), Some(b'4')) => (542, 392),
        (Some(b'0'), Some(b'5')) => (512, 102),
        (Some(b'0'), Some(b'6')) => (512, 71),
        (Some(b'0'), Some(b'7')) => (512, 22),
        (Some(b'0'), Some(b'9')) => (512, 241),
        (Some(b'1'), Some(b'0')) => (512, 392),
        _ => (0, 0),
    };
    (l, t, 1024, 1024)
}
