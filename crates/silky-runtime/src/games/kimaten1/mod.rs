//! きまぐれテンプテーション: what its interface needs beyond the game data.
//!
//! The first game of the pair. Its companion is the same kind of E-mote
//! model as in the sequel and takes the same calls, so those go through
//! [`super::kimaten2`]; its screens are its own.
//!
//! See `docs/games/kimaten1.md`.

mod actsel;
mod areasel;
mod chrome;
pub(crate) mod commands;
mod itemsel;
pub mod map;
pub mod notebook;
pub mod notice;
pub mod screen;
pub mod window;

pub use commands::SHARED_COMMANDS;
pub use notice::Notices;
pub use screen::{Click, Look, Screen};
