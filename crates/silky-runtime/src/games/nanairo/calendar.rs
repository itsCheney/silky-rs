//! The calendar of a date change in なないろリンカネーション (`Game.39`).
//!
//! The new day's background drifts in behind two white curtains that
//! open; the month's name fades in, then a column of dates, a grey bar
//! slides in, a disc lands on the column and the column scrolls down by
//! the number of days that passed, so that the new day ends under the
//! disc. The part of the column under the disc is drawn in yellow while
//! it scrolls. After a hold the curtains leave, the screen fades out and
//! the script goes on over the new background.
//!
//! All the pieces are cut from `Interlude.akb`; `InterludeBG.akb` is a
//! fine veil over the background. Times are frames of 16 ms at the
//! effect speed. Moves and fades are taken to be even, and the layers
//! that come in later start hidden.

use silky_vm::Flow;

use super::effects::{clear, scaled, Fx};
use crate::games::apeiria::anim::{Key, Look, Step};
use crate::games::Call;
use crate::host::Runtime;

/// Owner of the calendar's sprites on the effect stage.
pub(super) const CALENDAR: u32 = 39;

/// The sheet the pieces are cut from (the script loads it into sprite
/// plane 3) and the veil.
const SHEET: &str = "Interlude.akb";
const VEIL: &str = "InterludeBG.akb";

/// Drawing order, from the back: the new background, the veil, the
/// curtains, the bar, the disc, the month and the column.
const Z_BACK: i32 = 30;

/// The word variable that holds the table row of the new day.
const DAY_WORD: usize = 100;

/// A date cell is 92 pixels apart in the column; a digit is 47 by 52 of
/// the sheet, a weekday label 19 by 45.
const PITCH: i32 = 92;
const DIGIT: (u32, u32) = (47, 52);
const LABEL: (u32, u32) = (19, 45);
/// Width of the column and where the disc and its window are.
const COLUMN_W: u32 = 132;
const COLUMN_X: i32 = 603;
const DISC: (i32, i32) = (603, 521);
const DISC_SIZE: i32 = 132;
/// Frames of the pieces, as in the original.
const DRIFT: i32 = 260;
const CURTAINS_IN: i32 = 32;
const MONTH_IN: i32 = 32;
const COLUMN_IN: i32 = 16;
const BAR_IN: i32 = 12;
const DISC_IN: i32 = 12;
const SCROLL: i32 = 32;
const CURTAINS_OUT: i32 = 20;
const FADE_OUT: i32 = 20;
const VEIL_OUT: i32 = 16;

/// The dates of the story days: month, day and weekday (0 Monday).
const DATES: [(u8, u8, u8); 31] = [
    (8, 18, 0),
    (8, 19, 1),
    (8, 20, 2),
    (8, 21, 3),
    (8, 22, 4),
    (8, 25, 0),
    (8, 26, 1),
    (8, 28, 3),
    (8, 29, 4),
    (8, 30, 5),
    (8, 31, 6),
    (9, 1, 0),
    (9, 2, 1),
    (9, 3, 2),
    (9, 4, 3),
    (9, 5, 4),
    (9, 6, 5),
    (9, 7, 6),
    (9, 8, 0),
    (9, 9, 1),
    (9, 10, 2),
    (9, 11, 3),
    (9, 12, 4),
    (9, 13, 5),
    (9, 15, 0),
    (9, 16, 1),
    (9, 17, 2),
    (9, 20, 5),
    (9, 21, 6),
    (9, 22, 0),
    (9, 26, 4),
];

/// Rows of coloured digits on the sheet, in cells of 52 pixels.
const GREY: usize = 0;
const BLUE: usize = 2;
const RED: usize = 3;

/// A rectangle of the sheet and where it goes.
type Piece = ((u32, u32, u32, u32), (i32, i32));

