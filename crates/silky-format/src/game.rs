//! A game folder: its archives grouped by kind, as listed in `Patch.xml`
//! (international builds) or `FileInfo.xml` (Japanese builds).
//!
//! The XML names one or more archives per kind, e.g.
//! `<ScriptArchive><File>Script.arc;Script1.arc</File></ScriptArchive>`.
//! Later archives are newer and win when both hold the same name. Without
//! either file, the usual names (`script.arc`, then `script1.arc`) are used.
//! The "eomaia" engine has neither file. It lists the archives in
//! `updateInfo.json` (else `systeminfo.json`) under `GameFile`, per kind
//! (`script`, `image`, `data`, `music`, `effect`, `voice`, `media`), newest
//! first.
//! `update.arc`, which the engine also opens, is read as the newest
//! archive of every kind: it holds E-mote motions added by patches, and in
//! 同級生 Remake newer scripts, text tables and pictures.

use std::fs;
use std::path::{Path, PathBuf};

use crate::arc::{Archive, Entry};
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::mes::Script;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Script,
    Layer,
    Data,
    Movie,
    Music,
    Effect,
    Voice,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Script,
        Kind::Layer,
        Kind::Data,
        Kind::Movie,
        Kind::Music,
        Kind::Effect,
        Kind::Voice,
    ];

    /// Element name in `Patch.xml` and base name of the default archive.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Script => "Script",
            Kind::Layer => "Layer",
            Kind::Data => "Data",
            Kind::Movie => "Movie",
            Kind::Music => "Music",
            Kind::Effect => "Effect",
            Kind::Voice => "Voice",
        }
    }
}

/// Archives of one kind, newest last.
pub struct ArchiveSet {
    archives: Vec<Archive>,
}

impl ArchiveSet {
    pub fn archives(&self) -> &[Archive] {
        &self.archives
    }

    /// Finds a file, newest archive first.
    pub fn find(&self, name: &str) -> Option<(&Archive, &Entry)> {
        self.archives
            .iter()
            .rev()
            .find_map(|a| a.find(name).map(|e| (a, e)))
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let (archive, entry) = self
            .find(name)
            .ok_or_else(|| Error::NotFound(name.to_string()))?;
        archive.read(entry)
    }

    /// Every file name once, as seen through the overrides.
    pub fn names(&self) -> Vec<String> {
        let mut seen = std::collections::BTreeMap::new();
        for a in &self.archives {
            for e in a.entries() {
                seen.insert(e.name.to_ascii_lowercase(), e.name.clone());
            }
        }
        seen.into_values().collect()
    }
}

/// Game a folder holds. The engine is shared, but each game adds its own
/// commands (native class 29), so the runtime needs to know which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Title {
    /// ふゆから、くるる。
    Fuyukuru,
    /// きまぐれテンプテーション2.
    Kimaten2,
    /// 景の海のアペイリア.
    Apeiria,
    /// 同級生 Remake.
    Doukyuusei,
    /// A game not known yet: it gets the shared engine and the common
    /// `Game` commands only.
    Unknown,
}

impl Title {
    /// From the name of the program file in the folder, with that file.
    fn detect(root: &Path) -> (Self, Option<PathBuf>) {
        for (exe, title) in [
            ("kimaten2.exe", Title::Kimaten2),
            ("apeiria.exe", Title::Apeiria),
            ("fuyukuru.exe", Title::Fuyukuru),
            ("doukyuusei.exe", Title::Doukyuusei),
        ] {
            if let Some(path) = find_file(root, exe) {
                return (title, Some(path));
            }
        }
        (Title::Unknown, None)
    }
}

pub struct Game {
    pub root: PathBuf,
    sets: Vec<(Kind, ArchiveSet)>,
    pub title: Title,
    /// The engine build of the folder's program.
    pub engine: Engine,
    update: bool,
}

