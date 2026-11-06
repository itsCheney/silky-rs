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
//! a fixed operand (see [`operand_kind`]). Function calls into the engine are
//! made by name: the script pushes the function name and its module
//! (`"msg"`, `"main.inc"`) as strings before the call opcodes.

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
    /// Big-endian 32-bit value of unknown meaning.
    U32Be,
    /// Little-endian 32-bit value (only in `LIBLARY.LIB`).
    U32Le,
    /// NUL-terminated Shift-JIS string.
    Str,
    /// NUL-terminated obfuscated Shift-JIS string.
    CryptStr,
}

/// Operand kind of an opcode, or `None` if the opcode is unknown.
pub fn operand_kind(op: u8) -> Option<OperandKind> {
    use OperandKind::*;
    Some(match op {
        0x00 | 0x02..=0x06 | 0x0c..=0x0f | 0x11 | 0x17 | 0x18 => None,
        0x34 | 0x35 | 0x37..=0x39 | 0x3a..=0x3f | 0x40..=0x43 | 0xfa..=0xff => None,
        0x01 => U32Le,
        0x0a => CryptStr,
        0x0b | 0x33 => Str,
        0x10 | 0x1c | 0x36 => U8,
        0x14 | 0x15 | 0x16 | 0x1b => Target,
        0x19 => Message,
        0x1a => U32Be,
        0x32 => Int,
        _ => return Option::None,
    })
}

/// Short mnemonic for an opcode; unknown meaning gives `op_XX`.
pub fn mnemonic(op: u8) -> String {
    match op {
        0x00 => "ret".into(),
        0x0a => "str_crypt".into(),
        0x0b => "str".into(),
        0x14 => "jump".into(),
        0x15 => "msg_offset".into(),
        0x16 => "spec_offset".into(),
        0x19 => "message".into(),
        0x1c => "new_line".into(),
        0x32 => "push".into(),
        0x33 => "push_str".into(),
        0x36 => "jump_short".into(),
        _ => format!("op_{op:02x}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    None,
    U8(u8),
    Int(i32),
    Target(u32),
    Message(u32),
    U32(u32),
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
        OperandKind::U32Be => {
            let w = word(p)?;
            p += 4;
            Operand::U32(u32::from_be_bytes(w))
        }
        OperandKind::U32Le => {
            let w = word(p)?;
            p += 4;
            Operand::U32(u32::from_le_bytes(w))
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
