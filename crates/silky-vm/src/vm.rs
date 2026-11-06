use std::collections::HashMap;
use std::sync::Arc;

use silky_format::mes::{self, Operand, Profile, Script};

use crate::value::Value;

/// Variable index of the current message number (set by `message`).
pub const VAR_MESSAGE: usize = 52;
/// Variable index of the current selection message number (`smessage`).
pub const VAR_SELECT_MESSAGE: usize = 54;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Format(#[from] silky_format::Error),
    #[error("stack underflow")]
    StackUnderflow,
    #[error("expected {expected}, found {found}")]
    Type {
        expected: &'static str,
        found: Value,
    },
    #[error("bad variable index {0}")]
    BadIndex(i32),
    #[error("division by zero")]
    DivideByZero,
    #[error("call of undefined function {0}")]
    UnknownFunction(i32),
    #[error("opcode {0:#04x} is not supported")]
    Unsupported(u8),
    #[error("no option {0}")]
    NoOption(usize),
    #[error("no script is running")]
    NoScript,
    #[error("{0}")]
    Host(String),
    #[error("{script} at {offset:#x}: {source}")]
    At {
        script: String,
        offset: u32,
        source: Box<Error>,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/// What a host callback wants the interpreter to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    /// Return from [`Vm::run`] with [`Stop::Yield`], e.g. to wait for input.
    Yield,
}

/// Why [`Vm::run`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// A host callback returned [`Flow::Yield`].
    Yield,
    /// A list of selection options is complete; call [`Vm::choose`].
    Options,
    /// `ret` with nothing to return to: the script ended.
    Ended,
    /// The step budget ran out.
    Budget,
}

/// The engine side of the interpreter.
pub trait Host {
    /// Engine function class `class` (opcode 0x18). The class pops its
    /// sub-command and arguments, and pushes a result if it has one.
    fn native(&mut self, vm: &mut Vm, class: i32) -> Result<Flow>;

    /// `message n`: a story message starts. The interpreter has already set
    /// value variable [`VAR_MESSAGE`].
    fn message(&mut self, _vm: &mut Vm, _number: u32) -> Result<Flow> {
        Ok(Flow::Continue)
    }

    /// `smessage n`: a selection message starts ([`VAR_SELECT_MESSAGE`]).
    fn select_message(&mut self, _vm: &mut Vm, _number: u32) -> Result<Flow> {
        Ok(Flow::Continue)
    }
}

/// A script loaded into the interpreter.
pub struct Loaded {
    pub name: String,
    pub script: Script,
}

/// A function defined by `fdef`.
#[derive(Clone)]
pub struct Function {
    pub script: Arc<Loaded>,
    pub entry: u32,
    pub argc: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum ContextKind {
    /// Saved by `call`; `fret` also drops the locals frame.
    Function,
    /// Saved by [`Vm::call_script`].
    Script,
    /// Saved by [`Vm::choose`] before running an option body.
    Option,
}

#[derive(Clone)]
struct Context {
    script: Arc<Loaded>,
    pc: u32,
    kind: ContextKind,
}

/// Source position from the debug opcodes `fe` / `ff`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourcePos {
    /// Macro being expanded and its file, if inside a macro.
    pub call: Option<(String, String, i32)>,
    /// Line of the script source.
    pub line: i32,
}

/// Interpreter state as plain data, for save files. Scripts are named and
/// loaded again on restore.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    current: Option<String>,
    pc: u32,
    stack: Vec<Value>,
    statement_base: usize,
    bytes: Vec<u8>,
    words: Vec<u16>,
    raw: Vec<u8>,
    g: Vec<Value>,
    h: Vec<Value>,
    locals: Vec<Vec<Value>>,
    /// (number, script, entry, argument count)
    functions: Vec<(i32, String, u32, u32)>,
    /// (script, pc, kind)
    contexts: Vec<(String, u32, ContextKind)>,
    options: Vec<(u32, Value)>,
    rng: u32,
    source: SourcePos,
}

