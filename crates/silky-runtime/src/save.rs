//! Save files: one JSON file per slot in a folder chosen by the frontend.
//!
//! These are this runtime's own saves; the original engine's save files
//! are not read. Each save may have a thumbnail next to it (`<slot>.png`,
//! 128 x 72).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use silky_format::akb::Image;

use crate::{Error, Result, SaveState};

/// Bumped when the layout changes in a way older files cannot be read.
const VERSION: u32 = 1;
/// Slot of the quick save.
pub const QUICK: &str = "quick";
/// Auto saves (`auto01`..) kept before the oldest is replaced.
pub const AUTO_SLOTS: u32 = 10;
/// Thumbnail size.
pub const THUMB_SIZE: (u32, u32) = (128, 72);

/// The three kinds of save, as the tabs of the save screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `save001`, `save002`, ...: made by the player.
    Manual,
    /// `auto01`..: made when a selection comes up.
    Auto,
    /// `quick`: the quick save button.
    Quick,
}

impl Kind {
    /// Kind and number of a slot name.
    pub fn of(slot: &str) -> Option<(Kind, u32)> {
        if slot == QUICK {
            return Some((Kind::Quick, 1));
        }
        let (kind, n) = if let Some(n) = slot.strip_prefix("save") {
            (Kind::Manual, n)
        } else {
            (Kind::Auto, slot.strip_prefix("auto")?)
        };
        Some((kind, n.parse().ok()?))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveFile {
    pub version: u32,
    /// Seconds since 1970.
    pub saved_at: u64,
    /// The message shown when saving, for a save list.
    pub text: Option<String>,
    pub state: SaveState,
}

/// The save folder of one game.
#[derive(Debug, Clone)]
pub struct Saves {
    dir: PathBuf,
}

impl Saves {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, slot: &str) -> PathBuf {
        self.dir.join(format!("{slot}.json"))
    }

    fn thumb_path(&self, slot: &str) -> PathBuf {
        self.dir.join(format!("{slot}.png"))
    }

    /// Writes a save, with a thumbnail if one is given.
    pub fn write(&self, slot: &str, state: SaveState, thumb: Option<&Image>) -> Result<()> {
        let file = SaveFile {
            version: VERSION,
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            text: state.scene.text.clone(),
            state,
        };
        std::fs::create_dir_all(&self.dir).map_err(io_error(&self.dir))?;
        let json = serde_json::to_vec(&file).map_err(|e| Error::Invalid(e.to_string()))?;
        // Write to a temporary file first so a crash never leaves half a save.
        let path = self.path(slot);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json).map_err(io_error(&tmp))?;
        let thumb_path = self.thumb_path(slot);
        match thumb {
            Some(img) => {
                let png = encode_png(img)?;
                std::fs::write(&thumb_path, png).map_err(io_error(&thumb_path))?;
            }
            None => {
                let _ = std::fs::remove_file(&thumb_path);
            }
        }
        std::fs::rename(&tmp, &path).map_err(io_error(&path))
    }

    /// The thumbnail of a save, if it has one.
    pub fn thumb(&self, slot: &str) -> Option<Image> {
        decode_png(&std::fs::read(self.thumb_path(slot)).ok()?)
    }

    /// Removes a save.
    pub fn delete(&self, slot: &str) -> Result<()> {
        let _ = std::fs::remove_file(self.thumb_path(slot));
        let path = self.path(slot);
        std::fs::remove_file(&path).map_err(io_error(&path))
    }

    /// Removes every save of the folder.
    pub fn delete_all(&self) -> Result<()> {
        for slot in self.list() {
            self.delete(&slot)?;
        }
        Ok(())
    }

    pub fn read(&self, slot: &str) -> Result<SaveFile> {
        let path = self.path(slot);
        let data = std::fs::read(&path).map_err(io_error(&path))?;
        let file: SaveFile = serde_json::from_slice(&data)
            .map_err(|e| Error::Invalid(format!("{}: {e}", path.display())))?;
        if file.version != VERSION {
            return Err(Error::Invalid(format!(
                "{}: save version {} (expected {VERSION})",
                path.display(),
                file.version
            )));
        }
        Ok(file)
    }

    /// Slots with a save, newest first.
    pub fn list(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut slots: Vec<(SystemTime, String)> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let slot = name.strip_suffix(".json")?.to_string();
                let time = e.metadata().and_then(|m| m.modified()).ok()?;
                Some((time, slot))
            })
            .collect();
        slots.sort_by_key(|s| std::cmp::Reverse(s.0));
        slots.into_iter().map(|s| s.1).collect()
    }

    /// Slots of one kind, newest first.
    pub fn list_kind(&self, kind: Kind) -> Vec<String> {
        self.list()
            .into_iter()
            .filter(|s| Kind::of(s).is_some_and(|k| k.0 == kind))
            .collect()
    }

    /// The next free numbered slot (`save001`, `save002`, ...).
    pub fn next_slot(&self) -> String {
        let used = self.list();
        (1..)
            .map(|n| format!("save{n:03}"))
            .find(|s| !used.contains(s))
            .unwrap()
    }

    /// The auto save slot to write next: a free one, else the oldest.
    pub fn next_auto_slot(&self) -> String {
        let used = self.list_kind(Kind::Auto);
        (1..=AUTO_SLOTS)
            .map(|n| format!("auto{n:02}"))
            .find(|s| !used.contains(s))
            .or_else(|| used.last().cloned())
            .unwrap_or_else(|| "auto01".into())
    }
}

/// Shrinks a frame to a thumbnail by averaging blocks of pixels.
pub fn thumbnail(rgba: &[u8], width: u32, height: u32) -> Image {
    let (tw, th) = THUMB_SIZE;
    let mut out = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let (y0, y1) = (
            ty * height / th,
            ((ty + 1) * height / th).max(ty * height / th + 1),
        );
        for tx in 0..tw {
            let (x0, x1) = (
                tx * width / tw,
                ((tx + 1) * width / tw).max(tx * width / tw + 1),
            );
            let mut sum = [0u32; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = ((y * width + x) * 4) as usize;
                    for k in 0..3 {
                        sum[k] += rgba[i + k] as u32;
                    }
                }
            }
            let n = (x1 - x0) * (y1 - y0);
            let o = ((ty * tw + tx) * 4) as usize;
            for k in 0..3 {
                out[o + k] = (sum[k] / n) as u8;
            }
            out[o + 3] = 255;
        }
    }
    Image {
        width: tw,
        height: th,
        rgba: out,
    }
}

fn encode_png(img: &Image) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let err = |e: png::EncodingError| Error::Invalid(format!("thumbnail: {e}"));
    let mut w = enc.write_header().map_err(err)?;
    w.write_image_data(&img.rgba).map_err(err)?;
    w.finish().map_err(err)?;
    Ok(out)
}

fn decode_png(data: &[u8]) -> Option<Image> {
    let mut dec = png::Decoder::new(data);
    dec.set_transformations(png::Transformations::EXPAND);
    let mut r = dec.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return None,
    };
    Some(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |e| Error::Invalid(format!("{}: {e}", path.display()))
}
