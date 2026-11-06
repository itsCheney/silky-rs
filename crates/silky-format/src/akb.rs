//! AKB images.
//!
//! ```text
//! "AKB " or "AKB+"
//! u16 width, u16 height
//! u32 flags            0x40000000: 24 bpp (else 32), 0x80000000: alpha flag
//!                      (32 bpp images carry alpha either way: interface
//!                      sheets of きまぐれテンプテーション2 such as
//!                      `log_p.akb` have it without the flag)
//! [u8; 4] background   BGR(A) fill colour, 0 means transparent black
//! i32 left, top, right, bottom   rectangle that the pixel data covers
//! "AKB+" only: [u8; 32] base image name (NUL padded)
//! LZSS data: the rectangle, rows bottom-up, BGR(A), delta coded
//!            (each byte adds the byte one pixel left; then each row adds the
//!            row above)
//! ```
//!
//! An `AKB+` image is a difference on top of its base image: pixels equal to
//! pure green (B=0, G=255, R=0) keep the base.

use crate::error::{invalid, Result};
use crate::lzss;

const FORMAT: &str = "AKB";
const FLAG_24BPP: u32 = 0x4000_0000;
#[cfg(test)]
const FLAG_ALPHA: u32 = 0x8000_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub width: u32,
    pub height: u32,
    pub flags: u32,
    pub background: [u8; 4],
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// Name of the base image for `AKB+` difference images.
    pub base: Option<String>,
    pub data_offset: usize,
}

