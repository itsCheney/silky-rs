# Running

English | [简体中文](running.zh-CN.md)

## Building

Rust 1.85 or newer.

```sh
cargo build --release
cargo test --workspace
```

The binaries land in `target/release`. CI builds Windows binaries on each push (the `silky-rs-windows` artifact).

## silky-player

```sh
silky-player <game folder>                 # the game's first language
silky-player <game folder> --lang zh-hans  # Simplified Chinese (or 1)
silky-player <game folder> --font msyh.ttc --font arial.ttf
```

Saves go to `%APPDATA%\silky-rs\<game folder name>` on Windows (`~/Library/Application Support/silky-rs/...` on macOS, `~/.local/share/silky-rs/...` elsewhere); `--saves <folder>` picks another. Without `--font` the player looks for the fonts the game names (Arial, Microsoft YaHei, Microsoft JhengHei in the Windows fonts folder), then common fonts on macOS and Linux. Left click, Enter or Space goes on; right click hides the message window (the settings can change that); Escape closes a screen or ends a movie. The player composes frames on the GPU through wgpu (Metal, Vulkan, D3D12): the runtime records what each frame draws and `silky_runtime::compose` draws it at window size, keeping pictures, glyphs and E-mote models on the GPU. `--software` (or a machine without a usable GPU) draws frames in software instead.

## Embedding

`silky-runtime::Session` holds a running game (interpreter, runtime, renderer, interface) and takes pointer, key and clock input; the desktop player and future embeddings only forward input and show its frames.
