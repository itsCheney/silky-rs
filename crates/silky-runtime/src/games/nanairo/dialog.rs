//! The yes or no box of なないろリンカネーション (`ExitDialog.akb`).
//!
//! A dim layer over the whole screen, a band across the middle with the
//! question on it, the two buttons and the "do not show next time" check
//! box. The areas come from `dialog.area`: 0 is yes, 1 is no and the
//! check box is the third (read here as the shared "do not ask again"
//! id).

use silky_format::area::Area;

use crate::assets::Assets;
use crate::render::Canvas;
use crate::screens::{dialog_ids, Action};
use crate::ui::{Button, Ui};
use crate::Result;

const SHEET: &str = "exitdialog.akb";
const SHADE: [u8; 4] = [0, 0, 0, 127];
/// The band: its rectangle in the sheet and its place on the screen.
const BAND: (u32, u32, u32, u32) = (0, 0, 1280, 140);
const BAND_AT: (i32, i32) = (0, 290);
/// The questions, one under the other from this row of the sheet.
const QUESTION: (u32, u32, u32, u32) = (284, 320, 500, 33);
const QUESTION_AT: (i32, i32) = (389, 310);
/// The buttons: yes then no side by side, one row of the sheet for each
/// state (normal, hover, pressed, disabled).
const BUTTON: (u32, u32, u32, u32) = (0, 320, 142, 52);
/// The check box and its label.
const CHECK: (u32, u32, u32, u32) = (0, 528, 21, 19);
const CHECK_AT: (i32, i32) = (861, 394);
const LABEL: (u32, u32, u32, u32) = (21, 528, 143, 16);
const LABEL_AT: (i32, i32) = (886, 396);

/// Row of the question strips for what "yes" does: load, save, delete,
/// overwrite, delete all, scene list, title, quit.
fn question_row(action: &Action) -> Option<u32> {
    Some(match action {
        Action::Load(_) => 0,
        Action::Save(None) => 1,
        Action::Delete(_) => 2,
        Action::Save(Some(_)) => 3,
        Action::DeleteAll => 4,
        Action::Scene(_) | Action::ReplayReturn => 5,
        Action::Title => 6,
        Action::Quit => 7,
        _ => return None,
    })
}

/// Draws the box over the screen.
pub(crate) fn draw(
    canvas: &mut Canvas,
    assets: &mut Assets,
    ui: &Ui,
    action: &Action,
    dont_ask: bool,
    areas: &[Area],
) -> Result<()> {
    let sheet = assets.image(SHEET)?;
    canvas.fill(0, 0, canvas.width, canvas.height, SHADE);
    canvas.draw_part(&sheet, BAND, BAND_AT.0, BAND_AT.1, false, 255);
    if let Some(row) = question_row(action) {
        let (x, y, w, h) = QUESTION;
        let (qx, qy) = QUESTION_AT;
        canvas.draw_part(&sheet, (x, y + h * row, w, h), qx, qy, false, 255);
    }
    for a in areas {
        let state = ui.state(Button::Dialog(a.id), true, false) as u32;
        match a.id {
            dialog_ids::YES | dialog_ids::NO => {
                let (x, y, w, h) = BUTTON;
                let x = x + w * a.id as u32;
                canvas.draw_part(
                    &sheet,
                    (x, y + h * state.min(3), w, h),
                    a.left,
                    a.top,
                    false,
                    255,
                );
            }
            dialog_ids::DONT_ASK => {
                let (x, y, w, h) = CHECK;
                // Rows: empty, hover, ticked.
                let row = match (dont_ask, state) {
                    (true, _) => 2,
                    (false, 1 | 2) => 1,
                    _ => 0,
                };
                canvas.draw_part(
                    &sheet,
                    (x, y + h * row, w, h),
                    CHECK_AT.0,
                    CHECK_AT.1,
                    false,
                    255,
                );
                canvas.draw_part(&sheet, LABEL, LABEL_AT.0, LABEL_AT.1, false, 255);
            }
            _ => {}
        }
    }
    Ok(())
}
