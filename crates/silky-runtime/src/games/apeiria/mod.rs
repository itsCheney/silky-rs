//! 景の海のアペイリア (Japanese build with the official update).
//!
//! See `docs/games/apeiria.md`.

pub mod anim;
mod casablanca;
pub mod characters;
pub(crate) mod commands;
pub mod effects;
pub mod holo;
pub mod keys;
pub mod mail;
pub mod mail_select;
pub mod omake;
pub mod panels;
pub mod vo;

/// Variable H 69: 1 while the scene list replays a scene.
pub const REPLAY_VAR: usize = 69;
