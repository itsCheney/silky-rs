# Engine builds

English | [简体中文](README.zh-CN.md)

Each game carries its own copy of the engine, linked into its program file, and the engine classes (1 to 28) changed between these copies. The runtime reads the program's link time stamp and size and picks the build's profile (`crates/silky-format/src/engine.rs`). A program not listed gets the profile of the listed build with the same file name linked nearest in time, and an unknown game gets `engine-2026`.

| Profile | Program | Version | Linked (UTC) | Toolset | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| [engine-2017](engine-2017.md) | `apeiria.exe` (景の海のアペイリア) | 1.0.3.0 | 2017-11-09 07:06:49 | Visual Studio 2013 (linker 12.0) | `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e` |
| [engine-2021](engine-2021.md) | `fuyukuru.exe` (ふゆから、くるる。 package) | 1.0.0.0 | 2021-09-06 20:52:27 | Visual Studio 2019 (linker 14.29) | `9797a7cb71955c0ddb4b4865e0e592166f82386a5e642bd25072922cf9d6d9fa` |
| [engine-2021](engine-2021.md) | `fuyukuru.exe` (package, update 1.01) | 1.0.1.0 | 2021-09-29 05:47:46 | Visual Studio 2019 (linker 14.29) | `b5b163ceb979d9825c6586401da005d51a57f4005a93cf227ca304bf9e831988` |
| [engine-2022](engine-2022.md) | `doukyuusei.exe` (同級生 Remake, Steam) | 1.0.4.0 | 2022-03-15 07:54:45 | Visual Studio 2017 (linker 14.16) | `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451` |
| [engine-2024](engine-2024.md) | `kimaten2.exe` (きまぐれテンプテーション2) | 1.0.0.0 | 2024-10-15 01:13:11 | Visual Studio 2019 (linker 14.29) | `5cbd99107b94befe4935bbb7b8fdd7015d715a795945b434788657dee0a50d29` |
| [engine-2026](engine-2026.md) | `fuyukuru.exe` (ふゆから、くるる。 download, Japanese, English and Chinese) | 1.0.1.0 | 2026-03-18 14:12:42 | Visual Studio 2022 (linker 14.44) | `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707` |

The version is the program's version resource; the engine has no version number of its own, so the profiles are named by the year the program was linked. The same game can come in builds of different years: the ふゆから、くるる。 download was linked again in 2026 with a newer engine than its 2021 package.

## What a profile holds

Only what differs between the builds checked:

| Field | Meaning |
| --- | --- |
| `sprites_and_flags` | `Sprite.1`, the other `Sprite` calls, `Flag` and `Mouse.4` work as read on the 2026 build |
| `sheet_store` | A layer `Sprite.1` creates hidden as a store of button sheets |
| `backlog_voice_end` | `BackLog.3` without arguments ends the voice mark of the entry |
| `movie_2_closes` | `Movie.0` takes the file alone, `Movie.3` plays, `Movie.2` closes and returns nothing |
| `heffect_sounds` | `HEffect` (class 17) plays sounds like `Effect` |
| `results` | Engine calls that return a value, or not, unlike the common list |

The `Game` commands (class 29) are not part of a profile: they belong to each game (`crates/silky-runtime/src/games/`).

## Adding a build

Read the time stamp at offset 8 of the PE header, the size and the version resource, add a `Build` to `BUILDS`, and give it the profile it matches. Where it differs from every profile, add one with a page here.
