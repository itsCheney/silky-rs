//! Silky's `.arc` archives.
//!
//! Layout (all game archives of the Silky Engine):
//!
//! ```text
//! u32 LE   index_size        bytes of index that follow
//! repeat until index_size bytes were read:
//!   u8     name_len
//!   [u8]   name              byte j stored as (cp932[j] - (name_len - j))
//!   u32 BE packed_size
//!   u32 BE size              packed with LZSS when it differs from packed_size
//!   u32 BE offset            from the start of the file
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::{invalid, Error, Result};
use crate::{decode_cp932, lzss};

const FORMAT: &str = "ARC";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// File name as stored, e.g. `FK001.MES`.
    pub name: String,
    pub offset: u64,
    pub packed_size: u32,
    pub size: u32,
}

impl Entry {
    pub fn is_packed(&self) -> bool {
        self.packed_size != self.size
    }
}

/// Parses an archive index. `data` must start at the beginning of the file and
/// hold at least the whole index.
pub fn parse_index(data: &[u8], file_len: u64) -> Result<Vec<Entry>> {
    let Some(head) = data.get(..4) else {
        return invalid(FORMAT, "file shorter than its header");
    };
    let index_size = u32::from_le_bytes(head.try_into().unwrap()) as usize;
    let end = 4 + index_size;
    if end > data.len() || end as u64 > file_len {
        return invalid(FORMAT, format!("index size {index_size} exceeds the file"));
    }
    let mut entries = Vec::new();
    let mut p = 4;
    while p < end {
        let n = data[p] as usize;
        p += 1;
        if n == 0 || p + n + 12 > end {
            return invalid(FORMAT, format!("bad index entry at {p:#x}"));
        }
        let name_bytes: Vec<u8> = data[p..p + n]
            .iter()
            .enumerate()
            .map(|(j, &b)| b.wrapping_add((n - j) as u8))
            .collect();
        p += n;
        let be = |o: usize| u32::from_be_bytes(data[o..o + 4].try_into().unwrap());
        let entry = Entry {
            name: decode_cp932(&name_bytes),
            packed_size: be(p),
            size: be(p + 4),
            offset: u64::from(be(p + 8)),
        };
        p += 12;
        if entry.offset < end as u64 || entry.offset + u64::from(entry.packed_size) > file_len {
            return invalid(
                FORMAT,
                format!("entry {} lies outside the file", entry.name),
            );
        }
        entries.push(entry);
    }
    Ok(entries)
}

/// Builds an archive in memory (entries stored without compression). Used by
/// tests and tools; the engine never writes archives.
pub fn build(files: &[(&str, &[u8])]) -> Vec<u8> {
    let names: Vec<Vec<u8>> = files
        .iter()
        .map(|(name, _)| {
            let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(name);
            bytes.into_owned()
        })
        .collect();
    let index_size: usize = names.iter().map(|n| 1 + n.len() + 12).sum();
    let mut out = Vec::new();
    out.extend_from_slice(&(index_size as u32).to_le_bytes());
    let mut offset = 4 + index_size;
    for (name, (_, data)) in names.iter().zip(files) {
        let n = name.len();
        out.push(n as u8);
        out.extend(
            name.iter()
                .enumerate()
                .map(|(j, &b)| b.wrapping_sub((n - j) as u8)),
        );
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (_, data) in files {
        out.extend_from_slice(data);
    }
    out
}

/// An open archive file. Entry data is read on demand.
pub struct Archive {
    path: PathBuf,
    file: Mutex<File>,
    entries: Vec<Entry>,
    by_name: HashMap<String, usize>,
}

impl Archive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let file_len = file.metadata()?.len();
        let mut head = [0u8; 4];
        file.read_exact(&mut head)?;
        let index_size = u32::from_le_bytes(head) as u64;
        if 4 + index_size > file_len {
            return invalid(
                FORMAT,
                format!("{}: index exceeds the file", path.display()),
            );
        }
        let mut index = vec![0u8; 4 + index_size as usize];
        file.seek(SeekFrom::Start(0))?;
        file.read_exact(&mut index)?;
        let entries = parse_index(&index, file_len)?;
        let by_name = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.name.to_ascii_lowercase(), i))
            .collect();
        Ok(Self {
            path,
            file: Mutex::new(file),
            entries,
            by_name,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Looks a name up, ignoring ASCII case (scripts use lower case names).
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .map(|&i| &self.entries[i])
    }

    /// Reads and unpacks an entry.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>> {
        let mut raw = vec![0u8; entry.packed_size as usize];
        {
            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(entry.offset))?;
            file.read_exact(&mut raw)?;
        }
        if !entry.is_packed() {
            return Ok(raw);
        }
        let data = lzss::decompress(&raw, entry.size as usize);
        if data.len() != entry.size as usize {
            return invalid(FORMAT, format!("{}: truncated LZSS data", entry.name));
        }
        Ok(data)
    }

    pub fn read_name(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self
            .find(name)
            .ok_or_else(|| Error::NotFound(name.to_string()))?;
        self.read(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trip() {
        let data = build(&[("FK001.MES", b"abc"), ("ロゴ.AKB", b"0123456789")]);
        let entries = parse_index(&data, data.len() as u64).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "FK001.MES");
        assert_eq!(entries[1].name, "ロゴ.AKB");
        let e = &entries[1];
        assert_eq!(&data[e.offset as usize..][..e.size as usize], b"0123456789");
    }

    #[test]
    fn rejects_entry_outside_file() {
        let mut data = build(&[("A", b"abcd")]);
        data.truncate(data.len() - 1);
        assert!(parse_index(&data, data.len() as u64).is_err());
    }
}
