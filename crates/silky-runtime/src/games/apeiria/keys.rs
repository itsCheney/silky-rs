//! Arrow keys on the extras of 景の海のアペイリア, read from the
//! executable (the four key handlers of each screen: CG mode 0x444ca0..,
//! its grid 0x447d00.., scene mode 0x459770.., music mode 0x469f10..).
//!
//! A key moves the mouse pointer to the middle of another button, so the
//! hover follows it and Enter clicks there. Each handler works on the id
//! of the button under the pointer (none: -1) with the area files' ids:
//! cells from 0, the page arrows 0x100 and 0x101, pages from 0x200, back
//! 0x300 and the mode tabs from 0xa00.

use silky_format::area::Area;

use super::omake::{self, ids, Omake};
use crate::Extras;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

/// What a key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// Put the pointer on this button (the session's ids).
    To(i32),
    /// Music mode: scroll the list one row down.
    ScrollDown,
    /// A picture over the whole screen: the previous or next one.
    Picture {
        next: bool,
    },
    Stay,
}

fn id(group: i32, k: i32) -> i32 {
    group << 8 | k
}

/// The move from button `at` (original ids, -1 for none). `on` says
/// whether cell `i` holds something that can be opened.
pub fn arrow(kind: Extras, o: &Omake, at: i32, key: Arrow, on: &dyn Fn(i32) -> bool) -> Move {
    if let Some(v) = &o.view {
        if v.picture.is_some() {
            return match key {
                Arrow::Left => Move::Picture { next: false },
                Arrow::Right => Move::Picture { next: true },
                _ => Move::Stay,
            };
        }
        return grid(at, key, on).map_or(Move::Stay, |to| Move::To(from_view(to)));
    }
    let to = match kind {
        Extras::Gallery => cg(at, key, on, 7, true),
        Extras::Scenes => cg(at, key, on, 5, false),
        Extras::Music => return music(at, key),
    };
    to.map_or(Move::Stay, Move::To)
}

/// CG mode (pages 0..=7) and scene mode (0..=5): a grid of three columns
/// and four rows, the pages and arrows to its right, the tabs to its left.
fn cg(at: i32, key: Arrow, on: &dyn Fn(i32) -> bool, last_page: i32, gallery: bool) -> Option<i32> {
    if at == -1 {
        return Some(0);
    }
    let (g, k) = (at >> 8, at & 0xff);
    let left_cell = || if on(2) { 2 } else { 1 };
    match key {
        Arrow::Up => match g {
            0 => (k > 2).then_some(k - 3),
            1 => (k == 1).then_some(id(2, last_page)),
            2 => Some(if k == 0 { 0x100 } else { id(2, k - 1) }),
            3 => Some(0xa02),
            10 if gallery => (k != 1).then_some(0xa01),
            10 => (k != 0).then_some(0xa00),
            _ => None,
        },
        Arrow::Down => match g {
            0 => (k < 12 && on(k + 3)).then_some(k + 3),
            1 => (k == 0).then_some(0x200),
            2 => Some(if k == last_page { 0x101 } else { id(2, k + 1) }),
            10 => Some(if k == 2 { 0x300 } else { 0xa02 }),
            _ => None,
        },
        Arrow::Left => match g {
            0 if k % 3 == 0 => Some(if gallery { 0xa01 } else { 0xa00 }),
            0 => Some(k - 1),
            1 if k > 1 => None,
            1 | 2 => Some(left_cell()),
            _ => None,
        },
        Arrow::Right => match g {
            0 if k % 3 == 2 => Some(0x200),
            0 => on(k + 1).then_some(k + 1),
            3 | 10 => Some(0),
            _ => None,
        },
    }
}

/// The grid of a CG entry: four columns and three rows, its arrows and
/// back below.
fn grid(at: i32, key: Arrow, on: &dyn Fn(i32) -> bool) -> Option<i32> {
    if at == -1 {
        return Some(0);
    }
    let (g, k) = (at >> 8, at & 0xff);
    match key {
        Arrow::Up => match g {
            0 => (k > 3).then_some(k - 4),
            1 | 3 => (0..12).rev().find(|&i| on(i)),
            _ => None,
        },
        Arrow::Down => match g {
            0 if k < 10 && on(k + 4) => Some(k + 4),
            0 => Some(0x100),
            _ => None,
        },
        Arrow::Left => match g {
            0 => (k % 4 != 0).then_some(k - 1),
            1 => (k == 1).then_some(0x100),
            3 => Some(0x101),
            _ => None,
        },
        Arrow::Right => match g {
            0 => (k % 4 != 3 && on(k + 1)).then_some(k + 1),
            1 if k == 0 => Some(0x101),
            1 if k == 1 => Some(0x300),
            _ => None,
        },
    }
}

