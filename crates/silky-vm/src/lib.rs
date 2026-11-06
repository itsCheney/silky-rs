//! Interpreter for the byte code of Silky's engine scripts.
//!
//! The virtual machine runs `.MES` scripts and the `LIBLARY.LIB` function
//! library. It owns the script state (value stack, variable banks, function
//! table, return contexts, selection options) and nothing else: every engine
//! function ("native", opcode 0x18) is handed to a [`Host`], which pops its
//! own arguments from the stack. This keeps the crate free of graphics,
//! audio and platform code.
//!
//! See [`silky_format::mes`] for the instruction set.

pub mod native;
mod value;
mod vm;

pub use value::Value;
pub use vm::{Error, Flow, Function, Host, Result, Snapshot, Stop, Vm};
