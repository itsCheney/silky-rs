//! `.MES` scripts and the `LIBLARY.LIB` script library.
//!
//! ```text
//! u32 count_a, u32 count_b
//! [u32; count_a] table_a     code offsets, one per message in story scripts
//! [u32; count_b] table_b
//! code                       byte code; offsets are relative to its start
//! ```
//!
//! The byte code is a stack machine. Each instruction is one opcode byte and
//! a fixed operand (see [`operand_kind`]). Engine functions are called by
//! number (opcode 0x18). The macro names seen in the code (`"msg"`,
//! `"main.inc"`) are debug information for opcode 0xfe only.

use crate::error::{invalid, Result};

const FORMAT: &str = "MES";

/// Per engine build differences in the byte code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// `PUSH` (0x32) operand byte order. Big endian in the 2026 international
    /// build of ふゆから、くるる。, little endian in older builds.
    pub push_big_endian: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            push_big_endian: true,
        }
    }
}

/// Operand that follows an opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandKind {
    None,
    U8,
    /// Signed 32-bit integer, byte order from [`Profile`].
    Int,
    /// Big-endian code offset.
    Target,
    /// Big-endian message number.
    Message,
    /// NUL-terminated Shift-JIS string.
    Str,
    /// NUL-terminated obfuscated Shift-JIS string.
    CryptStr,
}

/// Operand kind of an opcode, or `None` if the opcode is unknown.
pub fn operand_kind(op: u8) -> Option<OperandKind> {
    use OperandKind::*;
    Some(match op {
        0x00..=0x09 | 0x0c..=0x13 | 0x17 | 0x18 => None,
        0x32..=0x43 if op != 0x32 && op != 0x33 => None,
        0xfa..=0xff => None,
        0x0a => CryptStr,
        0x0b | 0x33 => Str,
        0x1c => U8,
        0x14 | 0x15 | 0x16 | 0x1b => Target,
        0x19 | 0x1a => Message,
        0x32 => Int,
        _ => return Option::None,
    })
}