impl Game {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let patch = find_file(&root, "Patch.xml")
            .or_else(|| find_file(&root, "FileInfo.xml"))
            .map(fs::read)
            .transpose()?;
        let patch = patch.map(|b| crate::decode_cp932(&b));
        let json = if patch.is_some() {
            None
        } else {
            find_file(&root, "updateInfo.json")
                .or_else(|| find_file(&root, "systeminfo.json"))
                .map(fs::read)
                .transpose()?
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        };
        let mut sets = Vec::new();
        for kind in Kind::ALL {
            let names = match (&patch, &json) {
                (Some(xml), _) => archive_names(xml, kind),
                (None, Some(json)) => json_archive_names(json, kind),
                (None, None) => vec![
                    format!("{}.arc", kind.name()),
                    format!("{}1.arc", kind.name()),
                ],
            };
            let mut archives = Vec::new();
            for name in names.into_iter().chain(["update.arc".to_string()]) {
                if let Some(path) = find_file(&root, &name) {
                    archives.push(Archive::open(path)?);
                }
            }
            sets.push((kind, ArchiveSet { archives }));
        }
        if sets.iter().all(|(_, s)| s.archives.is_empty()) {
            return Err(Error::NotFound(format!(
                "no Silky's archives in {}",
                root.display()
            )));
        }
        let (title, exe) = Title::detect(&root);
        let engine = Engine::of(exe.as_deref());
        let update = find_file(&root, "update.arc").is_some();
        Ok(Self {
            update,
            title,
            engine,
            root,
            sets,
        })
    }

    /// The archives of one kind.
    pub fn set(&self, kind: Kind) -> &ArchiveSet {
        &self.sets.iter().find(|(k, _)| *k == kind).unwrap().1
    }

    /// Whether `update.arc` is in the folder (and so read as the newest
    /// archive). 同級生 Remake takes the update archive's variants of its
    /// pictures from this, not from the file list.
    pub fn has_update_archive(&self) -> bool {
        self.update
    }

    /// A file from the archives of `kind`.
    pub fn read(&self, kind: Kind, name: &str) -> Result<Vec<u8>> {
        self.set(kind).read(name)
    }

    pub fn exists(&self, kind: Kind, name: &str) -> bool {
        self.set(kind).find(name).is_some()
    }

    pub fn script(&self, name: &str) -> Result<Script> {
        Script::parse(&self.set(Kind::Script).read(name)?)
    }

    /// Whether the text is in `.MAP` tables (international builds) rather
    /// than in the scripts.
    pub fn has_text_tables(&self) -> bool {
        self.set(Kind::Data).archives().iter().any(|a| {
            a.entries()
                .iter()
                .any(|e| e.name.to_ascii_lowercase().ends_with(".map"))
        })
    }
}

/// Archive file names of one kind from `Patch.xml`.
pub fn archive_names(xml: &str, kind: Kind) -> Vec<String> {
    let open = format!("<{}Archive>", kind.name());
    let close = format!("</{}Archive>", kind.name());
    let Some(start) = xml.find(&open) else {
        return Vec::new();
    };
    let body = &xml[start + open.len()..];
    let body = &body[..body.find(&close).unwrap_or(body.len())];
    let mut names = Vec::new();
    let mut rest = body;
    while let Some(s) = rest.find("<File>") {
        let after = &rest[s + 6..];
        let e = after.find("</File>").unwrap_or(after.len());
        names.extend(
            after[..e]
                .split(';')
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(String::from),
        );
        rest = &after[e..];
    }
    names
}

/// Archive file names of one kind from `updateInfo.json` or
/// `systeminfo.json`, oldest first (the file lists the newest first).
pub fn json_archive_names(json: &str, kind: Kind) -> Vec<String> {
    let key = match kind {
        Kind::Script => "script",
        Kind::Layer => "image",
        Kind::Data => "data",
        Kind::Music => "music",
        Kind::Effect => "effect",
        Kind::Voice => "voice",
        Kind::Movie => "media",
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json.trim_start_matches('\u{feff}'))
    else {
        return Vec::new();
    };
    match &root["GameFile"][key]["Archive"] {
        serde_json::Value::String(name) => vec![name.clone()],
        serde_json::Value::Array(list) => list
            .iter()
            .rev()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// Finds a file in `dir` ignoring ASCII case (the games come from Windows).
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_patch_xml() {
        let xml = "<GameFile>\n<ScriptArchive>\n\t<File>Script.arc;Script1.arc</File>\n</ScriptArchive>\n\
                   <LayerArchive><File>layer.arc</File></LayerArchive></GameFile>";
        assert_eq!(
            archive_names(xml, Kind::Script),
            ["Script.arc", "Script1.arc"]
        );
        assert_eq!(archive_names(xml, Kind::Layer), ["layer.arc"]);
        assert!(archive_names(xml, Kind::Voice).is_empty());
    }

    #[test]
    fn reads_update_info() {
        let json = r#"{"GameFile": {
            "script": {"Archive": ["script1.arc", "script.arc"], "Enable": true},
            "image": {"Archive": ["image1.arc", "image.arc"]},
            "media": {"Archive": "media.arc"}}}"#;
        assert_eq!(
            json_archive_names(json, Kind::Script),
            ["script.arc", "script1.arc"]
        );
        assert_eq!(
            json_archive_names(json, Kind::Layer),
            ["image.arc", "image1.arc"]
        );
        assert_eq!(json_archive_names(json, Kind::Movie), ["media.arc"]);
        assert!(json_archive_names(json, Kind::Voice).is_empty());
        assert!(json_archive_names("not json", Kind::Data).is_empty());
    }

    #[test]
    fn newer_archive_wins() {
        let dir = std::env::temp_dir().join(format!("silky-game-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("script.arc"),
            crate::arc::build(&[("A.MES", b"old"), ("B.MES", b"b")]),
        )
        .unwrap();
        fs::write(
            dir.join("SCRIPT1.ARC"),
            crate::arc::build(&[("A.MES", b"new")]),
        )
        .unwrap();
        let game = Game::open(&dir).unwrap();
        let scripts = game.set(Kind::Script);
        assert_eq!(scripts.read("a.mes").unwrap(), b"new");
        assert_eq!(scripts.read("B.MES").unwrap(), b"b");
        assert_eq!(scripts.names(), ["A.MES", "B.MES"]);
        fs::remove_dir_all(&dir).unwrap();
    }
}
