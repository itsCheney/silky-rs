//! The keyboard on the game's own CG, scene and music screens.
//!
//! The four arrow keys move the mouse pointer to the middle of another
//! button; the hover follows the pointer, so there is no cursor of its own,
//! and Enter clicks the button under the pointer. Enter and keypad Enter
//! click, Escape, Delete and keypad 0 do what a right click does, and
//! nothing else is bound on these screens: Tab, Home, End, the page keys,
//! the space bar and the mouse wheel do nothing, so pages and tabs are
//! reached by moving onto their buttons and pressing Enter. A key held down
//! moves once; the next move needs the key to be let go.
//!
//! Every move works on the id of the button under the pointer (none when
//! it is on no button) with the area files' ids: cells from 0, the page
//! arrows 0x100 and 0x101, the tabs 0xf00 to 0xf02, back 0xf03 and quit
//! 0xf04. With the pointer on no button every arrow goes to cell 0.

pub use crate::games::apeiria::keys::Arrow;

const PREV: i32 = 0x100;
const NEXT: i32 = 0x101;
const OUTSIDE: i32 = 0x200;
const TAB_CG: i32 = 0xf00;
const TAB_SCENE: i32 = 0xf02;
const BACK: i32 = 0xf03;

/// The picture and scene grids: 15 cells in five columns, `exists` says
/// whether a cell is on the page (open or not), `page` is the page shown
/// of the six.
pub fn grid(
    at: Option<i32>,
    key: Arrow,
    page: usize,
    exists: &dyn Fn(usize) -> bool,
) -> Option<i32> {
    let Some(at) = at else { return Some(0) };
    let (g, low) = (at >> 8, at & 0xff);
    match key {
        Arrow::Up => match g {
            0 if low >= 5 => Some(low - 5),
            0 => Some(TAB_CG),
            1 | 0xf => (0..15).rev().find(|&k| exists(k)).map(|k| k as i32),
            _ => None,
        },
        Arrow::Down => match g {
            0 if low >= 10 || !exists(low as usize + 5) => {
                Some(if page + 1 == super::cgmode::PAGES {
                    PREV
                } else {
                    NEXT
                })
            }
            0 => Some(low + 5),
            0xf if low < 3 => Some(0),
            _ => None,
        },
        Arrow::Left => match g {
            0 if low % 5 != 0 => Some(low - 1),
            1 if low != 0 => Some(PREV),
            0xf if low == 3 => Some(NEXT),
            0xf if low != 0 => Some(0xf00 + low - 1),
            _ => None,
        },
        Arrow::Right => match g {
            0 if low % 5 != 4 && exists(low as usize + 1) => Some(low + 1),
            1 if low == 0 => Some(NEXT),
            1 if low == 1 => Some(BACK),
            0xf if low != 2 && low != 4 => Some(0xf00 + low + 1),
            _ => None,
        },
    }
}

/// The variant picker of the CG screen: `exists` says whether a cell is
/// on the page (seen or not).
pub fn picker(at: Option<i32>, key: Arrow, exists: &dyn Fn(usize) -> bool) -> Option<i32> {
    let Some(at) = at else { return Some(0) };
    let (g, low) = (at >> 8, at & 0xff);
    match key {
        Arrow::Up => match g {
            // The first cell of the second row does not go up.
            0 if low > 5 => Some(low - 5),
            1 | 2 => (0..15).rev().find(|&k| exists(k)).map(|k| k as i32),
            _ => None,
        },
        Arrow::Down => match g {
            0 if low >= 10 || !exists(low as usize + 5) => Some(OUTSIDE),
            0 => Some(low + 5),
            _ => None,
        },
        Arrow::Left => match g {
            0 if low % 5 != 0 => Some(low - 1),
            1 => Some(PREV),
            2 => Some(NEXT),
            _ => None,
        },
        Arrow::Right => match g {
            0 if low % 5 != 4 && exists(low as usize + 1) => Some(low + 1),
            1 if low == 0 => Some(NEXT),
            1 if low == 1 => Some(OUTSIDE),
            _ => None,
        },
    }
}