impl Header {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return invalid(FORMAT, "file shorter than its header");
        }
        let diff = match &data[..4] {
            b"AKB " => false,
            b"AKB+" => true,
            _ => return invalid(FORMAT, "bad signature"),
        };
        let u16_at = |o: usize| u16::from_le_bytes([data[o], data[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let i32_at = |o: usize| u32_at(o) as i32;
        let mut header = Header {
            width: u32::from(u16_at(4)),
            height: u32::from(u16_at(6)),
            flags: u32_at(8),
            background: data[12..16].try_into().unwrap(),
            left: i32_at(16),
            top: i32_at(20),
            right: i32_at(24),
            bottom: i32_at(28),
            base: None,
            data_offset: 32,
        };
        if diff {
            let Some(name) = data.get(32..64) else {
                return invalid(FORMAT, "difference image without base name");
            };
            let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            header.base = Some(crate::decode_cp932(&name[..len]));
            header.data_offset = 64;
        }
        let (w, h) = (header.width as i32, header.height as i32);
        if header.left < 0
            || header.top < 0
            || header.right < header.left
            || header.bottom < header.top
            || header.right > w
            || header.bottom > h
        {
            return invalid(FORMAT, "pixel rectangle outside the image");
        }
        Ok(header)
    }

    pub fn bytes_per_pixel(&self) -> usize {
        if self.flags & FLAG_24BPP != 0 {
            3
        } else {
            4
        }
    }

    pub fn has_alpha(&self) -> bool {
        self.bytes_per_pixel() == 4
    }
}

/// A decoded image, RGBA8, rows top-down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decodes an AKB image. `base` must be the decoded base image when the
/// header names one; it is ignored otherwise.
pub fn decode(data: &[u8], base: Option<&Image>) -> Result<Image> {
    let header = Header::parse(data)?;
    let (w, h) = (header.width as usize, header.height as usize);
    let ps = header.bytes_per_pixel();
    let alpha = header.has_alpha();
    let iw = (header.right - header.left) as usize;
    let ih = (header.bottom - header.top) as usize;

    let mut image = match (&header.base, base) {
        (Some(name), Some(base)) => {
            if base.width != header.width || base.height != header.height {
                return invalid(FORMAT, format!("base image {name} has another size"));
            }
            base.clone()
        }
        (Some(name), None) => return invalid(FORMAT, format!("base image {name} missing")),
        (None, _) => {
            let bg = header.background;
            let px = if u32::from_le_bytes(bg) == 0 {
                [0, 0, 0, 0]
            } else {
                [bg[2], bg[1], bg[0], if alpha { bg[3] } else { 255 }]
            };
            Image {
                width: header.width,
                height: header.height,
                rgba: px.repeat(w * h),
            }
        }
    };
    if iw == 0 || ih == 0 {
        return Ok(image);
    }

    let stride = iw * ps;
    let raw = lzss::decompress(&data[header.data_offset..], stride * ih);
    if raw.len() != stride * ih {
        return invalid(FORMAT, "truncated pixel data");
    }
    // Rows are stored bottom-up. Put them top-down first: the delta coding
    // works on the top-down image (first row horizontally, then each row adds
    // the row above).
    let mut px = vec![0u8; raw.len()];
    for r in 0..ih {
        px[(ih - 1 - r) * stride..][..stride].copy_from_slice(&raw[r * stride..][..stride]);
    }
    for i in ps..stride {
        px[i] = px[i].wrapping_add(px[i - ps]);
    }
    for i in stride..px.len() {
        px[i] = px[i].wrapping_add(px[i - stride]);
    }

    let diff = header.base.is_some();
    for row in 0..ih {
        let src = &px[row * stride..][..stride];
        let y = header.top as usize + row;
        for x in 0..iw {
            let s = &src[x * ps..][..ps];
            if diff && s[0] == 0 && s[1] == 0xff && s[2] == 0 {
                continue;
            }
            let d = ((y * w) + header.left as usize + x) * 4;
            image.rgba[d] = s[2];
            image.rgba[d + 1] = s[1];
            image.rgba[d + 2] = s[0];
            image.rgba[d + 3] = if alpha { s[3] } else { 255 };
        }
    }
    Ok(image)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Encodes BGR(A) pixels of the rectangle (top-down) as AKB data.
    pub fn encode(
        width: u16,
        height: u16,
        flags: u32,
        rect: [i32; 4],
        top_down: &[u8],
        base: Option<&str>,
    ) -> Vec<u8> {
        let ps = if flags & FLAG_24BPP != 0 { 3 } else { 4 };
        let iw = (rect[2] - rect[0]) as usize;
        let ih = (rect[3] - rect[1]) as usize;
        let stride = iw * ps;
        let mut delta = top_down[..stride * ih].to_vec();
        for i in (stride..delta.len()).rev() {
            delta[i] = top_down[i].wrapping_sub(top_down[i - stride]);
        }
        for i in (ps..stride).rev() {
            delta[i] = top_down[i].wrapping_sub(top_down[i - ps]);
        }
        let mut rows = Vec::new();
        for r in (0..ih).rev() {
            rows.extend_from_slice(&delta[r * stride..][..stride]);
        }
        let delta = rows;
        let mut out = Vec::new();
        out.extend_from_slice(if base.is_some() { b"AKB+" } else { b"AKB " });
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        for v in rect {
            out.extend_from_slice(&v.to_le_bytes());
        }
        if let Some(name) = base {
            let mut b = [0u8; 32];
            b[..name.len()].copy_from_slice(name.as_bytes());
            out.extend_from_slice(&b);
        }
        out.extend(lzss::store(&delta));
        out
    }

    #[test]
    fn decodes_24bpp_full_image() {
        // 2x2: red, green / blue, white (BGR order in the file).
        let px = [0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255];
        let data = encode(2, 2, FLAG_24BPP, [0, 0, 2, 2], &px, None);
        let img = decode(&data, None).unwrap();
        assert_eq!(
            img.rgba,
            [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn decodes_sprite_rectangle_with_alpha() {
        let px = [10, 20, 30, 128];
        let data = encode(3, 2, FLAG_ALPHA, [1, 1, 2, 2], &px, None);
        let img = decode(&data, None).unwrap();
        assert_eq!(img.rgba.len(), 3 * 2 * 4);
        assert_eq!(&img.rgba[(3 + 1) * 4..][..4], &[30, 20, 10, 128]);
        assert_eq!(&img.rgba[..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn difference_keeps_base_under_green() {
        let base_px = [1, 1, 1, 2, 2, 2];
        let base = decode(
            &encode(2, 1, FLAG_24BPP, [0, 0, 2, 1], &base_px, None),
            None,
        )
        .unwrap();
        let diff_px = [0, 255, 0, 9, 9, 9];
        let data = encode(2, 1, FLAG_24BPP, [0, 0, 2, 1], &diff_px, Some("BASE.AKB"));
        assert_eq!(
            Header::parse(&data).unwrap().base.as_deref(),
            Some("BASE.AKB")
        );
        assert!(decode(&data, None).is_err());
        let img = decode(&data, Some(&base)).unwrap();
        assert_eq!(img.rgba, [1, 1, 1, 255, 9, 9, 9, 255]);
    }
}
