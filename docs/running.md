# Running

English | [简体中文](running.zh-CN.md)

## Building

Rust 1.85 or newer.

```sh
cargo build --release
cargo test --workspace
```

The binaries land in `target/release`. CI builds Windows binaries on each push (the `silky-rs-windows` artifact).

### Movies in ASF files (optional)

先輩が私の妄想にドージンする keeps its movies in ASF files (VC-1 video, WMA Pro sound), which the built-in MPEG-1 decoder cannot read. The `ffmpeg` feature of `silky-runtime` (passed through by `silky-player` and `silky-run`) decodes them with a trimmed, LGPL-only FFmpeg linked statically. It is off by default; without it the build is unchanged and those movies are skipped.

```sh
tools/ffmpeg/build.sh                            # fetches FFmpeg n7.1 and installs into target/ffmpeg
cargo build --release --features silky-player/ffmpeg,silky-run/ffmpeg
```

`tools/ffmpeg/build.sh [prefix]` needs git, make and a C compiler (nasm is used when found); `FFMPEG_SRC=<dir>` builds an existing source tree instead of fetching one. The cargo build looks for the libraries in `target/ffmpeg`, or in `SILKY_FFMPEG_DIR`, and stops with a message naming the script when they are missing. The libraries are about 23 MB in all.

## silky-player

```sh
silky-player <game folder>                 # the game's first language
silky-player <game folder> --lang zh-hans  # Simplified Chinese (or 1)
silky-player <game folder> --font msyh.ttc --font arial.ttf
```

Saves go to `%APPDATA%\silky-rs\<game folder name>` on Windows (`~/Library/Application Support/silky-rs/...` on macOS, `~/.local/share/silky-rs/...` elsewhere); `--saves <folder>` picks another. Without `--font` the player looks for the fonts the game names (Arial, Microsoft YaHei, Microsoft JhengHei in the Windows fonts folder), then common fonts on macOS and Linux. Left click, Enter or Space goes on; right click hides the message window (the settings can change that); Escape closes a screen or ends a movie. On the extras of 景の海のアペイリア the arrow keys move the pointer between buttons and Enter clicks, as in the original. The player composes frames on the GPU through wgpu (Metal, Vulkan, D3D12): the runtime records what each frame draws and `silky_runtime::compose` draws it at window size, keeping pictures, glyphs and E-mote models on the GPU. `--software` (or a machine without a usable GPU) draws frames in software instead. `--trace <file>` writes the engine calls of the session to a trace file when the window closes (see [trace.md](trace.md)).

## Embedding

`silky-runtime::Session` holds a running game (interpreter, runtime, renderer, interface) and takes pointer, key and clock input; the desktop player and future embeddings only forward input and show its frames.