/// The music screen: 35 rows in columns of 13, 13 and 9, the stop button
/// (0x100) and the volume bar (0x200). `playing` is whether a track is
/// picked, `on` whether the music is on (the volume bar answers). The
/// answer 0x200 means the knob of the volume bar, not the middle of the
/// bar.
pub fn music(at: Option<i32>, key: Arrow, playing: bool, on: bool) -> Option<i32> {
    const STOP: i32 = 0x100;
    const VOLUME: i32 = 0x200;
    let Some(at) = at else { return Some(0) };
    let (g, low) = (at >> 8, at & 0xff);
    match key {
        Arrow::Up => match g {
            0 => Some(match low {
                0 => TAB_CG,
                13 | 26 => TAB_SCENE,
                _ => low - 1,
            }),
            1 => Some(if playing { 12 } else { 0 }),
            2 => Some(if playing && on { 12 } else { 0 }),
            0xf if low > 2 => Some(34),
            _ => None,
        },
        Arrow::Down => match g {
            0 => match low {
                12 if playing => Some(STOP),
                12 | 25 if on => Some(VOLUME),
                12 => Some(BACK),
                25 => None,
                34 => Some(BACK),
                _ => Some(low + 1),
            },
            1 => (!playing).then_some(0),
            2 => (!playing || !on).then_some(0),
            0xf if low < 3 => Some(0),
            _ => None,
        },
        Arrow::Left => match g {
            0 if low >= 13 => Some(low - 13),
            1 => (!playing).then_some(0),
            2 => Some(if !playing || !on { 0 } else { STOP }),
            0xf if low == 3 => {
                if on {
                    Some(VOLUME)
                } else {
                    playing.then_some(STOP)
                }
            }
            0xf if low != 0 => Some(0xf00 + low - 1),
            _ => None,
        },
        Arrow::Right => match g {
            // Rows 22 and 23 would land past the last row: no move.
            0 if low < 24 && low + 13 <= 34 => Some(low + 13),
            1 => Some(if !playing {
                0
            } else if on {
                VOLUME
            } else {
                BACK
            }),
            2 => Some(if on { BACK } else { 0 }),
            0xf if low != 2 && low != 4 => Some(0xf00 + low + 1),
            _ => None,
        },
    }
}

