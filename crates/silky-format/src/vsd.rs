//! `.VSD` movies: a `VSD1` header in front of an MPEG-1 program stream
//! (MPEG-1 video, MP2 audio).

use crate::error::{invalid, Result};

/// Returns the byte range of the MPEG stream inside a VSD file, given its
/// first 8 bytes and total length.
pub fn stream_range(head: &[u8], file_len: u64) -> Result<std::ops::Range<u64>> {
    if head.len() < 8 || &head[..4] != b"VSD1" {
        return invalid("VSD", "bad signature");
    }
    let skip = u64::from(u32::from_le_bytes(head[4..8].try_into().unwrap()));
    let start = 8 + skip;
    if start > file_len {
        return invalid("VSD", "header longer than the file");
    }
    Ok(start..file_len)
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_stream() {
        let head = [b'V', b'S', b'D', b'1', 0x10, 0, 0, 0];
        assert_eq!(super::stream_range(&head, 100).unwrap(), 24..100);
        assert!(super::stream_range(b"RIFF\0\0\0\0", 100).is_err());
    }
}
