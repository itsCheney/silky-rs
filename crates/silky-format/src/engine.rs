//! Builds of the engine.
//!
//! Every game ships its own copy of the engine linked into its program, and
//! the engine classes (1 to 28) changed between those copies: a call that
//! returns a value in one build returns nothing in another. A [`Build`] is
//! one program file, known by its link time stamp and size; its
//! [`Profile`] lists what its engine classes do where builds differ.
//! `docs/builds/` has a page for each profile.

use std::fs;
use std::io::Read;
use std::path::Path;

/// What the engine classes of one build do where builds differ.
#[derive(Debug, PartialEq, Eq)]
pub struct Profile {
    /// Name of the profile, also the name of its page in `docs/builds/`.
    pub name: &'static str,
    /// `Sprite.1`, the other `Sprite` calls, `Flag` and `Mouse.4` work as
    /// read on the 2026 build. Off where that was not checked: the calls
    /// are then reported as not carried out.
    pub sprites_and_flags: bool,
    /// A layer `Sprite.1` creates hidden, as a store of button sheets the
    /// game's screens draw in cells; it never shows.
    pub sheet_store: Option<i32>,
    /// `BackLog.3` without arguments ends the voice mark of the entry.
    pub backlog_voice_end: bool,
    /// `Movie.0` takes the file alone, `Movie.3` plays the movie and
    /// returns whether it played, `Movie.2` closes it and returns nothing.
    /// Later builds take (channel, file) and play with `Movie.2`.
    pub movie_2_closes: bool,
    /// `HEffect` (class 17) plays sounds like `Effect`.
    pub heffect_sounds: bool,
    /// Engine calls whose result differs from the common list: (class,
    /// sub, returns a value).
    pub results: &'static [(i32, i32, bool)],
}

/// The engine of 景の海のアペイリア's program (2017, Visual Studio 2013).
pub const ENGINE_2017: Profile = Profile {
    name: "engine-2017",
    sprites_and_flags: true,
    sheet_store: Some(5),
    backlog_voice_end: true,
    movie_2_closes: true,
    heffect_sounds: true,
    results: &[
        (23, 2, false),
        (23, 3, true),
        (22, 4, true),
        (22, 5, true),
        (22, 6, true),
        (22, 7, true),
        (24, 5, true),
        (27, 3, true),
        (27, 8, true),
    ],
};

/// The engine of the ふゆから、くるる。 package (2021, Visual Studio 2019).
/// Like [`ENGINE_2026`] except for the movie calls, which are still those
/// of 2017.
pub const ENGINE_2021: Profile = Profile {
    name: "engine-2021",
    movie_2_closes: true,
    results: &[(23, 2, false), (23, 3, true)],
    ..ENGINE_2026
};

/// The engine of the ふゆから、くるる。 download program, linked again in
/// 2026 (Visual Studio 2022). The engine part of the runtime was first
/// written against it, and games not known yet get it.
pub const ENGINE_2026: Profile = Profile {
    name: "engine-2026",
    sprites_and_flags: true,
    sheet_store: None,
    backlog_voice_end: false,
    movie_2_closes: false,
    heffect_sounds: false,
    results: &[],
};

/// The engine of 同級生 Remake's program (2022). Its screen is 1920 x
/// 1080. Not checked call by call yet; its scripts use the movie calls of
/// 2017 (they keep what `Movie.3` returns).
pub const ENGINE_2022: Profile = Profile {
    name: "engine-2022",
    sprites_and_flags: false,
    movie_2_closes: true,
    results: &[(23, 2, false), (23, 3, true)],
    ..ENGINE_2026
};

/// The engine of きまぐれテンプテーション2's program (2024, Visual Studio
/// 2019). Its movie calls are those of [`ENGINE_2026`].
pub const ENGINE_2024: Profile = Profile {
    name: "engine-2024",
    sprites_and_flags: false,
    ..ENGINE_2026
};

/// The engine of 先輩が私の妄想にドージンする's program (2024-08, Visual
/// Studio 2019): the results, voice end and `HEffect` of [`ENGINE_2017`]
/// with the movie calls and `Sprite.1` of [`ENGINE_2026`].
pub const ENGINE_2024_08: Profile = Profile {
    name: "engine-2024-08",
    sprites_and_flags: true,
    sheet_store: None,
    backlog_voice_end: true,
    movie_2_closes: false,
    heffect_sounds: true,
    results: &[
        (22, 4, true),
        (22, 5, true),
        (22, 6, true),
        (22, 7, true),
        (24, 5, true),
        (27, 3, true),
    ],
};

/// One program file.
#[derive(Debug, PartialEq, Eq)]
pub struct Build {
    /// Program file name.
    pub exe: &'static str,
    /// Version in its version resource.
    pub version: &'static str,
    /// Link time stamp of the PE header (seconds since 1970, UTC).
    pub linked: u32,
    /// File size in bytes.
    pub size: u64,
    /// SHA-256 of the file, for the docs and for checking a copy by hand.
    pub sha256: &'static str,
    pub profile: &'static Profile,
}

