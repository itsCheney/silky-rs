//! What the investigation screens share: the bar of system buttons at the
//! lower left, the information strip with its help line, and the numbering
//! of the areas in their `.area` files (the high byte is the kind of
//! element, the low byte its number).

use silky_format::area::Area;

use super::screen::{Click, Look};
use crate::assets::Assets;
use crate::render::Canvas;
use crate::Result;

/// The sheet all these screens draw from.
pub const SHEET: &str = "move_p.akb";

/// Id of the first system button; the eight follow it.
pub const BAR: i32 = 0xf00;
/// Number of system buttons.
pub const BAR_LEN: usize = 8;

/// Help line of each system button.
const BAR_HELP: [&str; BAR_LEN] = [
    "システム設定を開きます",
    "サウンド設定を開きます",
    "履歴画面を開きます",
    "セーブ画面を開きます",
    "ロード画面を開きます",
    "クイックセーブします",
    "クイックロードします",
    "捜査手帳を開きます",
];

/// Where the help line sits, and its text size.
const HELP_POS: (i32, i32) = (716, 695);
const HELP_SIZE: f32 = 16.0;

/// What each system button does, as a message window button
/// ([`crate::ui::bar`]). The sound settings open the settings screen.
const BAR_ACTIONS: [Option<i32>; BAR_LEN] = {
    use crate::ui::bar;
    [
        Some(bar::CONFIG),
        Some(bar::CONFIG),
        Some(bar::BACKLOG),
        Some(bar::SAVE),
        Some(bar::LOAD),
        Some(bar::QUICK_SAVE),
        Some(bar::QUICK_LOAD),
        Some(bar::K1_NOTEBOOK),
    ]
};

/// A press on system button `id`: the message window button it stands
/// for. `None` when `id` is not a system button.
pub fn bar_click(id: i32) -> Option<Click> {
    let i = bar_index(id)?;
    Some(BAR_ACTIONS[i].map_or(Click::None, Click::Bar))
}

/// Index of the system button `id` names.
pub fn bar_index(id: i32) -> Option<usize> {
    (BAR..BAR + BAR_LEN as i32)
        .contains(&id)
        .then(|| (id - BAR) as usize)
}

/// What the engine looks at when it decides which system buttons are grey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarFacts {
    /// H 69: a scene of the scene list is replayed.
    pub replay: bool,
    /// H 77: a second lock that greys the same buttons. No script sets it.
    pub locked: bool,
    /// The backlog has lines. The window's own log switch is on whenever a
    /// screen can be up, so it is not a separate input.
    pub log: bool,
    /// Some save slot, a quick save included, holds a save.
    pub saves: bool,
    /// The quick save slot holds a save.
    pub quick: bool,
    /// Byte 899: the notebook is unlocked.
    pub notebook: bool,
}

impl Default for BarFacts {
    /// Nothing locked, everything there to use.
    fn default() -> Self {
        Self {
            replay: false,
            locked: false,
            log: true,
            saves: true,
            quick: true,
            notebook: true,
        }
    }
}

/// The two rules the screens' bars follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarKind {
    /// Action menu, area and item selection and the map.
    Action,
    /// The option list. Its rule was written for a bar of six buttons
    /// (the notebook last), so on the eight of the sheet it greys the quick
    /// save button for a locked notebook, and the quick load and the
    /// notebook buttons are never grey.
    Option,
}

/// Which of the bar's buttons are grey.
pub fn grey(kind: BarKind, f: &BarFacts) -> [bool; BAR_LEN] {
    let mut off = [false; BAR_LEN];
    let scene = f.replay || f.locked;
    off[2] = !f.log;
    off[3] = scene;
    off[4] = scene || !f.saves;
    match kind {
        BarKind::Action => {
            off[5] = f.replay;
            off[6] = f.replay || !f.quick;
            off[7] = !f.notebook;
        }
        BarKind::Option => off[5] = !f.notebook,
    }
    off
}

/// The area under (x, y) that `want` accepts.
pub fn hit_area(areas: &[Area], x: i32, y: i32, want: impl Fn(i32) -> bool) -> Option<&Area> {
    areas.iter().find(|a| want(a.id) && a.contains(x, y))
}

