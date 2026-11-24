//! `.SCRIPT`, `.LIB` and `.PACKAGE` scripts of the "eomaia" engine.
//!
//! These scripts share Silky's archive format and most of its byte code, but
//! number the variable and jump instructions differently.
//!
//! ```text
//! "SCRE"                       magic
//! u32 a                        not read yet (0 in story scripts, 2..91 in libraries and packages)
//! u32 n                        message count
//! u32 table                    file offset of the message table (20)
//! u32 code                     file offset of the code
//! [u32; n] at table            code offsets (relative to the code) of each message instruction
//! code                         byte code up to the end of the file
//! ```
//!
//! All header fields are little endian. Each instruction is one opcode byte
//! and a fixed operand; multi-byte operands are big endian.
//!
//! | op | mnemonic | operand |
//! | --- | --- | --- |
//! | 00 | `ret` | |
//! | 01 | `fret` | |
//! | 0a 0b 0c 0d | `ld.g` `ld.x` `ld.s` `ld.l` | |
//! | 0e 0f 10 11 | `st.g` `st.x` `st.s` `st.l` | |
//! | 15 | `jz` | u32 target |
//! | 16 | `jmp` | u32 target |
//! | 17 | `native` | none, or a u32 target (see below) |
//! | 18 | `message` | u32 message number, u32 (0 or 1, not read yet) |
//! | 19 | `option` | u32 target |
//! | 32 | `push` | i32 |
//! | 33 | `push.s` | NUL-terminated UTF-8 string |
//! | 34..43 | `add` `sub` `mul` `div` `mod` `rand` `land` `lor` `and` `or` `lt` `gt` `le` `ge` `eq` `ne` | |
//! | fa | `pop` | |
//! | fb fc fd | `nop` | |
//! | fe | `dbg.call` | |
//! | ff | `dbg.line` | |
//!
//! `native` pops a class number, and the class pops a sub command. When the
//! two instructions before it are `push sub` and `push class`, the class is
//! 11, 12, 13 or 14 and the sub command is 1, the instruction is a function
//! definition (in a library, package, user animation or task) and a u32
//! target follows: where the body of the definition ends. Decoding therefore
//! needs the two previous instructions.

use crate::error::{invalid, Result};

const FORMAT: &str = "SCRE";

/// Whether the data starts with the script magic.
pub fn is_scre(data: &[u8]) -> bool {
    data.starts_with(b"SCRE")
}

/// Operand that follows an opcode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    None,
    Int(i32),
    /// Code offset.
    Target(u32),
    Message {
        number: u32,
        /// Second number of the instruction (0 or 1; not read yet).
        extra: u32,
    },
    Str(String),
    /// `native`: the class and sub command when the two previous
    /// instructions gave them, and the end of the body for a definition.
    Native {
        class: Option<i32>,
        sub: Option<i32>,
        end: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instr {
    /// Offset relative to the start of the code.
    pub offset: u32,
    pub opcode: u8,
    pub operand: Operand,
}

/// Short mnemonic for an opcode.
pub fn mnemonic(op: u8) -> String {
    let name = match op {
        0x00 => "ret",
        0x01 => "fret",
        0x0a => "ld.g",
        0x0b => "ld.x",
        0x0c => "ld.s",
        0x0d => "ld.l",
        0x0e => "st.g",
        0x0f => "st.x",
        0x10 => "st.s",
        0x11 => "st.l",
        0x15 => "jz",
        0x16 => "jmp",
        0x17 => "native",
        0x18 => "message",
        0x19 => "option",
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
        0xfa => "pop",
        0xfb..=0xfd => "nop",
        0xfe => "dbg.call",
        0xff => "dbg.line",
        _ => return format!("op_{op:02x}"),
    };
    name.to_string()
}

pub struct Script {
    pub header_a: u32,
    /// Code offset of each message instruction.
    pub messages: Vec<u32>,
    pub code: Vec<u8>,
}

impl Script {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if !is_scre(data) {
            return invalid(FORMAT, "bad magic");
        }
        let u32_at = |o: usize| -> Result<u32> {
            match data.get(o..o + 4) {
                Some(b) => Ok(u32::from_le_bytes(b.try_into().unwrap())),
                None => invalid(FORMAT, "read past the end of the file"),
            }
        };
        let header_a = u32_at(4)?;
        let n = u32_at(8)? as usize;
        let table = u32_at(12)? as usize;
        let code = u32_at(16)? as usize;
        let table_end = n
            .checked_mul(4)
            .and_then(|b| b.checked_add(table))
            .filter(|&e| e <= data.len());
        if table_end.is_none() || code > data.len() {
            return invalid(FORMAT, "tables past the end of the file");
        }
        let messages = (0..n)
            .map(|i| u32_at(table + 4 * i))
            .collect::<Result<_>>()?;
        Ok(Self {
            header_a,
            messages,
            code: data[code..].to_vec(),
        })
    }

    /// Decodes the instruction at `offset`; `prev` is the `(class, sub)`
    /// pair of the two instructions before it, if they were `push sub`,
    /// `push class`.
    pub fn decode_at(&self, offset: u32, prev: Option<(i32, i32)>) -> Result<(Instr, u32)> {
        decode_at(&self.code, offset, prev)
    }

    /// Decodes the whole code linearly.
    pub fn disassemble(&self) -> Result<Vec<Instr>> {
        let mut out: Vec<Instr> = Vec::new();
        let mut pos = 0u32;
        while (pos as usize) < self.code.len() {
            let prev = match out.as_slice() {
                [.., a, b] => match (a.opcode, &a.operand, b.opcode, &b.operand) {
                    (0x32, Operand::Int(sub), 0x32, Operand::Int(class)) => Some((*class, *sub)),
                    _ => None,
                },
                _ => None,
            };
            let (instr, next) = self.decode_at(pos, prev)?;
            out.push(instr);
            pos = next;
        }
        Ok(out)
    }
}

