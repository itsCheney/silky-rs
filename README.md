> [!WARNING]
> Work in progress. Four games run; other Silky Engine games are untested.

# silky-rs

English | [简体中文](README.zh-CN.md)

A cross-platform reimplementation of the Silky's engine (the "Silky Engine" used by SILKY'S PLUS and elf titles from 2014 on), written in Rust. It reads the original game files; you need your own copy of the game. Engine behaviour is taken from the game files and, where needed, from the engine executable, which the runtime itself never loads.

## Status

| Game | What works | Main gaps |
| --- | --- | --- |
| ふゆから、くるる。 (2026 international build 1.0.1.0) | All scripts and images decode; every route plays to its ending in English and both Chinese; title, message window, selections, saves, backlog, settings, extras, movies and the effects read from the executable | English text metrics and selection placement not compared with the game; a few effect timings are approximate |
| きまぐれテンプテーション (Japanese build with the official update 1.04) | Title, message window and button bar, action menu, investigation, evidence and option lists, map, notebook, notices, E-mote models (companion, standing, CG and voice-timed), effects, CG, scene and music modes, Anne viewer, staff roll | Effects of `Game.1001` and `1002` are not compared with the original; skip tutorial; headless, it reaches the bad endings and some branches, not the 06 endings |
| きまぐれテンプテーション2 (2024 Japanese build, patch 1.02) | Story text, interface, action menu and map, costume screen, notebook and collection, extras, E-mote models, H mode (compared with the game, censor mosaic included) | Investigations need the right answers at selections when run headless |
| 先輩が私の妄想にドージンする (Japanese build 1.0.1.0) | Story text to an ending headless | Interface, own commands and movies not yet |
| 景の海のアペイリア (Japanese build with the official update) | All scripts decode and the story runs to its end; title, message window, selections, ruby, the in-game screens (αMAIL, DNA PC, battle gauges), attack and spell effects, the animations of the pictures over the scene, the mail forward screen, CG, scene and music modes; the routes traced against the game match its engine calls to their endings | Only one effect compared frame by frame with the game; the others follow the executable's code |
| 景の海のアペイリア ～カサブランカの騎士～ (Japanese build 1.0.0.0) | Both routes and the last chapter run to the ending; the Apeiria screens and effects, the new wipes and attack kind 18, CG, scene and music modes; the opening's engine calls match the game's, apart from a name and voice call it repeats | A few screens still differ from the game (deduction board wipe, CG page on return); effects not compared frame by frame |
| なないろリンカネーション (2014 Japanese build) | Title menu with its unlocks, message window with buttons, text, speaker name, standing characters, face window, CG registry | Calendar screen, ending overlay, event pictures and scene movies, staff roll; text line length and wait-mark animation not measured; figure positions not compared with the game |
| 同級生 Remake (Steam, English and Chinese) | Title, the hero's room, the hours dial, the walking maps with map jump and place labels, conversations and selections, the flag list with its jump, the menu's slide, name entry with keys and paste, character picker, save and load with comments, hint bars in the GDI font, backlog, settings, the extras (album, memory reel, music, comments) | Other place screens; marked selection options, the settings screen's fade-in and the new keyboard input not yet compared with the game; the GDI font is the frontend's TrueType face standing in for SimHei |
| Other Silky Engine games | Not tested; they run on the shared engine and the common commands | Each game adds its own engine commands, which are reported until added (see [docs/adding-a-game.md](docs/adding-a-game.md)) |

Desktop: Windows first; macOS and Linux build and pass the tests in CI. The core crates also build for iOS. Details per game are in [docs/games.md](docs/games.md), shared engine features in [docs/features.md](docs/features.md).

## Getting started

Rust 1.85 or newer.

```sh
cargo build --release
cargo run --release -p silky-player -- <game folder>
cargo run --release -p silky-player -- <game folder> --lang zh-hans
```

Click, Enter or Space goes on, right click hides the message window, the mouse wheel opens the backlog, Escape closes a screen. Fonts, saves and other options are in [docs/running.md](docs/running.md); the inspection tool and headless runner in [docs/cli.md](docs/cli.md).

## Docs

- [Games](docs/games.md): how far each game runs and what is left.
- [Engine](docs/engine.md): what the games share and what each adds, and how sure each part is.
- [Engine builds](docs/builds/README.md): the program builds checked and how their engines differ.
- [Game commands](docs/game-commands.md): each game's own engine commands.
- [Features](docs/features.md): file formats, VM, renderer and engine screens shared by all games.
- [Adding a game](docs/adding-a-game.md).
- [Running](docs/running.md): the desktop player, fonts, saves and the GPU or software renderer.
- [Command line](docs/cli.md): `silky-tool` and `silky-run`.
- [Traces](docs/trace.md): engine calls of a run, compared with the original program or another run.
- [Third-party references](THIRD_PARTY.md).

## Layout

| Crate | What it is |
| --- | --- |
| `silky-format` | File formats. No platform code. |
| `silky-vm` | Byte code interpreter. Engine functions are handed to a host. |
| `silky-runtime` | Engine functions on a scene model, the renderer and the engine screens; each game's own part in `games/`. No platform code. `Session` takes input and gives frames. |
| `silky-tool` | Command line tool to inspect a game folder. |
| `silky-run` | Headless runner: logs engine calls, or saves frames as PNG; writes and compares traces. |
| `silky-trace` | Trace files of engine calls and the diff of two traces. Nothing in the runtime depends on it. |
| `silky-player` | Desktop player (winit, wgpu). |

Before committing, run `cargo test --workspace`.

## Rules for this project

- Game data never goes into the repository: no images, scripts, sound, executables or dumps. Tests use small files built in code.
- The runtime never needs or loads the executable, and copy protection is out of scope.
- Unknown opcodes and engine functions are reported with their arguments. They are never treated as success just to keep a script going.

## Credits

The archive, LZSS, AKB and VSD layouts follow [GARbro](https://github.com/morkt/GARbro) (MIT). The script opcode table started from TesterTesterov's [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler). See [THIRD_PARTY.md](THIRD_PARTY.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). `vendor/eluna_rs` (a patched copy of [Eluna](https://github.com/xmoezzz/eluna), see its `README.silky.md`) and `crates/silky-runtime/src/emote/draws.rs` (adapted from [siglus_rs](https://github.com/xmoezzz/siglus_rs)) are MPL-2.0.

The license gives no rights to the original games, their programs, pictures, sound or scripts.
