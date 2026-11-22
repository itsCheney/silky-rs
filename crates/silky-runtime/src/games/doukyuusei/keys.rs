//! Arrow keys on the selections of 同級生 Remake, read from the
//! executable.
//!
//! A key moves the mouse pointer to the middle of another button, so the
//! hover follows it and Enter clicks there. The focus is the selection
//! element under the pointer (bars 0..n of the page, 4 the previous-page
//! arrow above, 5 the next-page arrow below, -1 for none); otherwise the
//! hovered button is a bottom button of the message window (ids 0..=6 of
//! `mes_o.area`) or one of the menu buttons (0 the menu button, 1..=6 the
//! six of the open menu, in [`super::menu::BUTTONS`] order).

pub use crate::games::apeiria::keys::Arrow;

/// The previous-page arrow's focus id; the next-page arrow is 5.
pub const PREV: i32 = 4;
pub const NEXT: i32 = 5;

/// The button under the pointer when no selection element is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hover {
    None,
    /// A bottom button of the message window, by `mes_o.area` id.
    Bottom(i32),
    /// The menu button (0) or one of its six (1..=6).
    Menu(i32),
}

/// Where a key puts the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A bar of the page, 0..n.
    Bar(i32),
    Prev,
    Next,
    Bottom(i32),
    Menu(i32),
}

/// What the keys depend on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    /// The focused element (see the module notes), -1 for none.
    pub focus: i32,
    pub hover: Hover,
    /// Bars on this page.
    pub bars: i32,
    pub page: usize,
    pub last_page: usize,
    pub menu_open: bool,
    /// Whether the menu button of each index is left out.
    pub hidden: [bool; 7],
}

/// The target of a key, if it moves anywhere.
pub fn arrow(s: &State, key: Arrow) -> Option<Target> {
    let (f, n) = (s.focus, s.bars);
    let hidden = |k: usize| s.hidden[k];
    let g0 = Some(Target::Bar(0));
    let last_bar = Some(Target::Bar(n - 1));
    let b0 = Some(Target::Bottom(0));
    match key {
        Arrow::Up => {
            if f != -1 {
                return match f {
                    0 => (s.page > 0).then_some(Target::Prev),
                    NEXT => last_bar,
                    PREV => None,
                    f => Some(Target::Bar(f - 1)),
                };
            }
            match s.hover {
                Hover::None => g0,
                Hover::Menu(_) if !s.menu_open => Some(Target::Menu(0)),
                Hover::Menu(0) => None,
                Hover::Menu(6) => {
                    if hidden(6) {
                        b0
                    } else if hidden(5) {
                        Some(Target::Menu(4))
                    } else {
                        Some(Target::Menu(5))
                    }
                }
                Hover::Menu(5) => {
                    if hidden(5) {
                        b0
                    } else {
                        Some(Target::Menu(4))
                    }
                }
                Hover::Menu(k) => Some(Target::Menu(k - 1)),
                Hover::Bottom(_) => last_bar,
            }
        }
        Arrow::Down => {
            if f != -1 {
                return match f {
                    PREV => g0,
                    NEXT => b0,
                    f if f == n - 1 => {
                        if s.page == s.last_page {
                            b0
                        } else {
                            Some(Target::Next)
                        }
                    }
                    f => Some(Target::Bar(f + 1)),
                };
            }
            match s.hover {
                Hover::None => g0,
                Hover::Menu(_) if !s.menu_open => Some(Target::Bottom(6)),
                Hover::Menu(4) => {
                    if hidden(5) {
                        Some(if hidden(6) {
                            Target::Bottom(6)
                        } else {
                            Target::Menu(6)
                        })
                    } else {
                        Some(Target::Menu(5))
                    }
                }
                Hover::Menu(5) => Some(if hidden(5) || hidden(6) {
                    Target::Bottom(6)
                } else {
                    Target::Menu(6)
                }),
                Hover::Menu(6) => Some(Target::Bottom(6)),
                Hover::Menu(k) => Some(Target::Menu(k + 1)),
                Hover::Bottom(5 | 6) => b0,
                Hover::Bottom(_) => None,
            }
        }
        Arrow::Left => match s.hover {
            Hover::None => (f == -1).then_some(Target::Bar(0)),
            Hover::Menu(_) => g0,
            Hover::Bottom(k @ (1..=4 | 6)) => Some(Target::Bottom(k - 1)),
            Hover::Bottom(_) => None,
        },
        Arrow::Right => {
            if f != -1 {
                return Some(Target::Menu(0));
            }
            match s.hover {
                Hover::None => g0,
                Hover::Menu(_) => (!s.menu_open).then_some(Target::Menu(0)),
                Hover::Bottom(k @ (0..=3 | 5)) => Some(Target::Bottom(k + 1)),
                Hover::Bottom(_) => None,
            }
        }
    }
}

/// Fixed screen points of the menu button and its six.
pub const MENU_POINTS: [(i32, i32); 7] = [
    (1820, 91),
    (1732, 189),
    (1813, 277),
    (1735, 360),
    (1818, 447),
    (1737, 532),
    (1821, 616),
];

#[cfg(test)]
mod tests {
    use super::*;
    use Arrow::*;

    fn st(focus: i32, hover: Hover) -> State {
        State {
            focus,
            hover,
            bars: 4,
            page: 0,
            last_page: 0,
            menu_open: true,
            hidden: [false; 7],
        }
    }

    fn go(s: State, k: Arrow) -> Option<Target> {
        arrow(&s, k)
    }

