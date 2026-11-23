# ふゆから、くるる。

English | [简体中文](fuyukuru.zh-CN.md)

International build 1.0.1.0 (Japanese, English, Simplified Chinese). The runtime knows the game by `fuyukuru.exe` in its folder; the build described here has SHA-256 `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707`. Program version 1.0.1.0, linked 2026-03-18. Engine profile: [engine-2026](../builds/engine-2026.md). Its own `Game` commands: [game-commands.md](../game-commands.md#ふゆからくるる).

The Japanese package (`fuyukuru.exe` 1.0.0.0, or 1.0.1.0 with official update 1.01, both linked in 2021) also runs. Its text is in the scripts, its art has no language suffix, and its movie calls are older ([engine-2021](../builds/engine-2021.md)). The rest of this page describes the download.

## What works

- All 56 scripts (7.9 million instructions) and all 3,255 images decode. Headless, the VM boots `start.mes` through the logo, caution and title scripts and plays the story from `fk001.mes` in all three languages. Every route reaches its ending with no engine call left undone.
- Message text: `Font.0(28, 28)` cells (14 pixels per letter), solid black shadow; lines 36 apart in Japanese, 28 in English and Chinese. A line wraps at 61 letters (854 pixels).
- Window band drawn at the window opacity under the paper texture; 40 % transparent by default.
- Settings page with the window sample (the game shows through it while the sample text types out). Star thumbs on the settings, backlog, save and music screens.
- Backlog text with a black edge. Save rows: number, time to the second, title in orange; the Auto and Quick tabs appear only when loading. Help lines. Music volume and share of tracks heard.
- Selections: bars built from three pieces of `message_p`, sized to the longest choice, centred, 84 pixels apart above the window; white text over a shadow.
- With movies on (the default), the scripts ask for moving backgrounds of the running scenes (`Game.12` sub 10); the picture stands still here, as with movies off.
- Effects: rule-mask wipes and screen dimming (sprite animations), snow, the future flash, pulse zoom, slash, white zoom flash, screen quake, character hops and swings (steps of 16 ms, the engine's sprite animation step), queued music that fades in over the time the script gives, and pictures that change while a voice line plays (`.vo` files).
- Future flash: the picture shrinks away turning once every 16 frames, and comes back growing while its turns slow down.
- `Flag` calls keep the flags from byte 1000 on across games in the system data, as the executable does with its system flag file.
- The "fixed window" setting is kept but changes nothing, since the window never moves here.

## Where things are in the executable

Addresses in `fuyukuru.exe` of the download build above. The code itself does not name them.

| What | Address |
| --- | --- |
| Future flash (`Game.112`) | 0x4a2770 |
| Future flash animation (spin and scale per frame) | 0x46d7f0 |
| Selection bars (size and placement, drawing) | 0x485aa0, 0x485ea0 |
| Character hop and swing steps, keyframe table | 0x460a40, 0x45fb50 |
