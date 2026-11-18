//! `.AX` files: frame programs for the engine's `Anime` class, such as the
//! blinking faces of 同級生 Remake.
//!
//! ```text
//! 320 × u32   program offsets: 10 groups of 32 programs (group × 32 + index)
//! n × 7 i32   cells: kind, source x, y, width, height, screen x, y
//! ...         the programs' bytes
//! ```
//!
//! The cells run from byte 1280 up to the first program. A program is a
//! list of operations, each a byte and for most a u32 after it:
//!
//! | Byte | Operand | Meaning |
//! |---|---|---|
//! | 0 | | nothing this frame |
//! | 1 | frames | hold: wait, and stop or pause here when asked to |
//! | 2 | frames | wait |
//! | 3 | | back to the start |
//! | 4, 255 | | end |
//! | 5, 7 | count | start of a loop (two levels) |
//! | 6, 8 | | end of that loop |
//! | 9 and up | cell | draw a cell |
//!
//! An unused program points at a 255.

use crate::error::{invalid, Result};

const FORMAT: &str = "AX";
/// Groups of programs.
pub const GROUPS: usize = 10;
/// Programs in a group.
pub const PROGRAMS: usize = 32;
const CELLS: usize = GROUPS * PROGRAMS * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// How the cell is drawn: 3 lays it over the background, putting the
    /// background back under it first.
    pub kind: i32,
    pub sx: i32,
    pub sy: i32,
    pub w: i32,
    pub h: i32,
    pub x: i32,
    pub y: i32,
}

/// One operation of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Nothing,
    Hold(u32),
    Wait(u32),
    Restart,
    End,
    Loop { level: u8, count: u32 },
    Next { level: u8 },
    Draw(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ax {
    /// Program offsets into `data`.
    programs: Vec<u32>,
    pub cells: Vec<Cell>,
    data: Vec<u8>,
}

impl Ax {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < CELLS {
            return invalid(FORMAT, "shorter than its program table");
        }
        let word = |at: usize| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        let programs: Vec<u32> = (0..GROUPS * PROGRAMS).map(|i| word(i * 4)).collect();
        if let Some(p) = programs.iter().find(|&&p| p as usize >= data.len()) {
            return invalid(FORMAT, format!("program at {p} past the end"));
        }
        let end = programs
            .iter()
            .map(|&p| p as usize)
            .filter(|&p| p >= CELLS)
            .min()
            .unwrap_or(data.len());
        let cells = data[CELLS..end]
            .chunks_exact(28)
            .map(|c| {
                let v = |k: usize| i32::from_le_bytes(c[k * 4..k * 4 + 4].try_into().unwrap());
                Cell {
                    kind: v(0),
                    sx: v(1),
                    sy: v(2),
                    w: v(3),
                    h: v(4),
                    x: v(5),
                    y: v(6),
                }
            })
            .collect();
        Ok(Self {
            programs,
            cells,
            data: data.to_vec(),
        })
    }

    /// Where program `index` of `group` starts.
    pub fn program(&self, group: usize, index: usize) -> Option<u32> {
        (group < GROUPS && index < PROGRAMS).then(|| self.programs[group * PROGRAMS + index])
    }

    /// The operation at `at` and where the next one starts.
    pub fn op(&self, at: u32) -> (Op, u32) {
        let at = at as usize;
        let Some(&b) = self.data.get(at) else {
            return (Op::End, at as u32);
        };
        let arg = || {
            self.data
                .get(at + 1..at + 5)
                .map_or(0, |w| u32::from_le_bytes(w.try_into().unwrap()))
        };
        let (op, len) = match b {
            0 => (Op::Nothing, 1),
            1 => (Op::Hold(arg()), 5),
            2 => (Op::Wait(arg()), 5),
            3 => (Op::Restart, 1),
            4 | 255 => (Op::End, 1),
            5 | 7 => (
                Op::Loop {
                    level: (b - 5) / 2,
                    count: arg(),
                },
                5,
            ),
            6 | 8 => (Op::Next { level: (b - 6) / 2 }, 1),
            _ => (Op::Draw(arg()), 5),
        };
        (op, (at + len) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_and_cells() {
        let mut d = vec![0u8; CELLS];
        let start = (CELLS + 28) as u32;
        for i in 0..GROUPS * PROGRAMS {
            d[i * 4..i * 4 + 4].copy_from_slice(&(start + 11).to_le_bytes());
        }
        d[..4].copy_from_slice(&start.to_le_bytes());
        for v in [3i32, 0, 192, 176, 192, 1344, 128] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&[9, 0, 0, 0, 0, 1, 150, 0, 0, 0, 3, 255]);
        let ax = Ax::parse(&d).unwrap();
        assert_eq!(ax.cells.len(), 1);
        assert_eq!(ax.cells[0].y, 128);
        let p = ax.program(0, 0).unwrap();
        assert_eq!(ax.op(p), (Op::Draw(0), p + 5));
        assert_eq!(ax.op(p + 5), (Op::Hold(150), p + 10));
        assert_eq!(ax.op(p + 10).0, Op::Restart);
        assert_eq!(ax.op(ax.program(0, 1).unwrap()).0, Op::End);
    }
}