pub fn decode_at(code: &[u8], offset: u32, prev: Option<(i32, i32)>) -> Result<(Instr, u32)> {
    let mut p = offset as usize;
    let Some(&opcode) = code.get(p) else {
        return invalid(
            FORMAT,
            format!("offset {offset:#x} past the end of the code"),
        );
    };
    p += 1;
    let word = |p: usize| -> Result<[u8; 4]> {
        match code.get(p..p + 4) {
            Some(b) => Ok(b.try_into().unwrap()),
            None => invalid(FORMAT, format!("operand past the end at {offset:#x}")),
        }
    };
    let operand = match opcode {
        0x00 | 0x01 | 0x0a..=0x11 | 0x34..=0x43 | 0xfa..=0xff => Operand::None,
        0x15 | 0x16 | 0x19 => {
            let w = word(p)?;
            p += 4;
            Operand::Target(u32::from_be_bytes(w))
        }
        0x17 => {
            let (class, sub) = match prev {
                Some((c, s)) => (Some(c), Some(s)),
                None => (None, None),
            };
            let end = if matches!(prev, Some((11..=14, 1))) {
                let w = word(p)?;
                p += 4;
                Some(u32::from_be_bytes(w))
            } else {
                None
            };
            Operand::Native { class, sub, end }
        }
        0x18 => {
            let number = u32::from_be_bytes(word(p)?);
            let extra = u32::from_be_bytes(word(p + 4)?);
            p += 8;
            Operand::Message { number, extra }
        }
        0x32 => {
            let w = word(p)?;
            p += 4;
            Operand::Int(i32::from_be_bytes(w))
        }
        0x33 => {
            let rest = &code[p..];
            let Some(len) = rest.iter().position(|&b| b == 0) else {
                return invalid(FORMAT, format!("unterminated string at {offset:#x}"));
            };
            let text = String::from_utf8_lossy(&rest[..len]).into_owned();
            p += len + 1;
            Operand::Str(text)
        }
        _ => {
            return invalid(
                FORMAT,
                format!("unknown opcode {opcode:#04x} at {offset:#x}"),
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn script(code: &[u8], messages: &[u32]) -> Vec<u8> {
        let mut d = b"SCRE".to_vec();
        d.extend_from_slice(&7u32.to_le_bytes());
        d.extend_from_slice(&(messages.len() as u32).to_le_bytes());
        d.extend_from_slice(&20u32.to_le_bytes());
        d.extend_from_slice(&(20 + 4 * messages.len() as u32).to_le_bytes());
        for m in messages {
            d.extend_from_slice(&m.to_le_bytes());
        }
        d.extend_from_slice(code);
        d
    }

    fn push(v: i32) -> Vec<u8> {
        let mut b = vec![0x32];
        b.extend_from_slice(&v.to_be_bytes());
        b
    }

    #[test]
    fn header_and_message_table() {
        let data = script(&[0x00, 0x01], &[0, 1]);
        assert!(is_scre(&data));
        let s = Script::parse(&data).unwrap();
        assert_eq!(s.header_a, 7);
        assert_eq!(s.messages, [0, 1]);
        assert_eq!(s.code, [0x00, 0x01]);
        assert!(Script::parse(b"MES\0").is_err());
        assert!(Script::parse(&data[..22]).is_err());
    }

    #[test]
    fn operand_kinds() {
        let mut code = push(-2);
        code.extend([0x33, b'h', b'i', 0xe3, 0x81, 0x82, 0]);
        code.extend([0x15, 0, 0, 0, 0x30]);
        code.extend([0x16, 0, 0, 1, 0]);
        code.extend([0x19, 0, 0, 0, 2]);
        code.extend([0x18, 0, 0, 0, 5, 0, 0, 0, 1]);
        code.extend([0x0a, 0x11, 0x34, 0x43, 0xfa, 0xfc, 0xfe, 0xff, 0x00]);
        let s = Script::parse(&script(&code, &[])).unwrap();
        let ins = s.disassemble().unwrap();
        assert_eq!(ins[0].operand, Operand::Int(-2));
        assert_eq!(ins[1].operand, Operand::Str("hiあ".into()));
        assert_eq!(ins[2].operand, Operand::Target(0x30));
        assert_eq!(ins[3].operand, Operand::Target(0x100));
        assert_eq!(ins[4].operand, Operand::Target(2));
        assert_eq!(
            ins[5].operand,
            Operand::Message {
                number: 5,
                extra: 1
            }
        );
        assert_eq!(ins.len(), 15);
        assert_eq!(mnemonic(ins[6].opcode), "ld.g");
        assert_eq!(mnemonic(ins[12].opcode), "dbg.call");
        assert_eq!(ins[1].offset, 5);
    }

    #[test]
    fn native_definition_takes_a_target() {
        let mut code = push(1);
        code.extend(push(12));
        code.extend([0x17, 0, 0, 0, 0x40]);
        code.extend(push(2));
        code.extend(push(12));
        code.extend([0x17]);
        code.extend(push(1));
        code.extend(push(5));
        code.extend([0x17]);
        let s = Script::parse(&script(&code, &[])).unwrap();
        let ins = s.disassemble().unwrap();
        assert_eq!(
            ins[2].operand,
            Operand::Native {
                class: Some(12),
                sub: Some(1),
                end: Some(0x40)
            }
        );
        // Other sub commands and classes carry no target.
        assert_eq!(
            ins[5].operand,
            Operand::Native {
                class: Some(12),
                sub: Some(2),
                end: None
            }
        );
        assert_eq!(
            ins[8].operand,
            Operand::Native {
                class: Some(5),
                sub: Some(1),
                end: None
            }
        );
        // Without pushes before it, there is nothing to know.
        let s = Script::parse(&script(&[0x17], &[])).unwrap();
        assert_eq!(
            s.disassemble().unwrap()[0].operand,
            Operand::Native {
                class: None,
                sub: None,
                end: None
            }
        );
    }

    #[test]
    fn unknown_opcode_is_an_error() {
        for op in [0x02, 0x14, 0x1a, 0x44, 0xf9] {
            let s = Script::parse(&script(&[op], &[])).unwrap();
            assert!(s.disassemble().is_err(), "{op:#x}");
        }
    }

    #[test]
    fn truncated_operand_is_an_error() {
        for code in [
            &[0x32, 0, 0][..],
            &[0x16, 0, 0, 0],
            &[0x18, 0, 0, 0, 1, 0, 0],
            &[0x33, b'a'],
        ] {
            let s = Script::parse(&script(code, &[])).unwrap();
            assert!(s.disassemble().is_err());
        }
    }
}
