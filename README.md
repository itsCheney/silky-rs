# silky-rs

English | [简体中文](README.zh-CN.md)

A cross-platform reimplementation of the Silky's engine (the "Silky Engine" used by SILKY'S PLUS and elf titles from 2014 on), written in Rust. It reads the original game files; you need your own copy of the game.

The first target game is ふゆから、くるる。 (Fuyukara, Kururu.), 2026 international build 1.0.1.0. The first desktop platform is Windows.

## Status

Early. What works today:

- `.arc` archives, including the LZSS compression and the override order from `Patch.xml` (`script.arc;script1.arc`: the later archive wins).
- AKB images, including `AKB+` difference images on top of a base image.
- `.MAP` text tables (one section per language).
- `.MES` scripts and `LIBLARY.LIB`: header, offset tables and the full byte code decoder.
- `.VSD` movie containers (an MPEG-1 program stream inside).

On ふゆから、くるる。 all 56 scripts (7.9 million instructions) decode with every jump landing on an instruction, and all 3,255 images decode.

Not done yet: the script VM, the engine functions the scripts call, rendering, sound, movies, saves, and the desktop player.

## Layout

| Crate | What it is |
| --- | --- |
| `silky-format` | File formats. No platform code. |
| `silky-tool` | Command line tool to inspect a game folder. |

Planned: `silky-vm` (byte code interpreter), `silky-runtime` (engine functions, scene, text, sound), `silky-player` (desktop player: winit, wgpu, cpal) and `silky-run` (headless runner for tests).

## Building

Rust 1.85 or newer.

```sh
cargo build --release
cargo test
```

## silky-tool

```sh
silky-tool info  <game folder>                  # archives and file counts
silky-tool check <game folder> [--images]       # decode every script, MAP and image
silky-tool dis   <game folder> FK001.MES        # disassemble a script
silky-tool map   <game folder> FK001.MAP --section 1
silky-tool akb   <game folder> TITLE_BG_01.AKB title.png
silky-tool ls      <file.arc>
silky-tool extract <file.arc> <out folder>
```

## Rules for this project

- Game data never goes into the repository: no images, scripts, sound, executables or dumps. Tests use small files built in code.
- Engine behaviour is learned from the game's own files and, where needed, by reading the engine executable. The runtime never needs or loads the executable, and copy protection is out of scope.
- Unknown opcodes and engine functions are reported with their arguments. They are never treated as success just to keep a script going.

## Credits

The archive, LZSS, AKB and VSD layouts follow [GARbro](https://github.com/morkt/GARbro) (MIT). The script opcode table started from TesterTesterov's [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler). See [THIRD_PARTY.md](THIRD_PARTY.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
