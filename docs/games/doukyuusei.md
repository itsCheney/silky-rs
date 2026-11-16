# 同級生 Remake

English | [简体中文](doukyuusei.zh-CN.md)

Steam release (Shiravune, 2022) in English, Simplified Chinese and Traditional Chinese; there is no Japanese text. The runtime knows the game by `doukyuusei.exe` in its folder; the build checked is `doukyuusei.exe` 1.0.4.0 with SHA-256 `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451`, linked 2022-03-15. Its engine profile is [engine-2022](../builds/engine-2022.md). The screen is 1920 x 1080.

The game is an adventure in a town: the hero spends the days of a summer break going from place to place, and the story scenes start from what he looks at and whom he meets. The scripts run that part themselves and ask the engine for the screens (the room, the town map, the time of day, the people present).

## What works

- Text tables: each script `X.MES` has `X_EN.MAP`, `X_CN.MAP` and `X_TW.MAP`. `Game.24(id, 0)` takes line `id` of the running script's table for the next `MesWnd.3`; `Game.24(id, 1)` names the speaker from `NAMES_<lang>.MAP`; `Game.24(name, 2)` loads a table. In the lines, `\n` breaks the line, `\i` turns italics on and off and `:myname:` is the hero's name (Takurou, 拓郎). `Game.22` makes the hero the speaker.
- The message window, measured under Wine and read from the executable:
  - The window starts at y = 730 (`/Config/MesWnd/PosY`): rows 0..350 of `message_p.akb`, a black band at 63 % opacity (the default `Alpha` of 37), then rows 350..700, its frame lines.
  - Name plate: rows 1350..1400 of the sheet (a star and a gold rule) at (231, 799), the name from (294, 800).
  - Text in the game's bitmap fonts from (242, 870): half-width letters take a 20-pixel cell, full-width characters two; lines are 48 pixels apart. Each glyph is drawn black at its eight neighbours, then white, as the executable does. `\i` switches to `font_han_i.akb`.
  - Word wrap as in the executable: a word (with the space after it) goes to the next line when its letters would reach 74 cells from the line start; its trailing space does not count. Checked on many English lines under Wine.
  - Buttons from `mes_o.area`: skip, skip all, auto, quick save and quick load along the bottom (ids 0..4, screen pixels) and the voice and close buttons (ids 5 and 6, relative to the window top; drawn a pixel left of their areas). Cells of `message_p.akb` with rows by state.
  - Click wait: the yellow mark of `message_p.akb` (three 48-pixel frames down from (1716, 700)) at (1723, 996). The frame time is a guess.
  - The first lines match the original under Wine within a few levels per pixel.
- Title menu (`Game.28`): `title_bg_<lang>.akb` (logo included) and six entries from `title.area`, 494 x 78 cells of `title_p_<lang>.akb` three to a row with four state rows; continue and quick load are grey without saves. The extras page (`title2.area`): album, memory reel, music, a hidden fourth entry and back. New game returns 0 and the extras 257 to 260; quit closes the game. Matches the original under Wine.
- Backgrounds take the common `Game.12` calls.
- Headless, `silky-run snap` reaches the hero's room: the opening and the first phone call play with their text.

## Not done yet

- The room screen (`Game.16`): the program's area selection (`CAreaSelect`). The room's hot spots come from an `.AREA` file per room (`42.AREA` for the hero's room: many rectangles per object), the buttons (sleep, look, talk, go out) from `search_p.akb`, the date, time and money plate from `cal_p_<lang>.akb`. The call returns verb × 256 + object (verb 1 looks, 255 is a button). Until it is done, the story stops in the hero's room.
- The town map, the time of day and the people present (`Game.17`, `Game.29`, `Game.36` and others), the name entry (`Game.18`), the system menu and the other screens.
- Chinese text: the layout of full-width characters is assumed (two cells each, a character a word); it is not checked under Wine yet.
- The other `Game` commands; the engine classes are taken from [engine-2026](../builds/engine-2026.md) unchecked.

## Where things are in the executable

Addresses in the program as it runs (the Steam wrapper unpacks it at its own image base, 0x400000). The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` command dispatch (`CFuncExec`) | 0x4e2020 |
| `Game.24` text lines | 0x4e4a20 |
| `Game.22` hero speaks | 0x4e31e0 |
| `Game.28` title menu | 0x4e5820 |
| `Game.16` room screen (`CAreaSelect`, run) | 0x4a3500 |
| Bitmap fonts loaded | 0x4a4340 |
| One glyph drawn with its outline | 0x4a44e0 |
| Message text laid out (words, `\n`, `\i`) | 0x4a6a00 |
| Line breaks before closing punctuation | 0x4a7150 |
