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
- The script VM: stack, variable banks, functions, script calls and selections. Engine functions go to a host.
- First engine functions and a software renderer: backgrounds, event pictures with their difference pictures, standing characters with expressions, the message window with its name plate, portrait and text, localized file names, the title picture.
- The engine-drawn interface: title menu buttons (layout taken from the executable, per language), the button bar and icons of the message window (`mes_o.area`), the click-wait indicator, and selections (`Game.20`).
- A desktop player: click or Enter to go on, right click to hide the window, the mouse wheel for the backlog, Escape to close a screen, auto and skip modes, the title menu and selections.
- Sound: music (looping at the `smpl` loop points of the WAV files), sound effects and voices, mixed in software; the desktop player plays them with cpal.
- Timing and transitions: timed waits (a click skips them), fades to and from a colour, crossfades of pictures and characters, rule wipes with the game's `NN.bin` masks, screen shake, and waits for a sound to end. Times are read as frames of 1/60 s for now.
- The save and load screen as the engine draws it (from `saveload.area` and its sheets): save, auto save and quick save tabs, thumbnails, dates, the newest-save badge, overwriting, deleting one or all saves, help lines from `saveloadhelp.map`, and the yes/no dialog with "do not ask again". An auto save is made when a selection comes up; Continue opens the newest save. Saves are this runtime's own JSON files, with a PNG thumbnail each.
- The backlog (`backlog00.area`): the last 300 messages with their nameplates, replaying a line's voice, playing marked voices one after another, and jumping back to a message.
- The settings screen (`config00.area`, `config01.area`), opened from the title menu, the window's settings button or a right click: text speed and letter-by-letter text with an optional pause after punctuation, skip (read text only, speed, keep after selections), auto mode, message window colour and opacity, effect speed, which questions to ask, what a right click does, the language (switched while playing), and the volumes of all sound, music, effects, voices and each character's voice. Help lines come from `confighelp.map` and `soundconfighelp.map`.
- The extras of the title menu: the CG gallery (`cgmode.area`, nine pages of thumbnails, click through the pictures of an entry), scene replay (`scmode.area`) and the music room (`soundmode.area`, with a progress bar). What unlocks them is kept with the settings in `system.json` in the save folder: pictures and music met in the story, and lines read. The tables of gallery pictures, scenes and tracks are taken from the executable of ふゆから、くるる。.
- Movies (`.vsd`): MPEG-1 video and MP2 sound decoded with [pl_mpeg](https://github.com/phoboslab/pl_mpeg) (MIT, in `crates/silky-runtime/vendor`); the picture follows the sound clock, and a click ends the movie.
- The window's previous choice button (back to the message before the last selection) and next choice button (skip to the next selection, through unread text too).

On ふゆから、くるる。 all 56 scripts (7.9 million instructions) decode with every jump landing on an instruction, and all 3,255 images decode. Headless, the VM boots `start.mes` through the logo, caution and title scripts, and plays the story from `fk001.mes` on with its text in all three languages.

Not done yet: close-up insets and other effects. Calls seen in the story that are not carried out include `Game.12` subs 4 and 5 (they look like a camera move over an event picture) and `Game.13` subs 7 and 14 (character moves); what they do is still to be read from the executable. The "fixed window" setting is kept but changes nothing, since the window never moves here. `silky-run snap` lists every engine call the runtime does not carry out yet.

Compared with screenshots of the original (Windows, Simplified Chinese): the message window, name plate, window portrait, the position and pitch of Chinese text, and the size and position of standing characters. Still provisional: English text metrics (no English screenshots to compare with yet), the line pitch of two-line messages, the wrap width, and where selections sit on screen.

## Layout

| Crate | What it is |
| --- | --- |
| `silky-format` | File formats. No platform code. |
| `silky-vm` | Byte code interpreter. Engine functions are handed to a host. |
| `silky-runtime` | Engine functions on a scene model, and a software renderer. No platform code. |
| `silky-tool` | Command line tool to inspect a game folder. |
| `silky-run` | Headless runner: logs engine calls, or saves frames as PNG. |
| `silky-player` | Desktop player (winit; frames drawn in software). |

`silky-runtime::Session` holds a running game (interpreter, runtime, renderer, interface) and takes pointer, key and clock input; the desktop player and future embeddings only forward input and show its frames.

## silky-player

```sh
silky-player <game folder>                 # English
silky-player <game folder> --lang 1        # Simplified Chinese
silky-player <game folder> --font msyh.ttc --font arial.ttf
```

Saves go to `%APPDATA%\silky-rs\<game folder name>` on Windows (`~/Library/Application Support/silky-rs/...` on macOS, `~/.local/share/silky-rs/...` elsewhere); `--saves <folder>` picks another. Without `--font` the player looks for the fonts the game names (Arial, Microsoft YaHei, Microsoft JhengHei in the Windows fonts folder), then common fonts on macOS and Linux. Left click, Enter or Space goes on; right click hides the message window (the settings can change that); Escape closes a screen or ends a movie.

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

## silky-run

```sh
silky-run trace <game folder>                        # boot log until the title menu
silky-run trace <game folder> --result 29.110=1      # pick "Start" on the title menu
silky-run trace <game folder> --result 29.110=1 --text-only --lang 1 --max-calls 50000
```

```sh
silky-run snap <game folder> --out snaps --font DejaVuSans.ttf --lines 0,10,50
silky-run snap <game folder> --lang 1 --font msyh.ttc --lines 170
```

`snap` runs the game with `silky-runtime` and saves PNG frames of the title screen and of the listed message lines (counted from the first line of the game). Fonts are not part of the game files: pass one or more with `--font`, in fallback order; without a font the text is left out.

`trace` performs no engine function; it logs each call with its arguments. Calls whose result the scripts use return 0 unless `--result CLASS.SUB=VALUE` says otherwise. Selections (`Game.20`) take `--choose`, counted from 0 (default the first option). `--lang` picks the text section of the `.MAP` tables (0 English, 1 Simplified Chinese, 2 Traditional Chinese in this build).

## Rules for this project

- Game data never goes into the repository: no images, scripts, sound, executables or dumps. Tests use small files built in code.
- Engine behaviour is learned from the game's own files and, where needed, by reading the engine executable. The runtime never needs or loads the executable, and copy protection is out of scope.
- Unknown opcodes and engine functions are reported with their arguments. They are never treated as success just to keep a script going.

## Credits

The archive, LZSS, AKB and VSD layouts follow [GARbro](https://github.com/morkt/GARbro) (MIT). The script opcode table started from TesterTesterov's [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler). See [THIRD_PARTY.md](THIRD_PARTY.md).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