/// Every build checked.
pub const BUILDS: &[Build] = &[
    // なないろリンカネーション. Size and SHA-256 not recorded yet: the copy
    // read was not the file as shipped.
    Build {
        exe: "nanairo.exe",
        version: "1.0.0.2",
        linked: 1410290840, // 2014-09-09 19:27:20
        size: 0,
        sha256: "",
        profile: &ENGINE_2017,
    },
    Build {
        exe: "apeiria.exe",
        version: "1.0.3.0",
        linked: 1510211209, // 2017-11-09 07:06:49
        size: 1665536,
        sha256: "dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e",
        profile: &ENGINE_2017,
    },
    Build {
        exe: "casablanca.exe",
        version: "1.0.0.0",
        linked: 1512917527, // 2017-12-10 14:52:07
        size: 1559040,
        sha256: "4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716",
        profile: &ENGINE_2017,
    },
    // Package, as installed.
    Build {
        exe: "fuyukuru.exe",
        version: "1.0.0.0",
        linked: 1630961547, // 2021-09-06 20:52:27
        size: 1948672,
        sha256: "9797a7cb71955c0ddb4b4865e0e592166f82386a5e642bd25072922cf9d6d9fa",
        profile: &ENGINE_2021,
    },
    // Package with the official update 1.01.
    Build {
        exe: "fuyukuru.exe",
        version: "1.0.1.0",
        linked: 1632894466, // 2021-09-29 05:47:46
        size: 2935808,
        sha256: "b5b163ceb979d9825c6586401da005d51a57f4005a93cf227ca304bf9e831988",
        profile: &ENGINE_2021,
    },
    // Download, Japanese, English and Chinese.
    Build {
        exe: "fuyukuru.exe",
        version: "1.0.1.0",
        linked: 1773843162, // 2026-03-18 14:12:42
        size: 3117568,
        sha256: "eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707",
        profile: &ENGINE_2026,
    },
    // Steam release; the program carries Steam's DRM wrapper.
    Build {
        exe: "doukyuusei.exe",
        version: "1.0.4.0",
        linked: 1647330885, // 2022-03-15 07:54:45
        size: 11159880,
        sha256: "9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451",
        profile: &ENGINE_2022,
    },
    // Package with the official update 1.04.
    Build {
        exe: "kimaten.exe",
        version: "1.0.4.0",
        linked: 1576471729, // 2019-12-16 04:48:49
        size: 2039808,
        sha256: "36148a0286bd9d31ceaec4323f4be987319bd759e68b5961c59c992b07b54700",
        profile: &ENGINE_2017,
    },
    // 先輩が私の妄想にドージンする. Size and SHA-256 not recorded yet:
    // the copy read was not the file as shipped. The time stamp is that of
    // the program; the version is its version resource.
    Build {
        exe: "doujin.exe",
        version: "1.0.1.0",
        linked: 1725003615, // 2024-08-30 07:40:15
        size: 0,
        sha256: "",
        profile: &ENGINE_2024_08,
    },
    Build {
        exe: "kimaten2.exe",
        version: "1.0.0.0",
        linked: 1728954791, // 2024-10-15 01:13:11
        size: 3038720,
        sha256: "5cbd99107b94befe4935bbb7b8fdd7015d715a795945b434788657dee0a50d29",
        profile: &ENGINE_2024,
    },
];

/// The engine a folder's program carries.
#[derive(Debug, Clone, Copy)]
pub struct Engine {
    /// The build, when the program is one of [`BUILDS`].
    pub build: Option<&'static Build>,
    pub profile: &'static Profile,
}

impl Engine {
    /// From the program file: a build checked when its link time stamp
    /// and size match, otherwise the profile of a build of the same
    /// program name, otherwise [`ENGINE_2026`].
    pub fn of(exe: Option<&Path>) -> Self {
        let Some(path) = exe else {
            return Self::unknown();
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let linked = link_time(path);
        if let Some(b) = BUILDS
            .iter()
            .find(|b| Some(b.linked) == linked && b.size == size)
        {
            return Self {
                build: Some(b),
                profile: b.profile,
            };
        }
        // Of the builds with that name, the one linked nearest in time.
        let near = |b: &&Build| (b.linked as i64 - linked.unwrap_or(0) as i64).abs();
        match BUILDS.iter().filter(|b| b.exe == name).min_by_key(near) {
            Some(b) => Self {
                build: None,
                profile: b.profile,
            },
            None => Self::unknown(),
        }
    }

    pub fn unknown() -> Self {
        Self {
            build: None,
            profile: &ENGINE_2026,
        }
    }

    /// The result of an engine call where this build differs from the
    /// common list.
    pub fn returns_value(&self, class: i32, sub: i32) -> Option<bool> {
        self.profile
            .results
            .iter()
            .find(|r| r.0 == class && r.1 == sub)
            .map(|r| r.2)
    }
}

/// The link time stamp of a PE program, from its first 4 KiB.
fn link_time(path: &Path) -> Option<u32> {
    let mut head = Vec::with_capacity(4096);
    fs::File::open(path)
        .ok()?
        .take(4096)
        .read_to_end(&mut head)
        .ok()?;
    let u32_at = |o: usize| {
        head.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let pe = u32_at(0x3c)? as usize;
    if head.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    u32_at(pe + 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_are_distinct() {
        for (i, a) in BUILDS.iter().enumerate() {
            for b in &BUILDS[i + 1..] {
                assert!(a.linked != b.linked || a.size != b.size);
            }
        }
    }

    #[test]
    fn reads_the_link_time() {
        let dir = std::env::temp_dir().join(format!("silky-engine-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut exe = vec![0u8; 0x200];
        exe[..2].copy_from_slice(b"MZ");
        exe[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        exe[0x80..0x84].copy_from_slice(b"PE\0\0");
        exe[0x88..0x8c].copy_from_slice(&1510211209u32.to_le_bytes());
        let path = dir.join("apeiria.exe");
        fs::write(&path, &exe).unwrap();
        assert_eq!(link_time(&path), Some(1510211209));
        // Same name, other size: the profile of that program's build.
        let e = Engine::of(Some(&path));
        assert!(e.build.is_none());
        assert_eq!(e.profile.name, "engine-2017");
        assert_eq!(e.returns_value(23, 2), Some(false));
        fs::remove_dir_all(&dir).ok();
        assert_eq!(Engine::of(None).profile.name, "engine-2026");
    }
}