    #[test]
    fn up_on_bars() {
        assert_eq!(go(st(0, Hover::None), Up), None);
        let mut s = st(0, Hover::None);
        s.page = 1;
        assert_eq!(go(s, Up), Some(Target::Prev));
        assert_eq!(go(st(2, Hover::None), Up), Some(Target::Bar(1)));
        assert_eq!(go(st(5, Hover::None), Up), Some(Target::Bar(3)));
        assert_eq!(go(st(4, Hover::None), Up), None);
    }

    #[test]
    fn up_from_buttons() {
        assert_eq!(go(st(-1, Hover::None), Up), Some(Target::Bar(0)));
        for h in [
            Hover::Bottom(0),
            Hover::Bottom(4),
            Hover::Bottom(5),
            Hover::Bottom(6),
        ] {
            assert_eq!(go(st(-1, h), Up), Some(Target::Bar(3)));
        }
        let mut s = st(-1, Hover::Menu(3));
        assert_eq!(go(s, Up), Some(Target::Menu(2)));
        assert_eq!(go(st(-1, Hover::Menu(0)), Up), None);
        s.menu_open = false;
        assert_eq!(go(s, Up), Some(Target::Menu(0)));
        assert_eq!(go(st(-1, Hover::Menu(6)), Up), Some(Target::Menu(5)));
        let mut s = st(-1, Hover::Menu(6));
        s.hidden[5] = true;
        assert_eq!(go(s, Up), Some(Target::Menu(4)));
        s.hidden[6] = true;
        assert_eq!(go(s, Up), Some(Target::Bottom(0)));
        let mut s = st(-1, Hover::Menu(5));
        assert_eq!(go(s, Up), Some(Target::Menu(4)));
        s.hidden[5] = true;
        assert_eq!(go(s, Up), Some(Target::Bottom(0)));
    }

    #[test]
    fn down_on_bars() {
        assert_eq!(go(st(4, Hover::None), Down), Some(Target::Bar(0)));
        assert_eq!(go(st(5, Hover::None), Down), Some(Target::Bottom(0)));
        assert_eq!(go(st(1, Hover::None), Down), Some(Target::Bar(2)));
        // The last bar: the bottom buttons on the last page, else the arrow.
        assert_eq!(go(st(3, Hover::None), Down), Some(Target::Bottom(0)));
        let mut s = st(3, Hover::None);
        s.last_page = 1;
        assert_eq!(go(s, Down), Some(Target::Next));
        let mut s = st(1, Hover::None);
        s.bars = 2;
        assert_eq!(go(s, Down), Some(Target::Bottom(0)));
    }

    #[test]
    fn down_from_buttons() {
        assert_eq!(go(st(-1, Hover::None), Down), Some(Target::Bar(0)));
        let mut s = st(-1, Hover::Menu(2));
        assert_eq!(go(s, Down), Some(Target::Menu(3)));
        s.menu_open = false;
        assert_eq!(go(s, Down), Some(Target::Bottom(6)));
        assert_eq!(go(st(-1, Hover::Menu(0)), Down), Some(Target::Menu(1)));
        assert_eq!(go(st(-1, Hover::Menu(4)), Down), Some(Target::Menu(5)));
        let mut s = st(-1, Hover::Menu(4));
        s.hidden[5] = true;
        assert_eq!(go(s, Down), Some(Target::Menu(6)));
        s.hidden[6] = true;
        assert_eq!(go(s, Down), Some(Target::Bottom(6)));
        let mut s = st(-1, Hover::Menu(5));
        assert_eq!(go(s, Down), Some(Target::Menu(6)));
        s.hidden[6] = true;
        assert_eq!(go(s, Down), Some(Target::Bottom(6)));
        let mut s = st(-1, Hover::Menu(5));
        s.hidden[5] = true;
        assert_eq!(go(s, Down), Some(Target::Bottom(6)));
        assert_eq!(go(st(-1, Hover::Menu(6)), Down), Some(Target::Bottom(6)));
        assert_eq!(go(st(-1, Hover::Bottom(5)), Down), Some(Target::Bottom(0)));
        assert_eq!(go(st(-1, Hover::Bottom(6)), Down), Some(Target::Bottom(0)));
        assert_eq!(go(st(-1, Hover::Bottom(2)), Down), None);
    }

    #[test]
    fn left() {
        assert_eq!(go(st(-1, Hover::None), Left), Some(Target::Bar(0)));
        assert_eq!(go(st(2, Hover::None), Left), None);
        assert_eq!(go(st(-1, Hover::Menu(3)), Left), Some(Target::Bar(0)));
        for (from, to) in [(6, 5), (4, 3), (3, 2), (2, 1), (1, 0)] {
            assert_eq!(
                go(st(-1, Hover::Bottom(from)), Left),
                Some(Target::Bottom(to))
            );
        }
        assert_eq!(go(st(-1, Hover::Bottom(0)), Left), None);
        assert_eq!(go(st(-1, Hover::Bottom(5)), Left), None);
    }

    #[test]
    fn right() {
        assert_eq!(go(st(1, Hover::None), Right), Some(Target::Menu(0)));
        assert_eq!(go(st(5, Hover::None), Right), Some(Target::Menu(0)));
        assert_eq!(go(st(-1, Hover::None), Right), Some(Target::Bar(0)));
        assert_eq!(go(st(-1, Hover::Menu(2)), Right), None);
        let mut s = st(-1, Hover::Menu(2));
        s.menu_open = false;
        assert_eq!(go(s, Right), Some(Target::Menu(0)));
        assert_eq!(go(st(-1, Hover::Bottom(5)), Right), Some(Target::Bottom(6)));
        assert_eq!(go(st(-1, Hover::Bottom(6)), Right), None);
        for k in 0..4 {
            assert_eq!(
                go(st(-1, Hover::Bottom(k)), Right),
                Some(Target::Bottom(k + 1))
            );
        }
        assert_eq!(go(st(-1, Hover::Bottom(4)), Right), None);
    }
}