pub struct Vm {
    profile: Profile,
    current: Option<Arc<Loaded>>,
    pc: u32,
    stack: Vec<Value>,
    /// Stack depth at the last statement boundary (debug opcode).
    statement_base: usize,
    pub bytes: Vec<u8>,
    pub words: Vec<u16>,
    /// Raw area, read as 8, 16 or 32 bit elements.
    pub raw: Vec<u8>,
    /// Value bank G (opcodes 04 / 0e).
    pub g: Vec<Value>,
    /// Value bank H (opcodes 05 / 0f).
    pub h: Vec<Value>,
    locals: Vec<Vec<Value>>,
    functions: HashMap<i32, Function>,
    contexts: Vec<Context>,
    options: Vec<(u32, Value)>,
    rng: u32,
    pub source: SourcePos,
    /// Instructions executed so far.
    pub steps: u64,
    /// Set by `text 0`: the next string continues the one on the stack.
    line_break: bool,
    /// Strings the option bodies left (Japanese builds put each label
    /// there, as `option @x; push.cs "label"; ret`), in option order.
    pub option_labels: Vec<String>,
}

impl Vm {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            current: None,
            pc: 0,
            stack: Vec::new(),
            statement_base: 0,
            bytes: Vec::new(),
            words: Vec::new(),
            raw: Vec::new(),
            g: Vec::new(),
            h: Vec::new(),
            locals: Vec::new(),
            functions: HashMap::new(),
            contexts: Vec::new(),
            options: Vec::new(),
            rng: 0x2545_f491,
            source: SourcePos::default(),
            steps: 0,
            line_break: false,
            option_labels: Vec::new(),
        }
    }

    // ---- scripts and control flow -------------------------------------

    /// Name of the running script.
    pub fn script_name(&self) -> Option<&str> {
        self.current.as_deref().map(|l| l.name.as_str())
    }

    /// Offset of the next instruction in the running script.
    pub fn pc(&self) -> u32 {
        self.pc
    }

    /// Replaces the running script (or starts the first one).
    pub fn jump_script(&mut self, name: &str, script: Script) {
        self.current = Some(Arc::new(Loaded {
            name: name.to_string(),
            script,
        }));
        self.pc = 0;
        self.options.clear();
    }

    /// Runs `script` from its start; its `ret` comes back here.
    pub fn call_script(&mut self, name: &str, script: Script) {
        if let Some(cur) = self.current.clone() {
            self.contexts.push(Context {
                script: cur,
                pc: self.pc,
                kind: ContextKind::Script,
            });
        }
        self.jump_script(name, script);
    }

    /// Functions defined so far by `fdef`.
    pub fn functions(&self) -> &HashMap<i32, Function> {
        &self.functions
    }

    /// Selection options recorded since the last choice: (body offset, value).
    pub fn options(&self) -> &[(u32, Value)] {
        &self.options
    }

    /// The interpreter state, to be stored in a save file.
    pub fn snapshot(&self) -> Snapshot {
        let mut functions: Vec<_> = self
            .functions
            .iter()
            .map(|(&n, f)| (n, f.script.name.clone(), f.entry, f.argc))
            .collect();
        functions.sort_by_key(|f| f.0);
        Snapshot {
            current: self.current.as_ref().map(|c| c.name.clone()),
            pc: self.pc,
            stack: self.stack.clone(),
            statement_base: self.statement_base,
            bytes: self.bytes.clone(),
            words: self.words.clone(),
            raw: self.raw.clone(),
            g: self.g.clone(),
            h: self.h.clone(),
            locals: self.locals.clone(),
            functions,
            contexts: self
                .contexts
                .iter()
                .map(|c| (c.script.name.clone(), c.pc, c.kind))
                .collect(),
            options: self.options.clone(),
            rng: self.rng,
            source: self.source.clone(),
        }
    }

    /// Puts back a state from [`Vm::snapshot`]; `load` reads a script by
    /// name. Each script is loaded once.
    pub fn restore_snapshot(
        &mut self,
        snap: Snapshot,
        load: &mut dyn FnMut(&str) -> Result<Script>,
    ) -> Result<()> {
        let mut cache: HashMap<String, Arc<Loaded>> = HashMap::new();
        let mut get = |name: &str| -> Result<Arc<Loaded>> {
            if let Some(l) = cache.get(name) {
                return Ok(l.clone());
            }
            let l = Arc::new(Loaded {
                name: name.to_string(),
                script: load(name)?,
            });
            cache.insert(name.to_string(), l.clone());
            Ok(l)
        };
        let current = snap.current.as_deref().map(&mut get).transpose()?;
        let mut functions = HashMap::new();
        for (n, script, entry, argc) in snap.functions {
            functions.insert(
                n,
                Function {
                    script: get(&script)?,
                    entry,
                    argc,
                },
            );
        }
        let mut contexts = Vec::new();
        for (script, pc, kind) in snap.contexts {
            contexts.push(Context {
                script: get(&script)?,
                pc,
                kind,
            });
        }
        self.current = current;
        self.pc = snap.pc;
        self.stack = snap.stack;
        self.statement_base = snap.statement_base;
        self.bytes = snap.bytes;
        self.words = snap.words;
        self.raw = snap.raw;
        self.g = snap.g;
        self.h = snap.h;
        self.locals = snap.locals;
        self.functions = functions;
        self.contexts = contexts;
        self.options = snap.options;
        self.rng = snap.rng;
        self.source = snap.source;
        Ok(())
    }

    /// Runs every option body in list order, then goes on after the list.
    /// In the 2014+ engine the bodies only register their labels (for
    /// example `H[40 + i] = text line`); the selection itself is an engine
    /// call that follows the list.
    pub fn run_all_options(&mut self) -> Result<()> {
        let entries: Vec<u32> = self.options.iter().map(|o| o.0).collect();
        let Some((&first, rest)) = entries.split_first() else {
            return Err(Error::NoOption(0));
        };
        let cur = self.current.clone().ok_or(Error::NoScript)?;
        let after = self.pc;
        for pc in std::iter::once(after).chain(rest.iter().rev().copied()) {
            self.contexts.push(Context {
                script: cur.clone(),
                pc,
                kind: ContextKind::Option,
            });
        }
        self.pc = first;
        self.options.clear();
        self.option_labels.clear();
        Ok(())
    }

    /// Runs the body of option `index`; its `ret` comes back to the
    /// instruction after the option list.
    pub fn choose(&mut self, index: usize) -> Result<()> {
        let Some(&(entry, _)) = self.options.get(index) else {
            return Err(Error::NoOption(index));
        };
        let cur = self.current.clone().ok_or(Error::NoScript)?;
        self.contexts.push(Context {
            script: cur,
            pc: self.pc,
            kind: ContextKind::Option,
        });
        self.pc = entry;
        self.options.clear();
        Ok(())
    }

    /// Drops the recorded options without running one.
    pub fn discard_options(&mut self) {
        self.options.clear();
    }

    // ---- stack access for hosts -----------------------------------------

    pub fn push(&mut self, v: impl Into<Value>) {
        self.stack.push(v.into());
    }

    pub fn pop(&mut self) -> Result<Value> {
        self.stack.pop().ok_or(Error::StackUnderflow)
    }

    pub fn pop_int(&mut self) -> Result<i32> {
        match self.pop()? {
            Value::Int(v) => Ok(v),
            found => Err(Error::Type {
                expected: "integer",
                found,
            }),
        }
    }

    pub fn pop_str(&mut self) -> Result<String> {
        match self.pop()? {
            Value::Str(s) => Ok(s),
            found => Err(Error::Type {
                expected: "string",
                found,
            }),
        }
    }

    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    /// Pops every value pushed since the current statement began, oldest
    /// first. For hosts that do not know a native's argument count yet.
    pub fn pop_statement(&mut self) -> Vec<Value> {
        let base = self.statement_base.min(self.stack.len());
        self.stack.split_off(base)
    }

    // ---- variables ------------------------------------------------------

    pub fn var_g(&self, index: usize) -> Value {
        self.g.get(index).cloned().unwrap_or_default()
    }

    pub fn set_g(&mut self, index: usize, v: impl Into<Value>) {
        set(&mut self.g, index, v.into());
    }

    pub fn var_h(&self, index: usize) -> Value {
        self.h.get(index).cloned().unwrap_or_default()
    }

    pub fn set_h(&mut self, index: usize, v: impl Into<Value>) {
        set(&mut self.h, index, v.into());
    }

    /// Byte variable (`ld.b`).
    pub fn var_b(&self, index: usize) -> u8 {
        self.bytes.get(index).copied().unwrap_or(0)
    }

    /// Sets a byte variable, as a host screen does that writes game state
    /// (the costume screen of きまぐれテンプテーション2).
    pub fn set_var_b(&mut self, index: usize, value: u8) {
        set(&mut self.bytes, index, value);
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.rng = seed.max(1);
    }

    // ---- interpreter ----------------------------------------------------

    /// Runs until a host callback yields, options are ready, the script
    /// ends, or `budget` instructions have run.
    pub fn run(&mut self, host: &mut dyn Host, budget: u64) -> Result<Stop> {
        for _ in 0..budget {
            let offset = self.pc;
            match self.step(host) {
                Ok(None) => {}
                Ok(Some(stop)) => return Ok(stop),
                Err(e) => {
                    return Err(Error::At {
                        script: self.script_name().unwrap_or("?").to_string(),
                        offset,
                        source: Box::new(e),
                    })
                }
            }
        }
        Ok(Stop::Budget)
    }

    fn step(&mut self, host: &mut dyn Host) -> Result<Option<Stop>> {
        let cur = self.current.clone().ok_or(Error::NoScript)?;
        let (ins, next) = mes::decode_at(&cur.script.code, self.pc, self.profile)?;
        let op = ins.opcode;
        self.steps += 1;
        self.pc = next;
        match op {
            0x00 => return Ok(self.restore(false)),
            0x01 => return Ok(self.restore(true)),
            0x02..=0x09 => {
                let i = self.pop_index()?;
                let v = self.load(op - 0x02, i)?;
                self.stack.push(v);
            }
            0x0a | 0x0b | 0x33 => {
                let Operand::Str(b) = &ins.operand else {
                    unreachable!()
                };
                let text = cur.script.decode(b);
                // After `text 0`, the line goes on the string before it.
                match (self.line_break, self.stack.last_mut()) {
                    (true, Some(Value::Str(top))) => {
                        top.push('\n');
                        top.push_str(&text);
                    }
                    _ => self.stack.push(Value::Str(text)),
                }
                self.line_break = false;
            }
            0x0c..=0x13 => {
                let i = self.pop_index()?;
                let v = self.pop()?;
                self.store(op - 0x0c, i, v)?;
            }
            0x14 => {
                if !self.pop()?.is_true() {
                    self.pc = target(&ins);
                }
            }
            0x15 => self.pc = target(&ins),
            0x16 => {
                let id = self.pop_int()?;
                let argc = self.pop_int()?;
                self.functions.insert(
                    id,
                    Function {
                        script: cur.clone(),
                        entry: next,
                        argc: argc.max(0) as u32,
                    },
                );
                self.pc = target(&ins);
            }
            0x17 => {
                let id = self.pop_int()?;
                let f = self
                    .functions
                    .get(&id)
                    .cloned()
                    .ok_or(Error::UnknownFunction(id))?;
                let mut frame = Vec::with_capacity(f.argc as usize);
                for _ in 0..f.argc {
                    frame.push(self.pop()?);
                }
                self.locals.push(frame);
                self.contexts.push(Context {
                    script: cur,
                    pc: next,
                    kind: ContextKind::Function,
                });
                self.current = Some(f.script.clone());
                self.pc = f.entry;
            }
            0x18 => {
                let class = self.pop_int()?;
                if host.native(self, class)? == Flow::Yield {
                    return Ok(Some(Stop::Yield));
                }
            }
            0x19 | 0x1a => {
                let Operand::Message(n) = ins.operand else {
                    unreachable!()
                };
                let flow = if op == 0x19 {
                    self.set_g(VAR_MESSAGE, n as i32);
                    host.message(self, n)?
                } else {
                    self.set_g(VAR_SELECT_MESSAGE, n as i32);
                    host.select_message(self, n)?
                };
                if flow == Flow::Yield {
                    return Ok(Some(Stop::Yield));
                }
            }
            0x1b => {
                let v = self.pop()?;
                self.options.push((next, v));
                self.pc = target(&ins);
                if !self.option_follows(&cur) {
                    return Ok(Some(Stop::Options));
                }
            }
            // Message text control. Sub 0 breaks the line: the engine adds
            // a newline to the message being built from the strings.
            0x1c => match ins.operand {
                Operand::U8(0) => self.line_break = true,
                _ => return Err(Error::Unsupported(op)),
            },
            0x32 => {
                let Operand::Int(v) = ins.operand else {
                    unreachable!()
                };
                self.stack.push(Value::Int(v));
            }
            0x34..=0x38 | 0x3a..=0x43 => {
                let b = self.pop()?;
                let a = self.pop()?;
                let v = binary(op, a, b)?;
                self.stack.push(v);
            }
            0x39 => {
                let n = self.pop_int()?;
                let r = self.next_random();
                self.stack
                    .push(Value::Int(if n > 0 { (r % n as u32) as i32 } else { 0 }));
            }
            0xfa..=0xfd => {}
            0xfe => {
                let line = self.pop_int()?;
                let module = self.pop()?;
                let name = self.pop()?;
                self.source.call = Some((
                    name.as_str().unwrap_or_default().to_string(),
                    module.as_str().unwrap_or_default().to_string(),
                    line,
                ));
                self.statement_base = self.stack.len();
            }
            0xff => {
                self.source.line = self.pop_int()?;
                self.source.call = None;
                self.statement_base = self.stack.len();
            }
            _ => return Err(Error::Unsupported(op)),
        }
        Ok(None)
    }

    /// `ret` / `fret`: back to the last saved context.
    fn restore(&mut self, function: bool) -> Option<Stop> {
        if function {
            self.locals.pop();
        }
        if self
            .contexts
            .last()
            .is_some_and(|c| c.kind == ContextKind::Option)
        {
            if let Some(Value::Str(label)) = self.stack.last() {
                self.option_labels.push(label.clone());
            }
        }
        self.stack.clear();
        self.statement_base = 0;
        self.options.clear();
        match self.contexts.pop() {
            Some(ctx) => {
                if ctx.kind == ContextKind::Function && !function {
                    // A plain `ret` inside a function body still leaves it.
                    self.locals.pop();
                }
                self.current = Some(ctx.script);
                self.pc = ctx.pc;
                None
            }
            None => {
                self.current = None;
                Some(Stop::Ended)
            }
        }
    }

    /// Whether another `option` follows at `pc`, past the pushes and debug
    /// opcodes that lead up to it. If not, the option list is complete.
    fn option_follows(&self, cur: &Loaded) -> bool {
        let mut pc = self.pc;
        loop {
            match mes::decode_at(&cur.script.code, pc, self.profile) {
                Ok((ins, next)) => match ins.opcode {
                    0x1b => return true,
                    0x0a | 0x0b | 0x32 | 0x33 | 0xfa..=0xff => pc = next,
                    _ => return false,
                },
                Err(_) => return false,
            }
        }
    }

    fn pop_index(&mut self) -> Result<usize> {
        let i = self.pop_int()?;
        usize::try_from(i).map_err(|_| Error::BadIndex(i))
    }

    fn load(&self, bank: u8, i: usize) -> Result<Value> {
        Ok(match bank {
            0 => Value::Int(self.bytes.get(i).copied().unwrap_or(0) as i32),
            1 => Value::Int(self.words.get(i).copied().unwrap_or(0) as i32),
            2 => self.var_g(i),
            3 => self.var_h(i),
            4 => self
                .locals
                .last()
                .and_then(|f| f.get(i).cloned())
                .unwrap_or_default(),
            5 => Value::Int(self.raw.get(i).copied().unwrap_or(0) as i8 as i32),
            6 => {
                let mut b = [0u8; 2];
                read_raw(&self.raw, 2 * i, &mut b);
                Value::Int(i16::from_le_bytes(b) as i32)
            }
            _ => {
                let mut b = [0u8; 4];
                read_raw(&self.raw, 4 * i, &mut b);
                Value::Int(i32::from_le_bytes(b))
            }
        })
    }

    fn store(&mut self, bank: u8, i: usize, v: Value) -> Result<()> {
        match bank {
            0 => set(&mut self.bytes, i, v.int() as u8),
            1 => set(&mut self.words, i, v.int() as u16),
            2 => set(&mut self.g, i, v),
            3 => set(&mut self.h, i, v),
            4 => {
                if self.locals.is_empty() {
                    self.locals.push(Vec::new());
                }
                set(self.locals.last_mut().unwrap(), i, v);
            }
            5 => set(&mut self.raw, i, v.int() as u8),
            6 => write_raw(&mut self.raw, 2 * i, &(v.int() as i16).to_le_bytes()),
            _ => write_raw(&mut self.raw, 4 * i, &v.int().to_le_bytes()),
        }
        Ok(())
    }

    fn next_random(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }
}