/// One cell of the column.
#[derive(Debug, Clone, PartialEq)]
struct Cell {
    /// The number shown (1 to 31) and its weekday (0 Monday).
    number: i32,
    weekday: i32,
    /// Top of the cell in the column.
    y: i32,
    /// Row of the sheet's digits: [`GREY`], [`BLUE`] or [`RED`] (the row
    /// between them is the highlighted one).
    colour: usize,
}

/// The cells of a column that scrolls `days` days to the date of table
/// row `index`: `days + 8` of them, the later dates higher up, the new day
/// at 562 whatever the number of days is. The numbers wrap at 31 whatever
/// the month.
fn cells(index: usize, days: i32) -> Vec<Cell> {
    let (month, day, weekday) = DATES[index.min(DATES.len() - 1)];
    let (day, weekday) = (day as i32, weekday as i32);
    (0..days + 8)
        .map(|k| {
            let raw = day - days - 1 + k;
            let mut number = raw;
            if number < 0 {
                number += 31;
            }
            if number == 0 {
                number = 31;
            }
            if number > 31 {
                number -= 31;
            }
            let w = (weekday - day + raw).rem_euclid(7);
            let colour = if w == 6 {
                RED
            } else if w == 5 {
                BLUE
            } else if month == 9 && (number == 15 || number == 23) {
                RED
            } else {
                GREY
            };
            Cell {
                number,
                weekday: w,
                y: 10 + PITCH * (days + 7 - k),
                colour,
            }
        })
        .collect()
}

/// The pieces of the column's picture: the tens digit (when there is
/// one) and the units digit of each date, and its weekday. The yellow
/// column has the same cells with the highlighted digits and labels.
fn column_pieces(cells: &[Cell], yellow: bool) -> Vec<Piece> {
    let mut out = Vec::new();
    for c in cells {
        let digit_y = if yellow {
            346
        } else {
            294 + 52 * c.colour as u32
        };
        let label_y = if yellow { 339 } else { 294 };
        if c.number >= 10 {
            out.push((
                (47 * (c.number / 10) as u32, digit_y, DIGIT.0, DIGIT.1),
                (4, c.y),
            ));
        }
        out.push((
            (47 * (c.number % 10) as u32, digit_y, DIGIT.0, DIGIT.1),
            (51, c.y),
        ));
        out.push((
            (602 + 19 * c.weekday as u32, label_y, LABEL.0, LABEL.1),
            (100, c.y + 2),
        ));
    }
    out
}

/// The picture of a column of `days` days.
fn column_look(cells: &[Cell], days: i32, yellow: bool) -> Look {
    Look::Pieces {
        name: SHEET.to_string(),
        w: COLUMN_W,
        h: (720 + PITCH * days) as u32,
        pieces: column_pieces(cells, yellow),
    }
}

/// What a sprite starts as.
struct Start {
    at: (i32, i32),
    shown: bool,
    alpha: u8,
    clip: Option<(i32, i32, i32, i32)>,
}

fn put(fx: &mut Fx, slot: u32, look: Look, z: i32, s: Start) -> u32 {
    let id = fx.sprite(slot, look, z);
    if let Some(Step::Add(sp)) = fx.steps.last_mut() {
        sp.x = s.at.0 as f32;
        sp.y = s.at.1 as f32;
        sp.shown = s.shown;
        sp.alpha = s.alpha;
        sp.clip = s.clip;
    }
    id
}

