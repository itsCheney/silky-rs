//! Trace files: the engine calls a run made, in order, as one JSON
//! document, and a diff that finds where two traces part.
//!
//! A trace comes from this runtime ([`silky_vm::Vm::trace_start`], written
//! by `silky-run`) or from the original program run under a debugger
//! (`tools/trace/`). Both write the same format, so either can be the
//! expected side of a diff. The format is described in `docs/trace.md`.
//!
//! Nothing in the runtime depends on this crate.

pub mod diff;

use std::path::Path;

pub use diff::{diff, Divergence, Fields, Projection, Report, Walk};
pub use silky_vm::trace::{Arg, Call};

/// Value of [`Trace::format`].
pub const FORMAT: &str = "silky-trace";
/// Version of the format written. A file of another version is refused.
pub const VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("not a trace: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not a trace (format {0:?})")]
    Format(String),
    #[error("trace version {0} is not supported (this build reads {VERSION})")]
    Version(u32),
}

/// Who made a trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Producer {
    /// `runtime` (this project) or `original` (the game's own program).
    pub kind: String,
    /// The tool that wrote it, as `silky-run snap`.
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Trace {
    pub format: String,
    pub version: u32,
    pub producer: Producer,
    /// The game, as the runtime names it (`Doukyuusei`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    /// How the run was made: options, inputs, anything needed to make it
    /// again. Free text, not compared.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub calls: Vec<Call>,
}

impl Trace {
    pub fn new(producer: Producer) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            producer,
            game: None,
            notes: Vec::new(),
            calls: Vec::new(),
        }
    }

    /// A trace written by this runtime's tool `name`.
    pub fn runtime(name: &str) -> Self {
        Self::new(Producer {
            kind: "runtime".into(),
            name: name.into(),
            version: env!("CARGO_PKG_VERSION").into(),
        })
    }

    pub fn parse(text: &str) -> Result<Self, Error> {
        // Check the format before the whole document, so another JSON
        // file is reported as such.
        #[derive(serde::Deserialize)]
        struct Head {
            format: Option<String>,
            version: Option<u32>,
        }
        let head: Head = serde_json::from_str(text)?;
        let format = head.format.unwrap_or_default();
        if format != FORMAT {
            return Err(Error::Format(format));
        }
        match head.version {
            Some(VERSION) => Ok(serde_json::from_str(text)?),
            v => Err(Error::Version(v.unwrap_or(0))),
        }
    }

    pub fn read(path: &Path) -> Result<Self, Error> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// The document as text: the header, then one call per line, so a
    /// text diff or `grep` works on it too.
    pub fn to_text(&self) -> String {
        let mut out = String::from("{\n");
        let field = |out: &mut String, name: &str, value: String| {
            out.push_str(&format!("  {}: {},\n", json(&name), value));
        };
        field(&mut out, "format", json(&self.format));
        field(&mut out, "version", self.version.to_string());
        field(&mut out, "producer", json(&self.producer));
        if let Some(game) = &self.game {
            field(&mut out, "game", json(game));
        }
        if !self.notes.is_empty() {
            field(&mut out, "notes", json(&self.notes));
        }
        out.push_str("  \"calls\": [");
        for (i, call) in self.calls.iter().enumerate() {
            out.push_str(if i == 0 { "\n    " } else { ",\n    " });
            out.push_str(&json(call));
        }
        out.push_str(if self.calls.is_empty() {
            "]\n}\n"
        } else {
            "\n  ]\n}\n"
        });
        out
    }

    pub fn write(&self, path: &Path) -> Result<(), Error> {
        Ok(std::fs::write(path, self.to_text())?)
    }
}

fn json<T: serde::Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).expect("trace values serialize")
}

/// One call on one line, for reports: `T.MES:1a2b Game.24("x") = 7`.
pub fn show(call: &Call) -> String {
    let mut s = String::new();
    if let Some(tick) = call.tick {
        s.push_str(&format!("@{tick} "));
    }
    match (&call.script, call.pc) {
        (Some(script), Some(pc)) => s.push_str(&format!("{script}:{pc:x} ")),
        (Some(script), None) => s.push_str(&format!("{script} ")),
        (None, Some(pc)) => s.push_str(&format!(":{pc:x} ")),
        (None, None) => {}
    }
    s.push_str(&call.call);
    if !call.args.is_empty() {
        let args: Vec<String> = call.args.iter().map(Arg::to_string).collect();
        s.push_str(&format!("({})", args.join(", ")));
    }
    if let Some(r) = &call.result {
        s.push_str(&format!(" = {r}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, args: Vec<Arg>) -> Call {
        Call {
            tick: Some(3),
            script: Some("A.MES".into()),
            pc: Some(0x1a),
            call: name.into(),
            args,
            result: None,
        }
    }

    #[test]
    fn round_trip_one_call_per_line() {
        let mut t = Trace::runtime("silky-run snap");
        t.game = Some("Doukyuusei".into());
        t.notes.push("--path 0,1".into());
        t.calls.push(call(
            "Game.24",
            vec![Arg::Str("a\n\"b\"".into()), Arg::Int(-2)],
        ));
        t.calls.push(Call {
            result: Some(Arg::Int(1)),
            ..call("message", vec![Arg::Int(5)])
        });
        let text = t.to_text();
        assert_eq!(text.lines().count(), 11);
        assert!(text.contains(
            "\n    {\"tick\":3,\"script\":\"A.MES\",\"pc\":26,\"call\":\"Game.24\",\"args\":[\"a\\n\\\"b\\\"\",-2]},\n"
        ));
        assert_eq!(Trace::parse(&text).unwrap(), t);
        assert_eq!(show(&t.calls[1]), "@3 A.MES:1a message(5) = 1");

        let empty = Trace::runtime("x");
        assert_eq!(Trace::parse(&empty.to_text()).unwrap(), empty);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(matches!(Trace::parse("{\"a\":1}"), Err(Error::Format(_))));
        let text = Trace::runtime("x")
            .to_text()
            .replace("\"version\": 1", "\"version\": 2");
        assert!(matches!(Trace::parse(&text), Err(Error::Version(2))));
        assert!(matches!(Trace::parse("[1,"), Err(Error::Json(_))));
    }
}
