//! なないろリンカネーション.
//!
//! See `docs/games/nanairo.md`.

mod animator;
mod calendar;
pub(crate) mod cg;
mod cg_table;
pub(crate) mod commands;
pub(crate) mod effects;
mod ending;
pub(crate) mod stage;
pub(crate) mod window;

/// Sprite of the sheet the calendar and the ending overlay draw from.
pub const SHEET_STORE: i32 = 3;
/// Sprite of the backgrounds and pictures, under the standing characters.
pub const BACKGROUND_SPRITE: i32 = 2;