/// The program of the calendar: `index` is the table row of the new day,
/// `days` the days the column scrolls, `background` the picture of the
/// new background (on sprite plane 2), `f` the effect speed's factor.
pub fn calendar(
    st: &mut crate::games::apeiria::anim::Stage,
    index: usize,
    days: i32,
    background: Option<&str>,
    f: f32,
) {
    clear(st, CALENDAR);
    let n = |frames: i32| scaled(frames, f);
    let days = days.max(0);
    let cells = cells(index, days);
    let mut fx = Fx::new(st, CALENDAR);
    let hidden = |at: (i32, i32)| Start {
        at,
        shown: false,
        alpha: 0,
        clip: None,
    };
    // Sprites that only move in, or take the place of the column.
    let moving = |at: (i32, i32)| Start {
        alpha: 255,
        ..hidden(at)
    };
    let shown = |at: (i32, i32)| Start {
        at,
        shown: true,
        alpha: 255,
        clip: None,
    };
    let top = -PITCH * days;
    let (month, ..) = DATES[index.min(DATES.len() - 1)];
    let month_part = if month == 8 {
        (0, 0, 489, 220)
    } else {
        (489, 0, 489, 220)
    };

    let back = background.map(|b| put(&mut fx, 0, Look::picture(b), Z_BACK, shown((120, 0))));
    let veil = put(&mut fx, 1, Look::picture(VEIL), Z_BACK + 1, shown((0, 0)));
    let white = |w| Look::Fill {
        color: [255; 4],
        w,
        h: 720,
    };
    let left = put(&mut fx, 2, white(895), Z_BACK + 2, shown((0, 0)));
    let right = put(&mut fx, 3, white(385), Z_BACK + 2, shown((895, 0)));
    let bar = put(
        &mut fx,
        4,
        Look::part(SHEET, (0, 220, 1280, 74)),
        Z_BACK + 3,
        moving((1280, 550)),
    );
    let disc = put(
        &mut fx,
        5,
        Look::part(SHEET, (470, 294, 132, 132)),
        Z_BACK + 4,
        hidden(DISC),
    );
    let month = put(
        &mut fx,
        6,
        Look::part(SHEET, month_part),
        Z_BACK + 5,
        hidden((120, 318)),
    );
    // The column in four sprites: whole while it fades in, and then (the
    // disc's window being yellow) the part above the window, the part
    // below it and the yellow part in it.
    let grey = column_look(&cells, days, false);
    let yellow_look = column_look(&cells, days, true);
    let z = Z_BACK + 6;
    let whole = put(&mut fx, 7, grey.clone(), z, hidden((COLUMN_X, top)));
    let clip = |y: i32, h: i32| Start {
        clip: Some((COLUMN_X, y, COLUMN_W as i32, h)),
        ..moving((COLUMN_X, top))
    };
    let above = put(&mut fx, 8, grey.clone(), z, clip(0, DISC.1));
    let below = put(
        &mut fx,
        9,
        grey,
        z,
        clip(DISC.1 + DISC_SIZE, 720 - DISC.1 - DISC_SIZE),
    );
    let window = put(&mut fx, 10, yellow_look, z, clip(DISC.1, DISC_SIZE));
    let mut all = vec![
        left, right, bar, disc, month, whole, above, below, window, veil,
    ];
    all.extend(back);
    let split = [above, below, window];

    // State 0: the curtains open while the background drifts.
    if let Some(back) = back {
        fx.play(
            vec![(
                back,
                vec![Key::new(n(DRIFT)).show(true).moving((120, 0), (0, 0))],
            )],
            false,
        );
    }
    let drift = fx.steps.iter().rev().find_map(|s| match s {
        Step::Start(g, _) => Some(*g),
        _ => None,
    });
    fx.play(
        vec![
            (
                left,
                vec![Key::new(n(CURTAINS_IN))
                    .show(true)
                    .moving((0, 0), (-155, 0))],
            ),
            (
                right,
                vec![Key::new(n(CURTAINS_IN))
                    .show(true)
                    .moving((895, 0), (1050, 0))],
            ),
        ],
        true,
    );
    // 1: the month's name.
    fx.play(
        vec![(month, vec![Key::new(n(MONTH_IN)).show(true).fade(0, 255)])],
        true,
    );
    // 2: the column.
    fx.play(
        vec![(whole, vec![Key::new(n(COLUMN_IN)).show(true).fade(0, 255)])],
        true,
    );
    // 3: the bar.
    fx.play(
        vec![(
            bar,
            vec![Key::new(n(BAR_IN)).show(true).moving((1280, 550), (0, 550))],
        )],
        true,
    );
    // 4: the disc lands, and the window under it turns yellow.
    let mut disc_keys = vec![
        (
            disc,
            vec![Key::new(n(DISC_IN)).show(true).fade(0, 255).scaling(
                (1.2, 1.2),
                (1.0, 1.0),
                (62.0, 62.0),
            )],
        ),
        (whole, vec![Key::new(1).show(false)]),
    ];
    disc_keys.extend(split.iter().map(|&id| (id, vec![Key::new(1).show(true)])));
    fx.play(disc_keys, true);
    // 5: the column scrolls to the new day; the window stays.
    fx.play(
        split
            .iter()
            .map(|&id| {
                (
                    id,
                    vec![Key::new(n(SCROLL))
                        .show(true)
                        .moving((COLUMN_X, top), (COLUMN_X, 0))],
                )
            })
            .collect(),
        true,
    );
    // 6: the rest of the drift is the hold.
    if let Some(g) = drift {
        fx.steps.push(Step::Wait(g));
    }
    // 7: the curtains leave, everything fades out, and the calendar is
    // taken away over the new background.
    let out = vec![
        (
            left,
            vec![Key::new(n(CURTAINS_OUT)).moving((-155, 0), (-895, 0))],
        ),
        (
            right,
            vec![Key::new(n(CURTAINS_OUT)).moving((1050, 0), (1280, 0))],
        ),
    ];
    fx.play(out, false);
    let curtains = fx.steps.iter().rev().find_map(|s| match s {
        Step::Start(g, _) => Some(*g),
        _ => None,
    });
    let mut fades: Vec<(u32, Vec<Key>)> = [bar, month, disc, above, below, window]
        .iter()
        .map(|&id| (id, vec![Key::new(n(FADE_OUT)).show(false).fade(255, 0)]))
        .collect();
    fades.push((veil, vec![Key::new(n(VEIL_OUT)).show(false).fade(255, 0)]));
    fx.play(fades, false);
    let fading = fx.steps.iter().rev().find_map(|s| match s {
        Step::Start(g, _) => Some(*g),
        _ => None,
    });
    for g in [curtains, fading].into_iter().flatten() {
        fx.steps.push(Step::Wait(g));
    }
    fx.remove(&all);
    fx.run();
}

