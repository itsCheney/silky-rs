//! `.MAP` text tables.
//!
//! In builds where the scripts carry no text, each script `X.MES` has a table
//! `X.MAP` with its lines, one section per language. Scripts show a line by
//! its position in the table (engine function 29, command 23), not by the
//! number stored with it; the number is a global line id kept for reference.
//!
//! ```text
//! u32 header_size                 = 8 * section count
//! repeat section count: u32 offset, u32 count
//! at each section offset, count times: u32 line id, u32 text offset
//! text: NUL-terminated (UTF-8 in the 2026 international build)
//! ```

use crate::error::{invalid, Result};

const FORMAT: &str = "MAP";

#[derive(Debug, Clone, Default)]
pub struct MapFile {
    /// Per section, `(line id, text)` in file order.
    sections: Vec<Vec<(u32, String)>>,
}

impl MapFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let u32_at = |o: usize| -> Result<u32> {
            match data.get(o..o + 4) {
                Some(b) => Ok(u32::from_le_bytes(b.try_into().unwrap())),
                None => invalid(FORMAT, format!("read past the end at {o:#x}")),
            }
        };
        let header_size = u32_at(0)? as usize;
        if header_size == 0 || header_size % 8 != 0 || header_size > data.len() {
            return invalid(FORMAT, format!("bad header size {header_size}"));
        }
        let mut sections = Vec::new();
        for s in 0..header_size / 8 {
            let offset = u32_at(8 * s)? as usize;
            let count = u32_at(8 * s + 4)? as usize;
            let mut table = Vec::with_capacity(count);
            for i in 0..count {
                let number = u32_at(offset + 8 * i)?;
                let text_at = u32_at(offset + 8 * i + 4)? as usize;
                let Some(rest) = data.get(text_at..) else {
                    return invalid(FORMAT, format!("text {number} outside the file"));
                };
                let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
                table.push((number, decode_text(&rest[..end])));
            }
            sections.push(table);
        }
        Ok(Self { sections })
    }

    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Text at `index` (file order) of a section; this is how scripts refer
    /// to lines.
    pub fn line(&self, section: usize, index: usize) -> Option<&str> {
        self.sections.get(section)?.get(index).map(|e| e.1.as_str())
    }

    /// Line id and text at a position.
    pub fn entry(&self, section: usize, index: usize) -> Option<(u32, &str)> {
        let (id, text) = self.sections.get(section)?.get(index)?;
        Some((*id, text.as_str()))
    }

    /// Text with the given line id.
    pub fn get(&self, section: usize, number: u32) -> Option<&str> {
        self.sections
            .get(section)?
            .iter()
            .find(|e| e.0 == number)
            .map(|e| e.1.as_str())
    }

    /// Entries of a section in file order.
    pub fn entries(&self, section: usize) -> Vec<(u32, &str)> {
        self.sections
            .get(section)
            .map(|t| t.iter().map(|(n, s)| (*n, s.as_str())).collect())
            .unwrap_or_default()
    }

    pub fn len(&self, section: usize) -> usize {
        self.sections.get(section).map_or(0, Vec::len)
    }

    pub fn is_empty(&self, section: usize) -> bool {
        self.len(section) == 0
    }
}

/// UTF-8 when valid, otherwise Shift-JIS (older builds).
fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => crate::decode_cp932(bytes),
    }
}

/// Builds a MAP file; used by tests.
pub fn build(sections: &[Vec<(u32, &str)>]) -> Vec<u8> {
    let header = 8 * sections.len();
    let tables: usize = sections.iter().map(|s| 8 * s.len()).sum();
    let mut out = vec![0u8; header + tables];
    let mut table_at = header;
    for (si, section) in sections.iter().enumerate() {
        out[8 * si..8 * si + 4].copy_from_slice(&(table_at as u32).to_le_bytes());
        out[8 * si + 4..8 * si + 8].copy_from_slice(&(section.len() as u32).to_le_bytes());
        for (i, (number, text)) in section.iter().enumerate() {
            let text_at = out.len() as u32;
            out.extend_from_slice(text.as_bytes());
            out.push(0);
            let e = table_at + 8 * i;
            out[e..e + 4].copy_from_slice(&number.to_le_bytes());
            out[e + 4..e + 8].copy_from_slice(&text_at.to_le_bytes());
        }
        table_at += 8 * section.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let data = build(&[
            vec![(7, "Hello"), (3, "World")],
            vec![(7, "你好"), (3, "世界")],
        ]);
        let map = MapFile::parse(&data).unwrap();
        assert_eq!(map.section_count(), 2);
        assert_eq!(map.get(0, 3), Some("World"));
        assert_eq!(map.get(1, 7), Some("你好"));
        assert_eq!(map.get(1, 4), None);
        assert_eq!(map.line(0, 0), Some("Hello"));
        assert_eq!(map.line(1, 1), Some("世界"));
        assert_eq!(map.line(1, 2), None);
    }
}
