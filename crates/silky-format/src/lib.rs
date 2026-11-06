//! File formats used by the Silky's engine ("Silky Engine", 2014 and later).
//!
//! This crate only parses and decodes data. It does no I/O beyond reading
//! archive files and has no platform code, so every host (desktop player,
//! headless runner, mobile embedding) can share it.

pub mod akb;
pub mod arc;
pub mod area;
pub mod error;
pub mod game;
pub mod lzss;
pub mod map;
pub mod mes;
pub mod vsd;

pub use error::{Error, Result};

/// Decodes a Shift-JIS (CP932) byte string, replacing invalid sequences.
pub fn decode_cp932(bytes: &[u8]) -> String {
    let (text, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
    text.into_owned()
}