impl Runtime {
    /// `Game.39(days)`: the calendar. The script waits until it is over;
    /// a click ends it at once. With the scenes skipped in the
    /// configuration nothing is done.
    pub(crate) fn nanairo_calendar(
        &mut self,
        vm: &silky_vm::Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        if self.scene_skip() {
            return self.done();
        }
        // The calendar writes an auto save as it starts.
        self.nanairo_auto_save();
        // The one value is the last before the command, so it may arrive
        // as the sub command.
        let days = if c.sub >= 0 { c.sub } else { c.int(0) };
        let index = vm.words.get(DAY_WORD).copied().unwrap_or(0) as usize;
        let background = self.scene.layers.get(&super::BACKGROUND_SPRITE).cloned();
        let f = self.system.settings.effect_scale();
        // The calendar is drawn on a screen that was faded out to white
        // before it; the new day's background is up when it ends, so the
        // cover is gone.
        self.scene.cover = None;
        // The message window is closed.
        self.scene.text = None;
        self.scene.speaker_name = None;
        calendar(
            &mut self.scene.effects,
            index,
            days,
            background.as_deref(),
            f,
        );
        self.nanairo_effect_wait()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::apeiria::anim::{Stage, TICK_MS};

    fn cell(cells: &[Cell], number: i32) -> Cell {
        cells.iter().find(|c| c.number == number).unwrap().clone()
    }

    #[test]
    fn the_first_day_is_a_tuesday_in_august() {
        let cs = cells(1, 1);
        assert_eq!(cs.len(), 9);
        // From Sunday the 17th, the 25th on top.
        assert_eq!(cs[0].number, 17);
        assert_eq!((cs[0].weekday, cs[0].colour, cs[0].y), (6, RED, 746));
        assert_eq!(cs[8].number, 25);
        assert_eq!(cs[8].y, 10);
        let new = cell(&cs, 19);
        assert_eq!((new.weekday, new.colour, new.y), (1, GREY, 562));
        let pieces = column_pieces(&[new], false);
        assert_eq!(pieces[0], ((47, 294, 47, 52), (4, 562)));
        assert_eq!(pieces[1], ((423, 294, 47, 52), (51, 562)));
        assert_eq!(pieces[2], ((621, 294, 19, 45), (100, 564)));
        // The 23rd is a Saturday, the 24th a Sunday.
        assert_eq!(cell(&cs, 23).colour, BLUE);
        assert_eq!(cell(&cs, 24).colour, RED);
    }

    #[test]
    fn weekend_days_have_their_colours() {
        // Aug 30 is a Saturday: blue digits.
        let cs = cells(9, 1);
        let new = cell(&cs, 30);
        assert_eq!((new.weekday, new.colour), (5, BLUE));
        let pieces = column_pieces(&[new], false);
        assert_eq!(pieces[0], ((141, 398, 47, 52), (4, 562)));
        assert_eq!(pieces[2].0, (697, 294, 19, 45));
        // Aug 31 is a Sunday: red, and the next dates wrap.
        let cs = cells(10, 1);
        let new = cell(&cs, 31);
        assert_eq!((new.weekday, new.colour), (6, RED));
        let pieces = column_pieces(&[new], false);
        assert_eq!(pieces[0], ((141, 450, 47, 52), (4, 562)));
        assert_eq!(pieces[1].0, (47, 450, 47, 52));
        assert_eq!(pieces[2].0, (716, 294, 19, 45));
        assert_eq!(
            cs.iter().map(|c| c.number).collect::<Vec<_>>(),
            [29, 30, 31, 1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn a_holiday_in_september_is_red() {
        // Sept 15 is a Monday and the table's 25th row.
        let cs = cells(24, 2);
        let new = cell(&cs, 15);
        assert_eq!((new.weekday, new.colour), (0, RED));
        assert_eq!(cell(&cs, 14).colour, RED);
        assert_eq!(cell(&cs, 13).colour, BLUE);
        // The yellow column has yellow digits and labels.
        let yellow = column_pieces(&[new], true);
        assert_eq!(yellow[0].0 .1, 346);
        assert_eq!(yellow[2].0 .1, 339);
    }

    #[test]
    fn the_column_has_a_cell_a_day() {
        let cs = cells(5, 3);
        assert_eq!(cs.len(), 11);
        assert_eq!(cs[0].y, 10 + 92 * 10);
        assert_eq!(cs[10].y, 10);
        // Index 5 is Aug 25: the new day lies at 562 whatever the days.
        assert_eq!(cell(&cs, 25).y, 562);
        match column_look(&cs, 3, false) {
            Look::Pieces { w, h, .. } => assert_eq!((w, h), (132, 996)),
            _ => panic!(),
        }
    }

    #[test]
    fn the_calendar_takes_its_frames_and_leaves_nothing() {
        let mut st = Stage::default();
        calendar(&mut st, 1, 1, Some("BG020000.akb"), 1.0);
        let wait = st.blocking_ms().unwrap() / TICK_MS;
        // About 280 frames and a few of lag between the states.
        assert!((280..=300).contains(&wait), "{wait}");
        // The curtains are open a little after 32 frames.
        st.advance(33 * TICK_MS);
        let left = st
            .sprites
            .iter()
            .find(|s| matches!(&s.look, Look::Fill { w: 895, .. }))
            .unwrap();
        assert_eq!(left.x, -155.0);
        st.settle();
        assert!(st.sprites.is_empty());
        // At the third effect speed it all takes a few frames.
        let mut st = Stage::default();
        calendar(&mut st, 1, 1, None, 0.0);
        assert!(st.blocking_ms().unwrap() / TICK_MS < 20);
    }
}