/// Music mode: eight rows, the arrows and the volume to their right.
fn music(at: i32, key: Arrow) -> Move {
    let to = Move::To;
    let (g, k) = (at / 256, at % 256);
    match key {
        Arrow::Up if at == -1 => to(0x100),
        Arrow::Up => match g {
            0 if k != 0 => to(k - 1),
            1 if k == 1 => to(0x100),
            2 => to(7),
            3 => to(0xa01),
            10 if k == 1 => to(0xa00),
            _ => Move::Stay,
        },
        Arrow::Down if at == -1 => to(0),
        Arrow::Down => match g {
            0 if k == 7 => Move::ScrollDown,
            0 => to(k + 1),
            1 if k == 0 => to(0x101),
            10 if k == 0 => to(0xa01),
            10 if k == 1 => to(0x300),
            _ => Move::Stay,
        },
        // None counts as cell 0 here (the original's id / 256).
        Arrow::Left => match g {
            0 => to(0xa00),
            1 => to(0),
            2 => to(0x300),
            _ => Move::Stay,
        },
        Arrow::Right => match g {
            0 => to(0x100),
            3 | 10 => to(0),
            _ => Move::Stay,
        },
    }
}

/// The session's ids of a CG entry's grid, from the original's.
fn from_view(id: i32) -> i32 {
    match id {
        0x100 => ids::VIEW_PREVIOUS,
        0x101 => ids::VIEW_NEXT,
        0x300 => ids::VIEW_BACK,
        i => ids::VIEW_CELL + i,
    }
}

/// The original's ids of a CG entry's grid.
pub fn to_view(id: i32) -> i32 {
    match id {
        ids::VIEW_PREVIOUS => 0x100,
        ids::VIEW_NEXT => 0x101,
        ids::VIEW_BACK => 0x300,
        i if i >= ids::VIEW_CELL => i - ids::VIEW_CELL,
        i => i,
    }
}

/// Middle of a button.
pub fn centre(areas: &[Area], o: &Omake, id: i32) -> Option<(i32, i32)> {
    if let Some(v) = &o.view {
        let (l, t, r, b) = omake::view_rect(v, id)?;
        return Some(((l + r) / 2, (t + b) / 2));
    }
    let a = areas.iter().find(|a| a.id == id)?;
    Some(((a.left + a.right) / 2, (a.top + a.bottom) / 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cg_mode() {
        let all = |_| true;
        let o = Omake::default();
        let k = |at, key| arrow(Extras::Gallery, &o, at, key, &all);
        assert_eq!(k(-1, Arrow::Up), Move::To(0));
        assert_eq!(k(4, Arrow::Up), Move::To(1));
        assert_eq!(k(3, Arrow::Left), Move::To(0xa01));
        assert_eq!(k(5, Arrow::Right), Move::To(0x200));
        assert_eq!(k(0x207, Arrow::Down), Move::To(0x101));
        assert_eq!(k(0x101, Arrow::Up), Move::To(0x207));
        assert_eq!(k(0xa02, Arrow::Down), Move::To(0x300));
        let s = |at, key| arrow(Extras::Scenes, &o, at, key, &all);
        assert_eq!(s(0x205, Arrow::Down), Move::To(0x101));
        assert_eq!(s(0, Arrow::Left), Move::To(0xa00));
    }

    #[test]
    fn music_and_grid() {
        let o = Omake::default();
        let m = |at, key| arrow(Extras::Music, &o, at, key, &|_| true);
        assert_eq!(m(7, Arrow::Down), Move::ScrollDown);
        assert_eq!(m(2, Arrow::Right), Move::To(0x100));
        assert_eq!(grid(5, Arrow::Down, &|i| i < 7), Some(0x100));
        assert_eq!(grid(0x300, Arrow::Up, &|i| i < 7), Some(6));
    }
}
