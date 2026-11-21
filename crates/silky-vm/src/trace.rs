//! Calls recorded for a trace: what the scripts asked of the engine, in
//! order. A host turns recording on with [`crate::Vm::trace_start`] and
//! takes the calls with [`crate::Vm::trace_take`]. The `silky-trace` crate
//! writes them to a file and compares two files (this runtime against the
//! original program, or two versions of this runtime).

use crate::Value;

/// One call. `call` names it:
///
/// - `Class.sub` for an engine function (opcode 0x18), as `Sprite.1`; for
///   class 29 the number is the `Game` command, as `Game.24`. A class
///   without a name is written by number, as `42.3`;
/// - `message` and `smessage` for a story or selection message
///   (`args` = its number);
/// - `options` when a list of selection options is complete
///   (`args` = their values);
/// - `choose` when the host runs an option body (`args` = its index).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Call {
    /// Frame count the host set with [`crate::Vm::set_trace_tick`]; none
    /// when the host does not count frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick: Option<u64>,
    /// Script running the call. The original program's side may not know.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Offset of the instruction in that script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pc: Option<u32>,
    pub call: String,
    /// The statement's values, oldest first, without the sub-command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Arg>,
    /// What the host left on the stack for the script, if anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Arg>,
}

/// A script value as it appears in a trace: a number or a string.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Arg {
    Int(i32),
    Str(String),
}

impl From<&Value> for Arg {
    fn from(v: &Value) -> Self {
        match v {
            Value::Int(i) => Arg::Int(*i),
            Value::Str(s) => Arg::Str(s.clone()),
        }
    }
}

impl std::fmt::Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arg::Int(v) => write!(f, "{v}"),
            Arg::Str(s) => write!(f, "{s:?}"),
        }
    }
}

/// Name of an engine call: `Class.sub`.
pub fn call_name(class: i32, sub: Option<i32>) -> String {
    let class = match crate::native::class_name(class) {
        Some(name) => name.to_string(),
        None => class.to_string(),
    };
    match sub {
        Some(sub) => format!("{class}.{sub}"),
        None => class,
    }
}

/// Recording state kept by the interpreter.
#[derive(Debug, Default)]
pub(crate) struct Recorder {
    pub calls: Vec<Call>,
    pub tick: Option<u64>,
}
