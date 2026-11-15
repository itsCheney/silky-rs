//! The αMAIL screen of 景の海のアペイリア (`Game.1012`), read from the
//! executable: a glass panel (`SLD.akb`) with the mail list
//! (`mail01_p.akb`), a "new mail" notice and an open mail
//! (`mail02_p.akb`). The opening and closing animations are not done;
//! the parts appear at once.

use std::sync::Arc;

use silky_format::akb::Image;

use crate::assets::Assets;
use crate::canvas::Canvas;
use crate::text::TextRenderer;

/// State of the screen, kept with the scene so saves keep it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Mail {
    /// 0 inbox, 1 sent tray.
    pub tray: i32,
    /// Number of the newest mail, from 1 (byte 1200 or 1400).
    pub newest: u8,
    /// The panel is up (sub 1 to sub 2).
    pub shown: bool,
    /// "n new mails received" notice (sub 5 to sub 6).
    pub notice: Option<i32>,
    /// Row picked in the list (sub 8 to sub 9).
    pub selected: Option<i32>,
    /// Text of the open mail, lines split (sub 3 to sub 4).
    pub body: Option<String>,
}

/// Glass panel: the top 958 x 518 of `SLD.akb`.
const PANEL: (i32, i32) = (155, 0);
const PANEL_SIZE: (u32, u32) = (958, 518);
/// The list: a 375 x 468 window of `mail01_p.akb` from (0, 142).
const LIST: (i32, i32) = (PANEL.0 + 299, PANEL.1 + 50);
const LIST_SHEET: (u32, u32, u32, u32) = (0, 142, 375, 468);
/// Tray name, 120 x 24, from (0, 662) (inbox) or (0, 686) (sent).
const TRAY: (i32, i32) = (113, 48);
/// Newest mail row: 346 x 70 cells, 13 a column, in 692-pixel columns
/// from x = 1067 (the picked look 346 pixels to the right).
const ROW: (i32, i32) = (6, 83);
const ROW_SIZE: (u32, u32) = (346, 70);
/// Older rows: 346 x 210 from (375, 30), then its first 48 rows again
/// below.
const OLDER: (i32, i32) = (6, 153);
const OLDER_SHEET: (u32, u32, u32, u32) = (375, 30, 346, 210);
const OLDER_MORE: (i32, i32) = (6, 363);
const OLDER_MORE_SHEET: (u32, u32, u32, u32) = (375, 30, 346, 48);
/// Scroll bar, 12 x 179 from (375, 382).
const SCROLL: (i32, i32) = (361, 114);
const SCROLL_SHEET: (u32, u32, u32, u32) = (375, 382, 12, 179);
/// Notice: 309 x 142 from (0, 0); the count is a 33 x 41 digit from
/// (375 + 33 n, 341), the wing a 63 x 52 picture from (309, 0).
const NOTICE: (i32, i32) = (491, 184);
const NOTICE_SHEET: (u32, u32, u32, u32) = (0, 0, 309, 142);
const DIGIT: (i32, i32) = (139, 27);
const WING: (i32, i32, u32, u32, u32, u32) = (203, 29, 309, 0, 63, 52);
/// Open mail: 461 x 471 from the top left of `mail02_p.akb`; the text,
/// 12-pixel characters on a 16-pixel line, was placed as measured under
/// Wine.
const BODY: (i32, i32) = (578, 31);
const BODY_SHEET: (u32, u32, u32, u32) = (0, 0, 461, 471);
const BODY_TEXT: (f32, f32) = (598.0, 127.0);
const BODY_SIZE: f32 = 12.0;
const BODY_LINE: f32 = 16.0;
/// Lines longer than this are wrapped (the panel's inner width; not
/// measured yet).
const BODY_WIDTH: f32 = 420.0;

impl Mail {
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        assets: &mut Assets,
        text: Option<&TextRenderer>,
    ) -> crate::Result<()> {
        if !self.shown {
            return Ok(());
        }
        let list = assets.image("mail01_p.akb")?;
        let glass = assets.image("sld.akb")?;
        let (pw, ph) = PANEL_SIZE;
        canvas.draw_part(&glass, (0, 0, pw, ph), PANEL.0, PANEL.1, false, 255);
        let at = |p: (i32, i32)| (LIST.0 + p.0, LIST.1 + p.1);
        part(canvas, &list, LIST_SHEET, LIST);
        let tray_row = if self.tray == 0 { 662 } else { 686 };
        part(canvas, &list, (0, tray_row, 120, 24), at(TRAY));
        if self.newest > 0 {
            let i = self.newest as u32 - 1;
            let picked = if self.selected == Some(0) {
                ROW_SIZE.0
            } else {
                0
            };
            let x = 1067 + 692 * (i / 13) + picked;
            let y = ROW_SIZE.1 * (i % 13);
            part(canvas, &list, (x, y, ROW_SIZE.0, ROW_SIZE.1), at(ROW));
        }
        part(canvas, &list, OLDER_SHEET, at(OLDER));
        part(canvas, &list, OLDER_MORE_SHEET, at(OLDER_MORE));
        part(canvas, &list, SCROLL_SHEET, at(SCROLL));
        if let Some(n) = self.notice {
            part(canvas, &list, NOTICE_SHEET, NOTICE);
            let n = n.clamp(0, 9) as u32;
            let d = (NOTICE.0 + DIGIT.0, NOTICE.1 + DIGIT.1);
            part(canvas, &list, (375 + 33 * n, 341, 33, 41), d);
            let (wx, wy, sx, sy, w, h) = WING;
            part(
                canvas,
                &list,
                (sx, sy, w, h),
                (NOTICE.0 + wx, NOTICE.1 + wy),
            );
        }
        if let Some(body) = &self.body {
            let sheet = assets.image("mail02_p.akb")?;
            part(canvas, &sheet, BODY_SHEET, BODY);
            if let Some(tr) = text {
                tr.with_wide_advance(1.0, |tr| draw_body(canvas, tr, body));
            }
        }
        Ok(())
    }
}

/// The open mail's text, its lines wrapped to the panel.
fn draw_body(canvas: &mut Canvas, tr: &TextRenderer, body: &str) {
    let (x, y) = BODY_TEXT;
    let lines = body.lines().flat_map(|l| {
        let wrapped = tr.wrap(l, BODY_SIZE, BODY_WIDTH);
        if wrapped.is_empty() {
            vec![String::new()]
        } else {
            wrapped
        }
    });
    for (k, line) in lines.enumerate() {
        let y = y + BODY_LINE * k as f32;
        tr.draw_plain(canvas, &line, x, y, BODY_SIZE, [40, 40, 40, 255]);
    }
}

fn part(canvas: &mut Canvas, img: &Arc<Image>, part: (u32, u32, u32, u32), at: (i32, i32)) {
    canvas.draw_part(img, part, at.0, at.1, false, 255);
}
