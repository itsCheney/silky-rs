> [!WARNING]
> Work in progress. Two games run; other Silky Engine games are untested.

# silky-rs

English | [简体中文](README.zh-CN.md)

A cross-platform reimplementation of the Silky's engine (the "Silky Engine" used by SILKY'S PLUS and elf titles from 2014 on), written in Rust. It reads the original game files; you need your own copy of the game. Engine behaviour is taken from the game files and, where needed, from the engine executable, which the runtime itself never loads.

## Status

| Game | What works | Main gaps |
| --- | --- | --- |
| ふゆから、くるる。 (2026 international build 1.0.1.0) | All scripts and images decode; every route plays to its ending in English and both Chinese; title, message window, selections, saves, backlog, settings, extras, movies and the effects read from the executable | English text metrics and selection placement not compared with the game; a few effect timings are approximate |
| きまぐれテンプテーション2 (2024 Japanese build, patch 1.02) | Story text, interface, action menu and map, costume, notebook and collection, extras, E-mote models, H mode | Pin of the button bar; parts of H mode; investigations need the right answers at selections when run headless |
| Other Silky Engine games | Not tested | Each game adds its own engine commands |

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
- [Features](docs/features.md): file formats, VM, renderer and engine screens shared by all games.
- [Running](docs/running.md): the desktop player, fonts, saves and the GPU or software renderer.
- [Command line](docs/cli.md): `silky-tool` and `silky-run`.
- [Third-party references](THIRD_PARTY.md).

## Layout

| Crate | What it is |
| --- | --- |
| `silky-format` | File formats. No platform code. |
| `silky-vm` | Byte code interpreter. Engine functions are handed to a host. |
| `silky-runtime` | Engine functions on a scene model, the renderer and the engine screens. No platform code. `Session` takes input and gives frames. |
| `silky-tool` | Command line tool to inspect a game folder. |
| `silky-run` | Headless runner: logs engine calls, or saves frames as PNG. |
| `silky-player` | Desktop player (winit, wgpu). |

Before committing, run `cargo test --workspace`.

## Rules for this project

- Game data never goes into the repository: no images, scripts, sound, executables or dumps. Tests use small files built in code.
- The runtime never needs or loads the executable, and copy protection is out of scope.
- Unknown opcodes and engine functions are reported with their arguments. They are never treated as success just to keep a script going.
- Only what the original engine reads is supported; fan translation patches are not.

## Credits

The archive, LZSS, AKB and VSD layouts follow [GARbro](https://github.com/morkt/GARbro) (MIT). The script opcode table started from TesterTesterov's [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler). See [THIRD_PARTY.md](THIRD_PARTY.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE). `vendor/eluna_rs` and `crates/silky-runtime/src/emote/draws.rs` (adapted from siglus_rs) are MPL-2.0.

The license gives no rights to the original games, their programs, pictures, sound or scripts.
