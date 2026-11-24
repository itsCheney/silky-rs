//! The title menu of 先輩が私の妄想にドージンする (`Game.110`).
//!
//! `title_p.akb` holds the button cells, 42 pixels high with four state
//! rows each (normal, hover, pressed, grey), in a row per page. The main
//! page has six buttons at the places of `title.area`; the extras page has
//! five at the places of `title2.area`, drawn over the same column. A
//! button's click area is the whole rectangle of its area, the cell may be
//! wider. The call returns the picked item plus one for the extras (the
//! script takes 257 and up from them).

use crate::ui::title;

/// Height of a cell and distance between its state rows.
pub const ROW: u32 = 42;

/// Extras entry that opens the doujinshi (the first button of the page).
pub const BODY_TOUCH: usize = title::BODY_TOUCH;

/// One button: area id, entry, cell on the sheet (left, top) and its
/// width.
pub type Spec = (i32, usize, (u32, u32), u32);

/// Main page: new game, continue from the last save, load, settings,
/// extras, quit.
pub const MAIN: [Spec; 6] = [
    (0, title::NEW_GAME, (0, 0), 166),
    (1, title::CONTINUE, (309, 0), 213),
    (2, title::LOAD, (166, 0), 143),
    (3, title::SETTINGS, (522, 0), 144),
    (4, title::EXTRAS, (666, 0), 120),
    (5, title::QUIT, (782, 0), 237),
];

/// Extras page: the doujinshi, CG mode, scene mode, music mode, back.
pub const EXTRAS: [Spec; 5] = [
    (256, BODY_TOUCH, (519, 336), 314),
    (257, title::CG_MODE, (0, 168), 213),
    (258, title::SCENE_MODE, (213, 168), 213),
    (259, title::MUSIC_MODE, (426, 168), 213),
    (260, title::BACK, (639, 168), 120),
];

/// Entries of the extras page and the byte variables that unlock them
/// (the doujinshi is opened by the script itself on the first title).
pub const UNLOCKS: [(usize, usize); 4] = [
    (title::CG_MODE, 0xbba),
    (title::SCENE_MODE, 0xbbb),
    (title::MUSIC_MODE, 0xbbc),
    (BODY_TOUCH, 0xbbd),
];

/// What the call returns for an entry that leaves the menu. Continue,
/// load, settings, extras and back are the menu's own; quit asks on its
/// own.
pub fn result(entry: usize) -> Option<i32> {
    Some(match entry {
        title::NEW_GAME => 1,
        BODY_TOUCH => 257,
        title::CG_MODE => 258,
        title::SCENE_MODE => 259,
        title::MUSIC_MODE => 260,
        _ => return None,
    })
}
