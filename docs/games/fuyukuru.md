# ふゆから、くるる。

English | [简体中文](fuyukuru.zh-CN.md)

International build 1.0.1.0, Japanese, English and Simplified Chinese. The runtime knows the game by `fuyukuru.exe` in its folder; the build checked is the one whose `fuyukuru.exe` has SHA-256 `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707`. Its own `Game` commands are listed in [game-commands.md](../game-commands.md#ふゆからくるる).

All 56 scripts (7.9 million instructions) decode with every jump landing on an instruction, and all 3,255 images decode. Headless, the VM boots `start.mes` through the logo, caution and title scripts, and plays the story from `fk001.mes` on with its text in all three languages. Every route plays to its ending.

Compared with screenshots of the original (Windows, Simplified Chinese): the message window, name plate, window portrait, the position and pitch of Chinese text, and the size and position of standing characters. Still provisional: English text metrics (no English screenshots to compare with yet), the line pitch of two-line messages, the wrap width, and where selections sit on screen.

Effects read from the executable: rule-mask wipes and screen dimming (sprite animations), snow, the future flash, pulse zoom, slash, white zoom flash, screen quake, character swing, and the pictures that change while a voice line plays (`.vo` files). Approximate: the picture does not shrink during the future flash, queued music starts without its fade-in, and the step time of character hops is a guess.

The `Flag` calls keep the flags from byte 1000 on across games in the system data, as the executable does with its system flag file. A run through every route leaves no engine call undone. The "fixed window" setting is kept but changes nothing, since the window never moves here.