/// Short mnemonic for an opcode.
///
/// Semantics (from the engine's interpreter; "pop" takes the top of stack):
///
/// | op | mnemonic | effect |
/// | --- | --- | --- |
/// | 00 | `ret` | return from a script call; end when nothing to return to |
/// | 01 | `fret` | return from a script function (drops its locals) |
/// | 02 03 | `ld.b` `ld.w` | push byte / u16 variable `[pop]` |
/// | 04 05 | `ld.g` `ld.h` | push value variable `[pop]` of bank G / H |
/// | 06 | `ld.l` | push local `[pop]` of the current function |
/// | 07 08 09 | `ld.r8` `ld.r16` `ld.r32` | push from the raw area, indexed by element |
/// | 0a 0b | `push.cs` `push.s` | push a string (0a obfuscated) |
/// | 0c..13 | `st.*` | `index = pop; value = pop;` store, same banks as 02..09 |
/// | 14 | `jz` | `if pop == 0 { goto target }` |
/// | 15 | `jmp` | `goto target` |
/// | 16 | `fdef` | `id = pop; argc = pop;` define a function whose body follows; goto target |
/// | 17 | `call` | `id = pop;` call function `id`, popping its arguments into locals |
/// | 18 | `native` | `id = pop;` engine function class `id` (it pops its own arguments) |
/// | 19 | `message` | message number (G[52] = n) |
/// | 1a | `smessage` | selection message number (G[54] = n) |
/// | 1b | `option` | `value = pop;` record a selection option starting after this; goto target |
/// | 1c | `text` | text output with a sub-command byte |
/// | 32 33 | `push` `push.s` | push an int / a string |
/// | 34..38 | `add` `sub` `mul` `div` `mod` | `b = pop; a = pop; push a op b` (add joins strings) |
/// | 39 | `rand` | `n = pop; push random % n` |
/// | 3a 3b | `land` `lor` | logical and / or (true is 1) |
/// | 3c 3d | `and` `or` | bitwise |
/// | 3e..43 | `lt` `gt` `le` `ge` `eq` `ne` | comparisons, push 1 or 0 |
/// | fa..fd | `nop` | |
/// | fe | `dbg.call` | `line = pop; module = pop; name = pop;` source macro call site |
/// | ff | `dbg.line` | `line = pop;` source line |
pub fn mnemonic(op: u8) -> String {
    let name = match op {
        0x00 => "ret",
        0x01 => "fret",
        0x02 => "ld.b",
        0x03 => "ld.w",
        0x04 => "ld.g",
        0x05 => "ld.h",
        0x06 => "ld.l",
        0x07 => "ld.r8",
        0x08 => "ld.r16",
        0x09 => "ld.r32",
        0x0a => "push.cs",
        0x0b => "push.s",
        0x0c => "st.b",
        0x0d => "st.w",
        0x0e => "st.g",
        0x0f => "st.h",
        0x10 => "st.l",
        0x11 => "st.r8",
        0x12 => "st.r16",
        0x13 => "st.r32",
        0x14 => "jz",
        0x15 => "jmp",
        0x16 => "fdef",
        0x17 => "call",
        0x18 => "native",
        0x19 => "message",
        0x1a => "smessage",
        0x1b => "option",
        0x1c => "text",
        0x32 => "push",
        0x33 => "push.s",
        0x34 => "add",
        0x35 => "sub",
        0x36 => "mul",
        0x37 => "div",
        0x38 => "mod",
        0x39 => "rand",
        0x3a => "land",
        0x3b => "lor",
        0x3c => "and",
        0x3d => "or",
        0x3e => "lt",
        0x3f => "gt",
        0x40 => "le",
        0x41 => "ge",
        0x42 => "eq",
        0x43 => "ne",
        0xfa..=0xfd => "nop",
        0xfe => "dbg.call",
        0xff => "dbg.line",
        _ => return format!("op_{op:02x}"),
    };
    name.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    None,
    U8(u8),
    Int(i32),
    Target(u32),
    Message(u32),
    /// Shift-JIS bytes (already de-obfuscated for `CryptStr`).
    Str(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instr {
    /// Offset relative to the start of the code.
    pub offset: u32,
    pub opcode: u8,
    pub operand: Operand,
}

impl Instr {
    pub fn text(&self) -> Option<String> {
        match &self.operand {
            Operand::Str(b) => Some(crate::decode_cp932(b)),
            _ => None,
        }
    }
}

pub struct Script {
    pub table_a: Vec<u32>,
    pub table_b: Vec<u32>,
    pub code: Vec<u8>,
}

impl Script {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let u32_at = |o: usize| -> Result<u32> {
            match data.get(o..o + 4) {
                Some(b) => Ok(u32::from_le_bytes(b.try_into().unwrap())),
                None => invalid(FORMAT, "header past the end of the file"),
            }
        };
        let a = u32_at(0)? as usize;
        let b = u32_at(4)? as usize;
        let code_start = 8 + 4 * (a + b);
        if code_start > data.len() {
            return invalid(FORMAT, "offset tables past the end of the file");
        }
        let table_a = (0..a).map(|i| u32_at(8 + 4 * i)).collect::<Result<_>>()?;
        let table_b = (0..b)
            .map(|i| u32_at(8 + 4 * (a + i)))
            .collect::<Result<_>>()?;
        Ok(Self {
            table_a,
            table_b,
            code: data[code_start..].to_vec(),
        })
    }

    /// Decodes a string operand (Shift-JIS).
    pub fn decode(&self, bytes: &[u8]) -> String {
        crate::decode_cp932(bytes)
    }

    /// Decodes the instruction at `offset`.
    pub fn decode_at(&self, offset: u32, profile: Profile) -> Result<(Instr, u32)> {
        decode_at(&self.code, offset, profile)
    }

    /// Decodes the whole code linearly.
    pub fn disassemble(&self, profile: Profile) -> Result<Vec<Instr>> {
        let mut out = Vec::new();
        let mut pos = 0u32;
        while (pos as usize) < self.code.len() {
            let (instr, next) = self.decode_at(pos, profile)?;
            out.push(instr);
            pos = next;
        }
        Ok(out)
    }
}

pub fn decode_at(code: &[u8], offset: u32, profile: Profile) -> Result<(Instr, u32)> {
    let mut p = offset as usize;
    let Some(&opcode) = code.get(p) else {
        return invalid(
            FORMAT,
            format!("offset {offset:#x} past the end of the code"),
        );
    };
    p += 1;
    let Some(kind) = operand_kind(opcode) else {
        return invalid(
            FORMAT,
            format!("unknown opcode {opcode:#04x} at {offset:#x}"),
        );
    };
    let word = |p: usize| -> Result<[u8; 4]> {
        match code.get(p..p + 4) {
            Some(b) => Ok(b.try_into().unwrap()),
            None => invalid(FORMAT, format!("operand past the end at {offset:#x}")),
        }
    };
    let operand = match kind {
        OperandKind::None => Operand::None,
        OperandKind::U8 => {
            let Some(&b) = code.get(p) else {
                return invalid(FORMAT, format!("operand past the end at {offset:#x}"));
            };
            p += 1;
            Operand::U8(b)
        }
        OperandKind::Int => {
            let w = word(p)?;
            p += 4;
            Operand::Int(if profile.push_big_endian {
                i32::from_be_bytes(w)
            } else {
                i32::from_le_bytes(w)
            })
        }
        OperandKind::Target => {
            let w = word(p)?;
            p += 4;
            Operand::Target(u32::from_be_bytes(w))
        }
        OperandKind::Message => {
            let w = word(p)?;
            p += 4;
            Operand::Message(u32::from_be_bytes(w))
        }
        OperandKind::Str | OperandKind::CryptStr => {
            let rest = &code[p..];
            let Some(len) = rest.iter().position(|&b| b == 0) else {
                return invalid(FORMAT, format!("unterminated string at {offset:#x}"));
            };
            let raw = &rest[..len];
            p += len + 1;
            Operand::Str(if kind == OperandKind::CryptStr {
                decrypt(raw)
            } else {
                raw.to_vec()
            })
        }
    };
    Ok((
        Instr {
            offset,
            opcode,
            operand,
        },
        p as u32,
    ))
}

/// Undoes the string obfuscation of opcode 0x0A: a byte below 0x81 stands for
/// the two-byte Shift-JIS character `byte + 0x829E` (hiragana and friends);
/// other bytes are plain Shift-JIS lead/trail pairs.
pub fn decrypt(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() * 2);
    let mut i = 0;
    while i < raw.len() {
        let c = raw[i];
        if c < 0x81 {
            let v = u16::from(c) + 0x829e;
            out.extend_from_slice(&v.to_be_bytes());
            i += 1;
        } else {
            out.push(c);
            if let Some(&t) = raw.get(i + 1) {
                out.push(t);
            }
            i += 2;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(code: &[u8], table_a: &[u32]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&(table_a.len() as u32).to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        for t in table_a {
            d.extend_from_slice(&t.to_le_bytes());
        }
        d.extend_from_slice(code);
        d
    }

    #[test]
    fn decodes_call_sequence() {
        let code = [
            0x33, b'm', b's', b'g', 0, // push_str "msg"
            0x32, 0, 0, 1, 0x50, // push 336 (big endian)
            0x19, 0, 0, 0, 1, // message 1
            0x14, 0, 0, 0, 0,    // jump 0
            0x00, // ret
        ];
        let s = Script::parse(&script(&code, &[10])).unwrap();
        assert_eq!(s.table_a, [10]);
        let ins = s.disassemble(Profile::default()).unwrap();
        assert_eq!(ins.len(), 5);
        assert_eq!(ins[0].text().as_deref(), Some("msg"));
        assert_eq!(ins[1].operand, Operand::Int(336));
        assert_eq!(ins[2].operand, Operand::Message(1));
        assert_eq!(ins[2].offset, 10);
        assert_eq!(ins[3].operand, Operand::Target(0));
        let le = s
            .disassemble(Profile {
                push_big_endian: false,
            })
            .unwrap();
        assert_eq!(le[1].operand, Operand::Int(0x5001_0000));
    }

    #[test]
    fn unknown_opcode_is_an_error() {
        let s = Script::parse(&script(&[0x20], &[])).unwrap();
        assert!(s.disassemble(Profile::default()).is_err());
    }

    #[test]
    fn decrypts_kana() {
        // 0x01 -> 0x829F (ぁ), 0x82 0xA0 stays (あ).
        assert_eq!(decrypt(&[0x01, 0x82, 0xa0]), [0x82, 0x9f, 0x82, 0xa0]);
    }
}