fn target(ins: &mes::Instr) -> u32 {
    match ins.operand {
        Operand::Target(t) => t,
        _ => unreachable!("jump without a target"),
    }
}

fn set<T: Clone + Default>(v: &mut Vec<T>, i: usize, x: T) {
    if v.len() <= i {
        v.resize(i + 1, T::default());
    }
    v[i] = x;
}

fn read_raw(raw: &[u8], at: usize, out: &mut [u8]) {
    for (k, b) in out.iter_mut().enumerate() {
        *b = raw.get(at + k).copied().unwrap_or(0);
    }
}

fn write_raw(raw: &mut Vec<u8>, at: usize, bytes: &[u8]) {
    if raw.len() < at + bytes.len() {
        raw.resize(at + bytes.len(), 0);
    }
    raw[at..at + bytes.len()].copy_from_slice(bytes);
}

fn binary(op: u8, a: Value, b: Value) -> Result<Value> {
    if let (0x34 | 0x42 | 0x43, true) = (op, a.as_str().is_some() || b.as_str().is_some()) {
        let text = |v: &Value| match v {
            Value::Str(s) => s.clone(),
            Value::Int(i) => i.to_string(),
        };
        let (x, y) = (text(&a), text(&b));
        return Ok(match op {
            0x34 => Value::Str(x + &y),
            0x42 => Value::Int((x == y) as i32),
            _ => Value::Int((x != y) as i32),
        });
    }
    let (x, y) = (a.int(), b.int());
    Ok(Value::Int(match op {
        0x34 => x.wrapping_add(y),
        0x35 => x.wrapping_sub(y),
        0x36 => x.wrapping_mul(y),
        0x37 | 0x38 => {
            if y == 0 {
                return Err(Error::DivideByZero);
            }
            if op == 0x37 {
                x.wrapping_div(y)
            } else {
                x.wrapping_rem(y)
            }
        }
        0x3a => (x != 0 && y != 0) as i32,
        0x3b => (x != 0 || y != 0) as i32,
        0x3c => x & y,
        0x3d => x | y,
        0x3e => (x < y) as i32,
        0x3f => (x > y) as i32,
        0x40 => (x <= y) as i32,
        0x41 => (x >= y) as i32,
        0x42 => (x == y) as i32,
        _ => (x != y) as i32,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small assembler for tests.
    #[derive(Default)]
    struct Asm(Vec<u8>);

    impl Asm {
        fn op(mut self, op: u8) -> Self {
            self.0.push(op);
            self
        }
        fn push(mut self, v: i32) -> Self {
            self.0.push(0x32);
            self.0.extend_from_slice(&v.to_be_bytes());
            self
        }
        fn push_s(mut self, s: &str) -> Self {
            self.0.push(0x33);
            self.0.extend_from_slice(s.as_bytes());
            self.0.push(0);
            self
        }
        fn jump(mut self, op: u8, to: u32) -> Self {
            self.0.push(op);
            self.0.extend_from_slice(&to.to_be_bytes());
            self
        }
        fn here(&self) -> u32 {
            self.0.len() as u32
        }
        fn script(self) -> Script {
            let mut d = vec![0u8; 8];
            d.extend_from_slice(&self.0);
            Script::parse(&d).unwrap()
        }
    }

    #[derive(Default)]
    struct Log {
        calls: Vec<(i32, Vec<Value>)>,
        messages: Vec<u32>,
    }

    impl Host for Log {
        fn native(&mut self, vm: &mut Vm, class: i32) -> Result<Flow> {
            let args = vm.pop_statement();
            self.calls.push((class, args));
            Ok(Flow::Continue)
        }
        fn message(&mut self, _vm: &mut Vm, n: u32) -> Result<Flow> {
            self.messages.push(n);
            Ok(Flow::Yield)
        }
    }

    fn run(script: Script) -> (Vm, Log, Stop) {
        let mut vm = Vm::new(Profile::default());
        vm.jump_script("TEST", script);
        let mut log = Log::default();
        let stop = vm.run(&mut log, 10_000).unwrap();
        (vm, log, stop)
    }

    #[test]
    fn arithmetic_and_variables() {
        // G[3] = (7 - 2) * 4 % 6;  H[1] = "a" + 5;  G[4] = G[3] == 2
        let s = Asm::default()
            .push(7)
            .push(2)
            .op(0x35)
            .push(4)
            .op(0x36)
            .push(6)
            .op(0x38)
            .push(3)
            .op(0x0e)
            .push_s("a")
            .push(5)
            .op(0x34)
            .push(1)
            .op(0x0f)
            .push(3)
            .op(0x04)
            .push(2)
            .op(0x42)
            .push(4)
            .op(0x0e)
            .op(0x00);
        let (vm, _, stop) = run(s.script());
        assert_eq!(stop, Stop::Ended);
        assert_eq!(vm.var_g(3), Value::Int(2));
        assert_eq!(vm.var_h(1), Value::from("a5"));
        assert_eq!(vm.var_g(4), Value::Int(1));
    }

    #[test]
    fn functions_and_natives() {
        // fdef 1 (argc 2) { native class 9 with local1, local0; fret }
        // call 1 (10, 20)
        let a = Asm::default().push(2).push(1);
        let fdef_at = a.here();
        let body = a.jump(0x16, 0);
        let body = body
            .push(1)
            .op(0x06)
            .push(0)
            .op(0x06)
            .push(9)
            .op(0x18)
            .op(0x01);
        let after = body.here();
        let mut code = body.push(20).push(10).push(1).op(0x17).op(0x00).0;
        code[fdef_at as usize + 1..fdef_at as usize + 5].copy_from_slice(&after.to_be_bytes());
        let (vm, log, stop) = run(Asm(code).script());
        assert_eq!(stop, Stop::Ended);
        assert_eq!(vm.functions()[&1].argc, 2);
        // First pop (10) is local 0.
        assert_eq!(log.calls, [(9, vec![Value::Int(20), Value::Int(10)])]);
    }

    /// message 5; option 0 { G[1] = 100; ret } option 1 { G[1] = 200; ret }
    /// native 3; ret
    fn option_code() -> (Vec<u8>, u32) {
        let a = Asm::default().jump(0x19, 5).push(0);
        let o1 = a.here();
        let a = a.jump(0x1b, 0).push(100).push(1).op(0x0e).op(0x00);
        let next = a.here();
        let a = a.push(1);
        let o2 = a.here();
        let a = a.jump(0x1b, 0).push(200).push(1).op(0x0e).op(0x00);
        let end = a.here();
        let mut code = a.push(3).op(0x18).op(0x00).0;
        let o2u = o2 as usize;
        code[o1 as usize + 1..o1 as usize + 5].copy_from_slice(&next.to_be_bytes());
        code[o2u + 1..o2u + 5].copy_from_slice(&end.to_be_bytes());

        (code, end)
    }

    #[test]
    fn snapshot_round_trip() {
        let (code, _) = option_code();
        let mut vm = Vm::new(Profile::default());
        vm.jump_script("TEST", Asm(code.clone()).script());
        let mut log = Log::default();
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Yield);
        vm.set_g(7, 42);
        let snap = vm.snapshot();
        let mut other = Vm::new(Profile::default());
        let mut loads = 0;
        other
            .restore_snapshot(snap, &mut |name| {
                assert_eq!(name, "TEST");
                loads += 1;
                Ok(Asm(code.clone()).script())
            })
            .unwrap();
        assert_eq!(loads, 1);
        assert_eq!(other.var_g(7), Value::Int(42));
        assert_eq!(other.pc(), vm.pc());
        assert_eq!(other.run(&mut log, 100).unwrap(), Stop::Options);
    }

    #[test]
    fn all_option_bodies_run_in_order() {
        let mut vm = Vm::new(Profile::default());
        vm.jump_script("TEST", Asm(option_code().0).script());
        let mut log = Log::default();
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Yield);
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Options);
        vm.run_all_options().unwrap();
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Ended);
        // Both bodies ran, the second last, then the code after the list.
        assert_eq!(vm.var_g(1), Value::Int(200));
        assert_eq!(log.calls.len(), 1);
    }

    #[test]
    fn options_and_messages() {
        // message 5; option 0 { G[1] = 100; ret } option 1 { G[1] = 200; ret }
        // native 3; ret
        let (code, end) = option_code();
        let mut vm = Vm::new(Profile::default());
        vm.jump_script("TEST", Asm(code).script());
        let mut log = Log::default();
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Yield);
        assert_eq!(log.messages, [5]);
        assert_eq!(vm.var_g(VAR_MESSAGE), Value::Int(5));
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Options);
        assert_eq!(vm.options().len(), 2);
        assert_eq!(vm.pc(), end);
        vm.choose(1).unwrap();
        assert_eq!(vm.run(&mut log, 100).unwrap(), Stop::Ended);
        assert_eq!(vm.var_g(1), Value::Int(200));
        assert_eq!(log.calls.len(), 1);
    }

    #[test]
    fn script_calls_return() {
        let mut vm = Vm::new(Profile::default());
        vm.jump_script("MAIN", Asm::default().push(1).op(0x18).op(0x00).script());
        struct H;
        impl Host for H {
            fn native(&mut self, vm: &mut Vm, _class: i32) -> Result<Flow> {
                let sub = Asm::default().push(7).push(2).op(0x0e).op(0x00).script();
                vm.call_script("SUB", sub);
                Ok(Flow::Continue)
            }
        }
        assert_eq!(vm.run(&mut H, 100).unwrap(), Stop::Ended);
        assert_eq!(vm.var_g(2), Value::Int(7));
    }
}
