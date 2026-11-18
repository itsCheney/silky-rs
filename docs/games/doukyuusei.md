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
- Movies: `Movie.0` (file), `Movie.3` plays and returns whether it played, `Movie.2` closes, as in the 2017 and 2021 builds; the opening plays after the first phone call.
- The room screen (`Game.16(set)`, the program's area selection), read from the executable and matched under Wine within a level per pixel:
  - Buttons from `search_p.akb`: three large ones (168 x 171 cells, a row per state) at x = 1752, 1584, 1416, y = 909, and two small ones (150 x 153) at (15, 927) and (165, 927). Set 0 has go out, look and sleep; the other sets have a back arrow (138 x 140 from (672, 0), at (1782, 940)) in place of go out and talk in place of sleep. The small buttons are quick save and quick load, quick load gray without a quick save.
  - `areasel_0N.carea` (a byte per screen pixel, 255 for none) says which button a pixel belongs to.
  - Look hides the buttons and shows the back arrow; the place's hot spots come from `<background up to "_">.area` (`42.area` for the hero's room), many rectangles per id. The high byte of an id is the verb (0 look, 4 open and so on), the low byte the object.
  - The call returns (verb + 1) × 256 + object for a hot spot and 255 × 256 + id for a button.
  - Hint bar: `search_p.akb` (0, 684, 584, 68) at (658, 1005), and the line of `SYSTEM_<lang>.MAP` for the element under the pointer: 232 to 238 for the buttons, 239 + verb for a hot spot, 252 for back; empty on nothing. The text is the bitmap font at half size from (725, 1029).
- The hours dial (`Game.29(kind)`, the program's time selection): `time_p_<lang>.akb` with the dial (750 x 756 from (0, 592)) at (586, 42), the question of the kind (403 x 76 from (1152, 76 × kind)) at (760, 456), eight hour buttons (144 x 148 cells) and back (151 x 153 from (750, 592)) placed by `TimeSelect.area`. Kind 0 asks how long to sleep, 1 how far to wind back, 2 how far to skip forward. Returns the hours, 0 for back. Matches the original under Wine.
- Selections (`Select.2`, the program's normal selection): each option body leaves its line id in `h[40 + i]`; the labels are those lines of the script's text table. Each option is a bar of `sel_p.akb` (rows of 110 pixels by state): the 60-pixel ends, 105 pixels of body next to each end and a 40-pixel body slice per two cells of the longest label. The bars are centred across the screen and, 118 pixels apart, in the 780 pixels above the bottom buttons; a label is centred in its bar, 35 pixels from its top, in the message font. The message window goes away; its bottom buttons stay. Matches the original under Wine; only the first four options show (paging is not done).
- The calendar (`Game.25`): the executable writes a year into the raw area from byte 18000, 32 bytes a month, a byte a day: the weekday (1 Monday to 7 Sunday), plus 8 on a holiday, 0 past the month's end. The scripts count a month's days and read the weekdays from it.
- Headless, `silky-run snap` plays the opening, looks around the hero's room, sleeps an hour, looks at everything again and goes out; it stops at the town map.

## Not done yet

- The room screen: the date, time and money plate (`cal_p_<lang>.akb`) and the system menu button at the top right; the quick save and quick load buttons do nothing yet.
- More than four options (the original pages them with arrows), and options grayed once taken.
- The town map (`Game.20(30)` in `town1.mes`, where the story stops headless), the people present and the other place screens (`Game.17`, `Game.36` and others), the name entry (`Game.18`), the system menu and the other screens.
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
| Room buttons placed | 0x4a2720 |
| Room button drawn by state | 0x4a39b0 |
| Room hint for a hot spot (verb) | 0x46e490 |
| Room hint for a button | 0x46cc90 |
| `Game.29` hours dial (`CTimeSelect`): call, set up, drawn | 0x4e250c, 0x4dd5d0, 0x4ddc70 |
| Hours dial: button drawn, clicked | 0x4ddae0, 0x4dd750 |
| Selection bars built, placed (`CNormalSelect`) | 0x4c5f50, 0x4c62a0 |
| `Game.25` calendar written | 0x4e4ca0 |
| Bitmap fonts loaded | 0x4a4340 |
| One glyph drawn with its outline | 0x4a44e0 |
| Message text laid out (words, `\n`, `\i`) | 0x4a6a00 |
| Line breaks before closing punctuation | 0x4a7150 |
