//! `.HLG` help tables: the tooltip lines of a screen in three languages
//! (きまぐれテンプテーション2; ふゆから、くるる。 keeps them in `.MAP`
//! tables instead).
//!
//! ```text
//! u32 count
//! count times: u32 offset (Japanese), u32 offset (English), u32 offset (Chinese)
//! text: NUL-terminated UTF-8
//! ```

use crate::error::{invalid, Result};

const FORMAT: &str = "HLG";

/// Columns of a help table.
pub const JAPANESE: usize = 0;
pub const ENGLISH: usize = 1;
pub const CHINESE: usize = 2;

#[derive(Debug, Clone, Default)]
pub struct HelpTable {
    lines: Vec<[String; 3]>,
}

impl HelpTable {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let u32_at = |o: usize| -> Result<usize> {
            match data.get(o..o + 4) {
                Some(b) => Ok(u32::from_le_bytes(b.try_into().unwrap()) as usize),
                None => invalid(FORMAT, format!("read past the end at {o:#x}")),
            }
        };
        let count = u32_at(0)?;
        if 4 + 12 * count > data.len() {
            return invalid(FORMAT, format!("{count} lines in {} bytes", data.len()));
        }
        let text = |o: usize| -> Result<String> {
            let Some(rest) = data.get(o..) else {
                return invalid(FORMAT, format!("text offset {o:#x} past the end"));
            };
            let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            Ok(String::from_utf8_lossy(&rest[..end]).into_owned())
        };
        let mut lines = Vec::with_capacity(count);
        for i in 0..count {
            let o = 4 + 12 * i;
            lines.push([
                text(u32_at(o)?)?,
                text(u32_at(o + 4)?)?,
                text(u32_at(o + 8)?)?,
            ]);
        }
        Ok(Self { lines })
    }

    /// Line `index` in column `column` ([`JAPANESE`], [`ENGLISH`],
    /// [`CHINESE`]); `None` if missing or empty.
    pub fn line(&self, column: usize, index: usize) -> Option<&str> {
        let s = self.lines.get(index)?.get(column)?;
        (!s.is_empty()).then_some(s.as_str())
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_columns() {
        let mut d = 2u32.to_le_bytes().to_vec();
        let texts: [&[u8]; 6] = [b"a\0", b"b\0", b"c\0", b"\0", b"\0", b"\0"];
        let mut o = 4 + 24;
        for t in texts {
            d.extend((o as u32).to_le_bytes());
            o += t.len();
        }
        for t in texts {
            d.extend(t);
        }
        let h = HelpTable::parse(&d).unwrap();
        assert_eq!(h.line(ENGLISH, 0), Some("b"));
        assert_eq!(h.line(CHINESE, 0), Some("c"));
        assert_eq!(h.line(JAPANESE, 1), None);
        assert!(HelpTable::parse(&d[..10]).is_err());
    }
}
