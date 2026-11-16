# ふゆから、くるる。

English | [简体中文](fuyukuru.zh-CN.md)

International build 1.0.1.0, Japanese, English and Simplified Chinese. The runtime knows the game by `fuyukuru.exe` in its folder; the build checked is the one whose `fuyukuru.exe` has SHA-256 `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707`. Its own `Game` commands are listed in [game-commands.md](../game-commands.md#ふゆからくるる). Program version 1.0.1.0, linked 2026-03-18; its engine profile is [engine-2026](../builds/engine-2026.md).

The Japanese package (`fuyukuru.exe` 1.0.0.0, or 1.0.1.0 with the official update 1.01, both linked in 2021) runs too: its text is in the scripts, its art has no language suffix, and its engine's movie calls are older ([engine-2021](../builds/engine-2021.md)). Checked with update 1.01: the story runs headless with every engine call carried out, and the title menu, settings, extras page and save screen show; the rest of this page is about the download.

All 56 scripts (7.9 million instructions) decode with every jump landing on an instruction, and all 3,255 images decode. Headless, the VM boots `start.mes` through the logo, caution and title scripts, and plays the story from `fk001.mes` on with its text in all three languages. Every route plays to its ending.

Compared with the original running under Wine, in English and Chinese (the download) and Japanese (the package with update 1.01): message text in `Font.0(28, 28)` cells (14 pixels for a letter) with a solid black shadow, lines 36 apart in Japanese and 28 in English and Chinese; the window band at the window opacity under the paper texture, 40 % transparent by default as in both programs; the settings page with the window sample (the game shows through it while the sample text types out); the star thumbs of the settings, backlog, save and music screens; backlog text with a black edge; save rows (number, time to the second, title in orange; the Auto and Quick tabs only when loading); help lines; the music volume and the share of tracks heard. Screenshots of the same places differ in under 2.5 % of their pixels, most of it in glyph shapes (the fonts differ). Still provisional: the wrap width and where selections sit on screen.

With movies on (the setting both programs start with), the scripts ask for moving backgrounds of the running scenes (`Game.12` sub 10); the picture stands still here, as it does with movies off.

Effects read from the executable: rule-mask wipes and screen dimming (sprite animations), snow, the future flash, pulse zoom, slash, white zoom flash, screen quake, character swing, and the pictures that change while a voice line plays (`.vo` files). Approximate: the picture does not shrink during the future flash, queued music starts without its fade-in, and the step time of character hops is a guess.

The `Flag` calls keep the flags from byte 1000 on across games in the system data, as the executable does with its system flag file. A run through every route leaves no engine call undone. The "fixed window" setting is kept but changes nothing, since the window never moves here.
