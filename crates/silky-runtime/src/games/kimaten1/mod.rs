//! きまぐれテンプテーション: what its interface needs beyond the game data.
//!
//! The first game of the pair. Its E-mote models are the sequel's kind and
//! mostly take the same calls (see [`emote`]); its screens are its own.
//!
//! See `docs/games/kimaten1.md`.

mod actsel;
mod areasel;
mod cgdata;
mod chrome;
pub(crate) mod commands;
pub mod effects;
mod emote;
mod exselect;
mod itemsel;
pub mod map;
pub mod notebook;
pub mod notice;
pub mod omake;
pub mod screen;
pub mod staff;
pub mod title;
pub mod viewer;
pub mod vo;
pub mod window;

pub use commands::SHARED_COMMANDS;
pub use notice::Notices;
pub use screen::{Click, Look, Screen};
