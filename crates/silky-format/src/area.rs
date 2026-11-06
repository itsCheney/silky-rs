//! `.AREA` files: clickable rectangles of a screen, such as the buttons of
//! the message window (`mes_o.area`) or the title menu (`title.area`).
//!
//! ```text
//! u32 count
//! count times: i32 id, i32 left, i32 top, i32 right, i32 bottom
//! ```
//!
//! Coordinates are screen pixels; `right` and `bottom` are exclusive.

use crate::error::{invalid, Result};

const FORMAT: &str = "AREA";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub id: i32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Area {
    pub fn width(&self) -> u32 {
        (self.right - self.left).max(0) as u32
    }

    pub fn height(&self) -> u32 {
        (self.bottom - self.top).max(0) as u32
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

pub fn parse(data: &[u8]) -> Result<Vec<Area>> {
    let Some(head) = data.get(..4) else {
        return invalid(FORMAT, "no header");
    };
    let count = u32::from_le_bytes(head.try_into().unwrap()) as usize;
    let body = &data[4..];
    if body.len() < count * 20 {
        return invalid(
            FORMAT,
            format!("{count} rectangles in {} bytes", body.len()),
        );
    }
    Ok(body
        .chunks_exact(20)
        .take(count)
        .map(|r| {
            let v = |i: usize| i32::from_le_bytes(r[i * 4..i * 4 + 4].try_into().unwrap());
            Area {
                id: v(0),
                left: v(1),
                top: v(2),
                right: v(3),
                bottom: v(4),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rectangles() {
        let mut d = 1u32.to_le_bytes().to_vec();
        for v in [13i32, 1167, 677, 1208, 720] {
            d.extend(v.to_le_bytes());
        }
        let a = parse(&d).unwrap();
        assert_eq!(a[0].width(), 41);
        assert!(a[0].contains(1167, 719) && !a[0].contains(1208, 700));
        assert!(parse(&d[..10]).is_err());
    }
}
