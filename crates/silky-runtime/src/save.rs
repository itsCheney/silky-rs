//! Save files: one JSON file per slot in a folder chosen by the frontend.
//!
//! These are this runtime's own saves; the original engine's save files
//! are not read.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Result, SaveState};

/// Bumped when the layout changes in a way older files cannot be read.
const VERSION: u32 = 1;
/// Slot of the quick save.
pub const QUICK: &str = "quick";

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

    pub fn write(&self, slot: &str, state: SaveState) -> Result<()> {
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
        std::fs::rename(&tmp, &path).map_err(io_error(&path))
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

    /// The next free numbered slot (`save001`, `save002`, ...).
    pub fn next_slot(&self) -> String {
        let used = self.list();
        (1..)
            .map(|n| format!("save{n:03}"))
            .find(|s| !used.contains(s))
            .unwrap()
    }
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |e| Error::Invalid(format!("{}: {e}", path.display()))
}