/// The middle of an area, as the game computes it from the corners.
pub fn middle((l, t, r, b): (i32, i32, i32, i32)) -> (i32, i32) {
    ((l + r) / 2, (t + b) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Arrow::*;

    fn all(_: usize) -> bool {
        true
    }

    /// The last page has 11 cells.
    fn eleven(k: usize) -> bool {
        k < 11
    }

    #[test]
    fn no_button_goes_to_the_first_cell() {
        for k in [Up, Down, Left, Right] {
            assert_eq!(grid(None, k, 0, &all), Some(0));
            assert_eq!(picker(None, k, &all), Some(0));
            assert_eq!(music(None, k, false, true), Some(0));
        }
    }

    #[test]
    fn grid_moves_inside_the_page() {
        assert_eq!(grid(Some(7), Up, 0, &all), Some(2));
        assert_eq!(grid(Some(7), Down, 0, &all), Some(12));
        assert_eq!(grid(Some(7), Left, 0, &all), Some(6));
        assert_eq!(grid(Some(7), Right, 0, &all), Some(8));
        // Edges of the columns do not wrap.
        assert_eq!(grid(Some(5), Left, 0, &all), None);
        assert_eq!(grid(Some(9), Right, 0, &all), None);
        // The top row goes to the first tab.
        assert_eq!(grid(Some(3), Up, 0, &all), Some(0xf00));
    }

    #[test]
    fn grid_bottom_row_and_last_page() {
        // Below the last row: the next arrow, or the previous one on the
        // last page.
        assert_eq!(grid(Some(12), Down, 0, &all), Some(NEXT));
        assert_eq!(grid(Some(12), Down, 5, &eleven), Some(PREV));
        // A cell with nothing below goes to the arrows too.
        assert_eq!(grid(Some(8), Down, 5, &eleven), Some(PREV));
        assert_eq!(grid(Some(5), Down, 5, &eleven), Some(10));
        // No cell to the right on a short row.
        assert_eq!(grid(Some(10), Right, 5, &eleven), None);
        assert_eq!(grid(Some(8), Right, 5, &eleven), Some(9));
        assert_eq!(grid(Some(9), Right, 5, &eleven), None);
        assert_eq!(grid(Some(10), Left, 5, &eleven), None);
        // Up from the arrows or the back button: the last cell there is.
        assert_eq!(grid(Some(NEXT), Up, 5, &eleven), Some(10));
        assert_eq!(grid(Some(BACK), Up, 0, &all), Some(14));
    }

    #[test]
    fn grid_arrows_tabs_and_buttons() {
        assert_eq!(grid(Some(PREV), Right, 1, &all), Some(NEXT));
        assert_eq!(grid(Some(NEXT), Right, 1, &all), Some(BACK));
        assert_eq!(grid(Some(NEXT), Left, 1, &all), Some(PREV));
        assert_eq!(grid(Some(PREV), Left, 1, &all), None);
        assert_eq!(grid(Some(0xf00), Right, 0, &all), Some(0xf01));
        assert_eq!(grid(Some(0xf02), Right, 0, &all), None);
        assert_eq!(grid(Some(0xf02), Left, 0, &all), Some(0xf01));
        assert_eq!(grid(Some(0xf00), Left, 0, &all), None);
        assert_eq!(grid(Some(0xf01), Down, 0, &all), Some(0));
        assert_eq!(grid(Some(BACK), Down, 0, &all), None);
        assert_eq!(grid(Some(BACK), Left, 0, &all), Some(NEXT));
        assert_eq!(grid(Some(BACK), Right, 0, &all), Some(0xf04));
        assert_eq!(grid(Some(0xf04), Right, 0, &all), None);
        assert_eq!(grid(Some(0xf04), Left, 0, &all), Some(BACK));
    }

    #[test]
    fn picker_moves() {
        // The first cell of the second row does not go up.
        assert_eq!(picker(Some(5), Up, &all), None);
        assert_eq!(picker(Some(6), Up, &all), Some(1));
        assert_eq!(picker(Some(2), Up, &all), None);
        assert_eq!(picker(Some(7), Down, &all), Some(12));
        assert_eq!(picker(Some(12), Down, &all), Some(OUTSIDE));
        assert_eq!(picker(Some(3), Down, &|k| k < 5), Some(OUTSIDE));
        assert_eq!(picker(Some(3), Right, &|k| k < 4), None);
        assert_eq!(picker(Some(2), Right, &|k| k < 4), Some(3));
        assert_eq!(picker(Some(0), Left, &all), None);
        assert_eq!(picker(Some(PREV), Right, &all), Some(NEXT));
        assert_eq!(picker(Some(NEXT), Right, &all), Some(OUTSIDE));
        assert_eq!(picker(Some(OUTSIDE), Left, &all), Some(NEXT));
        assert_eq!(picker(Some(NEXT), Left, &all), Some(PREV));
        assert_eq!(picker(Some(NEXT), Up, &|k| k < 7), Some(6));
    }

    #[test]
    fn music_rows() {
        assert_eq!(music(Some(0), Up, false, true), Some(0xf00));
        assert_eq!(music(Some(13), Up, false, true), Some(0xf02));
        assert_eq!(music(Some(26), Up, false, true), Some(0xf02));
        assert_eq!(music(Some(5), Up, false, true), Some(4));
        assert_eq!(music(Some(5), Down, false, true), Some(6));
        assert_eq!(music(Some(34), Down, false, true), Some(BACK));
        assert_eq!(music(Some(5), Left, false, true), None);
        assert_eq!(music(Some(18), Left, false, true), Some(5));
        assert_eq!(music(Some(5), Right, false, true), Some(18));
        assert_eq!(music(Some(21), Right, false, true), Some(34));
        assert_eq!(music(Some(22), Right, false, true), None);
        assert_eq!(music(Some(30), Right, false, true), None);
    }

    #[test]
    fn music_bottom_of_a_column_and_the_bars() {
        // Row 12: the stop button when a track plays, else the volume bar
        // when the music is on, else back.
        assert_eq!(music(Some(12), Down, true, true), Some(0x100));
        assert_eq!(music(Some(12), Down, false, true), Some(0x200));
        assert_eq!(music(Some(12), Down, false, false), Some(BACK));
        assert_eq!(music(Some(25), Down, false, true), Some(0x200));
        assert_eq!(music(Some(25), Down, false, false), None);
        // Stop button.
        assert_eq!(music(Some(0x100), Up, true, true), Some(12));
        assert_eq!(music(Some(0x100), Right, true, true), Some(0x200));
        assert_eq!(music(Some(0x100), Right, true, false), Some(BACK));
        assert_eq!(music(Some(0x100), Down, true, true), None);
        assert_eq!(music(Some(0x100), Down, false, true), Some(0));
        // Volume bar.
        assert_eq!(music(Some(0x200), Left, true, true), Some(0x100));
        assert_eq!(music(Some(0x200), Left, false, true), Some(0));
        assert_eq!(music(Some(0x200), Right, true, true), Some(BACK));
        assert_eq!(music(Some(0x200), Up, true, true), Some(12));
        assert_eq!(music(Some(0x200), Up, true, false), Some(0));
        // Back and the tabs.
        assert_eq!(music(Some(BACK), Left, false, true), Some(0x200));
        assert_eq!(music(Some(BACK), Left, true, false), Some(0x100));
        assert_eq!(music(Some(BACK), Left, false, false), None);
        assert_eq!(music(Some(BACK), Up, false, true), Some(34));
        assert_eq!(music(Some(0xf04), Left, false, true), Some(BACK));
        assert_eq!(music(Some(0xf01), Down, false, true), Some(0));
        assert_eq!(music(Some(0xf01), Right, false, true), Some(0xf02));
    }

    #[test]
    fn middle_of_an_area() {
        assert_eq!(middle((28, 140, 253, 267)), (140, 203));
    }
}