/// Row of a button's picture: 0 at rest, 1 under the pointer, 2 pressed.
pub fn state(look: &Look, id: i32) -> u32 {
    if look.pressed == Some(id) {
        2
    } else if look.hover == Some(id) {
        1
    } else {
        0
    }
}

/// The two strips at the bottom, over which the help line is written.
pub fn draw_strips(canvas: &mut Canvas, sheet: &std::sync::Arc<silky_format::akb::Image>) {
    canvas.draw_part(sheet, (548, 30, 545, 30), 0, 687, false, 255);
    canvas.draw_part(sheet, (548, 0, 513, 30), 602, 687, false, 255);
}

/// The system buttons; `off` are drawn grey.
pub fn draw_bar(
    canvas: &mut Canvas,
    sheet: &std::sync::Arc<silky_format::akb::Image>,
    look: &Look,
    off: &[bool; BAR_LEN],
) {
    for (i, &grey) in off.iter().enumerate() {
        let row = if grey { 3 } else { state(look, BAR + i as i32) };
        canvas.draw_part(
            sheet,
            (80 + 52 * i as u32, 38 * row, 52, 38),
            40 + 60 * i as i32,
            679,
            false,
            255,
        );
    }
}

/// The help line of the system button under the pointer, if it is one.
pub fn bar_help(look: &Look) -> Option<&'static str> {
    look.hover.and_then(bar_index).map(|i| BAR_HELP[i])
}

/// Writes `line` into the information strip.
pub fn draw_help(canvas: &mut Canvas, look: &Look, line: Option<&str>) {
    if let (Some(text), Some(line)) = (look.text, line) {
        text.draw_plain(
            canvas,
            line,
            HELP_POS.0 as f32,
            (HELP_POS.1 + 13) as f32,
            HELP_SIZE,
            [255, 255, 255, 255],
        );
    }
}

/// Loads the sheet and draws the strips and the bar.
pub fn draw_common(
    canvas: &mut Canvas,
    assets: &mut Assets,
    look: &Look,
    off: &[bool; BAR_LEN],
) -> Result<std::sync::Arc<silky_format::akb::Image>> {
    let sheet = assets.image(SHEET)?;
    draw_strips(canvas, &sheet);
    draw_bar(canvas, &sheet, look, off);
    Ok(sheet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_action_bar_follows_its_rule_button_by_button() {
        let f = BarFacts::default();
        assert_eq!(grey(BarKind::Action, &f), [false; BAR_LEN]);
        // Nothing to load, no backlog, no quick save, notebook locked.
        let none = BarFacts {
            log: false,
            saves: false,
            quick: false,
            notebook: false,
            ..f
        };
        assert_eq!(
            grey(BarKind::Action, &none),
            [false, false, true, false, true, false, true, true]
        );
        // Quick load needs the quick slot, load any slot.
        let some = BarFacts { quick: false, ..f };
        assert_eq!(
            grey(BarKind::Action, &some),
            [false, false, false, false, false, false, true, false]
        );
    }

    #[test]
    fn a_replay_greys_save_load_and_the_quick_pair_but_not_the_notebook() {
        let f = BarFacts {
            replay: true,
            ..BarFacts::default()
        };
        assert_eq!(
            grey(BarKind::Action, &f),
            [false, false, false, true, true, true, true, false]
        );
    }

    #[test]
    fn the_second_lock_greys_save_and_load_only() {
        let f = BarFacts {
            locked: true,
            ..BarFacts::default()
        };
        assert_eq!(
            grey(BarKind::Action, &f),
            [false, false, false, true, true, false, false, false]
        );
        assert_eq!(
            grey(BarKind::Option, &f),
            [false, false, false, true, true, false, false, false]
        );
    }

    #[test]
    fn the_option_list_has_its_own_rule() {
        let f = BarFacts {
            replay: true,
            log: false,
            saves: false,
            quick: false,
            notebook: false,
            ..BarFacts::default()
        };
        // Quick save follows the notebook byte; quick load and the
        // notebook button are never grey; a replay does not touch the
        // quick save button.
        assert_eq!(
            grey(BarKind::Option, &f),
            [false, false, true, true, true, true, false, false]
        );
        let unlocked = BarFacts {
            notebook: true,
            ..f
        };
        assert!(!grey(BarKind::Option, &unlocked)[5]);
    }
}
