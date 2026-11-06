//! Engine functions of Silky's engine games.
//!
//! [`Runtime`] is the [`silky_vm::Host`] of a game: it carries out the
//! engine calls of the scripts on a [`Scene`] (background, event pictures,
//! standing characters, message text) and renders that scene in software
//! with [`render`]. It has no window, audio or platform code; frontends
//! (the desktop player, the headless runner, a mobile embedding) drive it
//! and present its frames.
//!
//! Engine calls the runtime does not carry out yet are counted in
//! [`Runtime::unhandled`], never silently treated as done.

pub mod assets;
pub mod audio;
pub mod canvas;
pub mod game_tables;
mod host;
pub mod menus;
pub mod movie;
pub mod render;
pub mod save;
pub mod scene;
pub mod screens;
pub mod session;
pub mod system;
pub mod text;
pub mod ui;

pub use assets::{Assets, Language};
pub use host::{Config, Extras, Pause, Runtime, SaveState, Transition};
pub use scene::Scene;
pub use session::{Session, State};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Format(#[from] silky_format::Error),
    #[error("file not found: {0}")]
    Missing(String),
    #[error("{0}")]
    Invalid(String),
    #[error("font: {0}")]
    Font(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for silky_vm::Error {
    fn from(e: Error) -> Self {
        silky_vm::Error::Host(e.to_string())
    }
}

/// Screen size of the 2026 build.
pub const SCREEN_WIDTH: u32 = 1280;
pub const SCREEN_HEIGHT: u32 = 720;
