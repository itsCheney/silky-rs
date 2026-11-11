//! A game folder: its archives grouped by kind, as listed in `Patch.xml`
//! (international builds) or `FileInfo.xml` (Japanese builds).
//!
//! The XML names one or more archives per kind, e.g.
//! `<ScriptArchive><File>Script.arc;Script1.arc</File></ScriptArchive>`.
//! Later archives are newer and win when both hold the same name. Without
//! either file, the usual names (`script.arc`, then `script1.arc`) are used.
//! `update.arc`, which the engine also opens, is read as the newest data
//! archive (it holds E-mote motions added by patches).

use std::fs;
use std::path::{Path, PathBuf};

use crate::arc::{Archive, Entry};
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
    /// ふゆから、くるる。 (2018 international build).
    Fuyukuru,
    /// きまぐれテンプテーション2 (2024).
    Kimaten2,
}

impl Title {
    /// From the program file in the folder; ふゆから、くるる。 otherwise.
    fn detect(root: &Path) -> Self {
        if find_file(root, "kimaten2.exe").is_some() {
            Title::Kimaten2
        } else {
            Title::Fuyukuru
        }
    }
}

pub struct Game {
    pub root: PathBuf,
    sets: Vec<(Kind, ArchiveSet)>,
    pub title: Title,
}

impl Game {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let patch = find_file(&root, "Patch.xml")
            .or_else(|| find_file(&root, "FileInfo.xml"))
            .map(fs::read)
            .transpose()?;
        let patch = patch.map(|b| crate::decode_cp932(&b));
        let mut sets = Vec::new();
        for kind in Kind::ALL {
            let names = match &patch {
                Some(xml) => archive_names(xml, kind),
                None => vec![
                    format!("{}.arc", kind.name()),
                    format!("{}1.arc", kind.name()),
                ],
            };
            let mut archives = Vec::new();
            let update = (kind == Kind::Data).then(|| "update.arc".to_string());
            for name in names.into_iter().chain(update) {
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
        let title = Title::detect(&root);
        Ok(Self { title, root, sets })
    }

    /// The archives of one kind.
    pub fn set(&self, kind: Kind) -> &ArchiveSet {
        &self.sets.iter().find(|(k, _)| *k == kind).unwrap().1
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
