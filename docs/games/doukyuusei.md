# 同級生 Remake

English | [简体中文](doukyuusei.zh-CN.md)

Steam release (Shiravune, 2022) in English, Simplified Chinese and Traditional Chinese; there is no Japanese text. The runtime knows the game by `doukyuusei.exe` in its folder; the build checked is `doukyuusei.exe` 1.0.4.0 with SHA-256 `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451`, linked 2022-03-15. Its engine profile is [engine-2022](../builds/engine-2022.md). The screen is 1920 x 1080.

The game is an adventure in a town: the hero spends the days of a summer break going from place to place, and the story scenes start from what he looks at and whom he meets. The scripts run that part themselves and ask the engine for the screens (the room, the town map, the time of day, the people present).

## What works

- Text tables: each script `X.MES` has `X_EN.MAP`, `X_CN.MAP` and `X_TW.MAP`. `Game.24(id, 0)` takes line `id` of the running script's table for the next `MesWnd.3`; `Game.24(id, 1)` names the speaker from `NAMES_<lang>.MAP`; `Game.24(name, 2)` loads a table. In the lines, `\n` breaks the line, `\i` turns italics on and off and `:myname:` is the hero's name (Takurou, 拓郎). `Game.22` makes the hero the speaker.
- The message window, measured under Wine and read from the executable:
  - The window starts at y = 730 (`/Config/MesWnd/PosY`): rows 0..350 of `message_p.akb`, a black band at 63 % opacity (the default `Alpha` of 37), then rows 350..700, its frame lines. The band's alpha masks the window colour of the settings at their opacity, as the original does under Wine.
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
  - Look hides the buttons and shows the back arrow; the place's hot spots come from the `.area` file the scripts name with `Game.17(file, 0)` (`42.area` for the hero's room), many rectangles per id. The high byte of an id is the verb (0 look, 4 open and so on), the low byte the object.
  - The call returns (verb + 1) × 256 + object for a hot spot and 255 × 256 + id for a button.
  - `Game.17(1)` (the program's plain `CAreaSelect`) is the same selection without buttons: the hot spots of that file and the hint bar, returning (verb + 1) × 256 + object. Talk scenes use it. Not seen under Wine yet.
  - Quick save and quick load (quick load gray without a save). The save holds the message before the room; loading it shows that message, then the room again (the original returns to the room itself).
  - Hint bar: `search_p.akb` (0, 684, 584, 68) at (658, 1005), and the line of `SYSTEM_<lang>.MAP` for the element under the pointer: 232 to 238 for the buttons, 239 + verb for a hot spot, 252 for back; empty on nothing. The text is the bitmap font at half size from (725, 1029).
- The hours dial (`Game.29(kind)`, the program's time selection): `time_p_<lang>.akb` with the dial (750 x 756 from (0, 592)) at (586, 42), the question of the kind (403 x 76 from (1152, 76 × kind)) at (760, 456), eight hour buttons (144 x 148 cells) and back (151 x 153 from (750, 592)) placed by `TimeSelect.area`. Kind 0 asks how long to sleep, 1 how far to wind back, 2 how far to skip forward. Returns the hours, 0 for back. Matches the original under Wine.
- The character picker (`Game.36(sub)`; the fortune teller and the confession), read from the executable, not compared under Wine: the screen dimmed with black at half opacity, then `charselect.area` places 14 cards (ids 0..13, 316 x 305 from `chara_list_<lang>.akb`, seven to a row, a row per state and the second seven 1220 lower), the side buttons settings, backlog, save, load (256..259, 164 x 164 from `chara_menu.akb`), quick save and load (260, 261, 91 x 54 from (0, 656) and (91, 656)) and, for sub 0 only, back (512, 295 x 134 from (951, 0)). Byte variable i is 1 when card i can be picked (the script sets them first); the others are gray. Returns the card from 1, or 15 for back or the secondary click (sub 0).
- The album (`Game.50`, the extras page "Midsummer Album"), read from the executable and matched under Wine (the girl grid and an empty picture page compared): the grid of the girls (`cgmode.area`, cards of `chara_list_<lang>.akb`), and a girl's pictures nine to a page (`cgview.area`). A closed picture shows `thumb_emp_<lang>.akb`; an open one shows the girl's `cg_<name>[_r18].akb` cell under the `omake_p_<lang>.akb` frame. A picture is open when its byte variable is 1; the program has two tables of them, with and without the update archive's pictures, and the `_r18` sheets are used when `cg_v10_r18.akb` exists. The header shows the percentage of pictures seen out of all of them, at least 1 once any is open. The page arrows are gray at the ends. The time buttons set global 19 to 1200, 1600 or 1800. The side tabs answer 100 with global 12 set to the next extras page (memory reel, music, comments); without the update archive the tabs shift: tab 1 is music, tab 2 comments and tab 3 is hidden. Back and the secondary click return from a girl's pictures to the grid and from the grid answer -1. Clicking an open picture answers with globals 10 (the girl from 1), 13 (its place on the page from 1) and 17 (the page from 1) set and 16 cleared. The "unlock all" button shows after the girl's ending (ending flags 1779, 1788, 1806, 1796, 1813, 1742, 1751, 1823, 1766, 1731, 1758, 1838, 1829, 1726 by girl; the 15th card, "Other", after any of them), asks the dialog's two-line question (strip at (880, 519), 727 x 100, at (597, 395), without the "don't show again" box) and sets all the girl's picture flags. The ending flags (byte variables 1720 to 1899) and the picture, scene and music flags (4000 to 6999) are kept across games, outside the saves.
- The memory reel (`Game.51`, the program's `CScMode`), read from the executable and matched under Wine (girl grid and an empty scene page compared): built like the album on `scmode.area` and `cgview.area`, without time buttons. Its scenes are the ending-route flags (byte variables from 1720 on, open when not 0; again two tables, with and without the update archive's scenes, and without them no 15th card). An open scene shows its cell of `sc_<name>[_r18].akb`, its caption from `scene_ch[_r18]_<lang>.akb` (490 x 50 under the picture) and the frame. The header is the second row of `omake_p_<lang>.akb`; the second tab is the current one. Clicking an open scene answers 0 with globals 10 (girl from 1) and 13 (scene from 1) set and 16 cleared; the tabs answer 100 with global 12 at 0, 2 or 3. The unlock button asks the dialog's second two-line question (strip at (880, 619)). Opened from the hero's computer (global 14 is 1) it has no header, tabs or unlock button.
- The music mode (`Game.52`, "Youth Medley", the third extras page), read from the executable: `musicmode.area` with 16 track rows, the four tabs (as in the reel, the music tab current; the header is the third row of `omake_p_<lang>.akb`), four disc buttons and the volume slider. Four discs hold 16, 14, 14 and 14 of 58 tracks; a track is open when its byte variable (4100 to 4168) is 1 or any album ending is seen. The rows come from `music_disc<n>_<lang>.akb` (740 x 64 cells, four columns, four states: open, hover, playing, locked), the disc buttons and the volume parts (background, fill and knob, rows from y 1256) from `omake_p_<lang>.akb`, the record from `record.akb`, `recorddisc.akb` (the arm frame while a track plays) and the disc label `recorddisc<n>_s_<lang>.akb`. Clicking an open track plays its file of the music archive once and clicking the playing one stops it; the secondary click stops the music, else leaves. The slider (shown while the music switch is on) sets the music volume of the settings. The tabs answer 100 with global 12 set to 0, 1 or 3 (without the update archive the scene tab is hidden); back answers -1. The music stops on opening and on leaving. The label does not spin here.
- Selections (`Select.2`, the program's normal selection): each option body leaves its line id in `h[40 + i]`; the labels are those lines of the script's text table. Each option is a bar of `sel_p.akb` (rows of 110 pixels by state): the 60-pixel ends, 105 pixels of body next to each end and a 40-pixel body slice per two cells of the longest label. The bars are centred across the screen and, 118 pixels apart, in the 780 pixels above the bottom buttons; a label is centred in its bar, 35 pixels from its top, in the message font. The message window goes away; its bottom buttons stay. Matches the original under Wine; only the first four options show (paging is not done).
- The calendar (`Game.25`): the executable writes a year into the raw area from byte 18000, 32 bytes a month, a byte a day: the weekday (1 Monday to 7 Sunday), plus 8 on a holiday, 0 past the month's end. The scripts count a month's days and read the weekdays from it.
- The walking maps (`Game.20`, the program's `CCps`), read from the executable. The program takes a call's values from the last one back; the sub command is the last.
  - Sub 0 (`name.mpx`) loads the grid: width and height (u16), then 5 bytes a 48-pixel tile, row by row; byte 4 is the wall flag.
  - Sub 4 (`name.ccd`) loads the walkers. Header words: walker table, program table (a u16 offset per program), start table (camera x, y, walker x, y, a byte each) and tile patterns; there are (word 1 − 9) / 14 walkers of 14 bytes: flags (1 shown, 0x10 stopped by walls, 0x20 the camera follows, 0x40 the hero), look, width, height, x and y (u16), facing × 16 + frame, program, command, count and program counter. Sub 1 places a walker from the start table, sub 5 gives it a program, sub 7 sets its flags, sub 14 and 22 step and turn it, sub 20 makes it follow another, sub 31 puts the walkers and camera back from the raw area (camera at bytes 7097 and 7098, walker i at 7091 + 3i).
  - Programs: 0 restarts, 1 and 8 face, 2 to 5 step up, down, left, right, 6 follows the trail, 9 waits, 14 takes the player's input.
  - Sub 12 (`mode`, `name.eve`) loads the places: 12-byte records of id, left, top, right, bottom and the facings they take (up 1, down 2, left 4, right 8), up to 0xffff. A walker of size w × h at (x, y) is in a place when x ≤ right, x + w > left, y + 1 ≤ bottom, y + h > top and its facing is allowed. Sub 13 returns the place the hero is in.
  - Sub 30 lets the player walk and returns the place reached. Arrow keys step a tile, Shift or Ctrl two; holding the mouse walks towards the pointer in eight directions. The hero is stopped by walls along the rows y + 1 to y + h − 1 and slides along them as the program does. A step takes 33 ms (`/Config/MapSpeed`: 66, 33 or 16).
  - The view is 40 × 23 tiles with the program's scroll margins (20 and 12). Sub 27 and 28 load the map picture and its overlay; the picture is drawn from the camera, then the walkers sorted by y (144 × 144 frames: column [864, 0, 0, 864][facing] + (frame mod 6) × 144, row facing / 2 × 144; sheet slot 0 (sub 25) is the hero, slot 1 the others), then the overlay.
  - Under Wine the first map frame matches within a level or two per pixel away from the edges, and a step up into the home door enters the house as in the original.
- The status plate (the program's `CStatusPlate`, `cal_p_<lang>.akb`), up over the room, the walking maps and the selections: the calendar leaf (gray, red on Sunday, blue on Saturday) at (6, 0) with the day and weekday, the clock at (241, 13) (`HH:MM` on a 12-hour dial, am or pm), the money at (541, 13) (with a comma from 1,000 yen) and the game mode diamond at (841, 9). The values are the scripts' globals 4 (weekday, 0 for Sunday), 7 (month × 100 + day), 12 (money in hundreds of yen) and 19 (hours × 100 + minutes). The room and selections put it up and take it down when they end; the walking maps put it up and leave it, so it stays over the place reached; `Game.10` puts it up (28), takes it down (29) and draws it again (30). Matches the original under Wine.
- The menu button (`menu_m_p.akb`, 186 x 188 cells, rows normal, hover, pressed, disabled) at (1729, 2), up with the status plate. It opens six buttons (164 x 164 cells) down the right edge from (1655, 111), alternately 85 pixels right: settings, save, load, backlog, schedule and map jump; `mesmenu.carea` (a byte per screen pixel) says which is under the pointer. The map jump shows only on the walking maps, the backlog button only once the log has an entry. Placed as in the original under Wine. Settings opens the settings screen, save and load the save screen, backlog the backlog; schedule and map jump are not done, so their buttons are gray.
- Faces in conversations, read from the executable (not compared under Wine yet):
  - The engine's `Anime` class runs the frame programs of an `.ax` file: 10 groups of 32 programs, then cells of seven numbers (kind, source x, y, width, height, screen x, y) from byte 1280, then the programs. Their operations draw a cell, wait, hold (where a program stops or pauses when asked), loop on two levels, start again and end. Every program that runs takes a step a frame. The cells come from the picture in sprite 5 (`Sprite.1(5, name)`) and are drawn into the background, so a cell stays until the background is loaded again; a cell of kind 3 puts the background back under itself first.
  - A conversation loads its pictures first (`c_ce.mes` for one, `irce.mes` for two people): the standing picture, then the face coming in as three pictures laid onto it, then the sheet (`can.akb`, 176 x 192 faces, an expression a column) and its programs (`c.ax`, a blinking loop per expression).
  - Pictures over the background are objects of `Game.13`, by id (the library only uses id 0, at 100 %): sub 0 `(frames, alpha, picture, id)` puts a picture the size of the screen up (`c.akb`, a standing character), sub 1 `(frames, id, picture)` lays a picture of the same size onto an object (the faces; without the object it does nothing), sub 4 `(frames)` fades all objects out (library function 39, at the end of each conversation) and sub 2 forgets those that faded. While an object is up, the `Anime` cells are drawn onto it instead of the background. The original fades while the program goes on; here a fade is a cross-fade of the screen.
  - `Game.34(index, group, name)` changes the face of a person (the group) to an expression (the program); it starts the program unless it already runs. The name is not used.
  - The speaker's icon (`message_icon.akb`, 144 x 144 cells four to a row) for some lines: `Game.10(n, 15)` shows icon n (from 1) at (78, 848), `Game.10(16)` takes it away. Values of `Game.10` below 10 do nothing in the program.
  - `Anime.10` (pause at the next hold) and `Anime.12` (wait for a program to end) do not wait here: the first pauses at once, the second runs the program to its end at once.
- The save and load screen of the system menu, read from the executable and matched under Wine:
  - The system menu screens share a column of diamond buttons down the left edge (`menu_p.akb`, 164 x 164 cells, rows normal, hover, current, gray; click areas from `sysmenu.carea`): settings, sound, save, load, log, title, quit and back. Below the screen run a strip and the hint bar, which shows the button's line of `SYSTEM_<lang>.MAP` (lines 79 to 86 for the column, 132 to 154 for the save screen) in the small bitmap font. Log opens the backlog (gray while the log is empty); settings and sound open the two parts of the settings screen.
  - `save_bg.akb` or `load_bg.akb`, then ten cards of `saveload_p_<lang>.akb` (324 x 350; columns no data, data, the newest save; rows normal, hover, pressed) at the places of `saveload.area`. A card holds its number (`001`, `A01`, `Q01`), when the save was made (`yy/mm/dd hh:mm:ss`), its picture (280 x 158) and the game's day, weekday, time and money then, all in the sheet's own digits. The black field under them is the player's comment, which is not kept here.
  - Fourteen pages of manual saves, A1 and A2 for auto saves and Q1 for the quick save; while saving, those three tabs are gray squares. Above the cards are the newest quick save (one click quick saves, or loads) and the newest auto save (one click loads).
  - Cards, headers and buttons that do nothing are shaded with black at half. The left button at the bottom switches the deletion mode (loading only), the right one deletes everything.
  - The yes/no dialog of `dialog_p_<lang>.akb`: the screen shaded, the box at (520, 375) (with the comment field when saving; typing is not done), the question strip by its own order (save, load, title, quit, overwrite, delete, reset, ...), yes, no and "don't show again" at the places of `dialog.area`.
  - A save made at the room or another screen that answers the script holds the message before it, as everywhere in this runtime; loading it shows that message, then the screen again.
- The backlog, read from the executable and matched under Wine (opened from the menu, the system menu's log button or the wheel):
  - `log_bg.akb`, then three entries 290 pixels apart. An entry has a star and rule of `log_p.akb` (rows 816 to 870) at (97, 106), the speaker's name from (146, 112) and up to three lines of text from (97, 162), both in the message window's bitmap font and wrapped as there. A log of fewer than three entries sits in the bottom rows.
  - The buttons are columns of `log_p.akb` with rows normal, hover, pressed and gray, at the places of `backlog00.area`: J (go back to the entry) at x 536, the voice button at 764 and the check box at 1028 (ticked in row 3) for entries with a voice, the scroll arrows at 928 (down from row 4), UP and DOWN (a page) at 0 and 167, play marked voices at 334 and back at 626.
  - The scroll bar's thumb (columns 956, 980 and 1004 by state) is a top cap, middle pieces of up to 210 pixels and a bottom cap. It is 863 pixels long less two per entry past the first three, moves two pixels per entry, and is left out while all entries fit.
  - The hint bar shows lines 214 to 223 of `SYSTEM_<lang>.MAP` at size 24 (the bitmap font scaled), a second line 25 pixels below the first.
  - Going back to an entry asks the dialog's "jump to this log entry" question.
- The settings screen, read from the executable and matched under Wine (two pages, SYSTEM 1 and SYSTEM 2):
  - Page 1 is `system2_bg_<lang>.akb` with `config00.area`: game mode (classic or easy), screen speed (three places), text speed, read-text skip and auto mode (sliders of eleven places with an ON/OFF switch), punctuation pauses, language, map speed (three places), keep auto and keep skip after a choice, auto save at a choice, fixed text area, tooltips, and the window colour (R, G, B) and transparency (T). Page 2 is `system1_bg_<lang>.akb` with `config01.area`: the ten confirmation switches and the six right-click choices.
  - The parts are in `system_p_<lang>.akb`: switches 86 x 50 (ON rows normal, hover, pressed, then OFF the same), knobs 36 x 60 (a three-place slider puts the chosen row's knob on its place), the game mode buttons, the language buttons (rows normal, hover, current), the right-click choices and the page tabs (rows normal, hover, current, gray). A knob runs from the left end of its area to 36 pixels short of the right end. A speed slider's knob is left out while its switch is off.
  - The colour box shows the game screen behind the menu through the window colour, with "Message Sample" typing out at the text speed.
  - The hint bar shows lines 57 to 78 of `SYSTEM_<lang>.MAP`. The reset button puts back the defaults of these pages (the language stays); a new install has those of the original under Wine.
  - Choosing a language changes the text and the screens at once.
- The sound page of the settings, read from the executable and matched under Wine:
  - `sound2_bg_<lang>.akb` with `config02.area`; each character slot in use gets its panel copied from `sound1_bg_<lang>.akb` (784 x 152 from (280, 469), slots 792 and 160 pixels apart), so the empty slots of the last page keep only their dashed outline.
  - Master, music, effects and voice: an ON/OFF switch, the value in the sheet's digits (25 pixels apart, the per cent sign 28 after the last, right-aligned at fixed places) and a slider. A slider is the dark bar of `sound_p_<lang>.akb`, a pink fill up to the knob (red while the pointer is on it) and the knob, which travels 594 pixels; with its switch off only the dark bar shows. The effects and voice sliders sit 4 pixels lower in their areas than the others.
  - Fourteen characters and everyone else, six to a page over three pages (tabs and arrows from the sheet, rows normal, hover, current, gray): face (150 x 150), name (230 x 46), value, switch and slider. A voice file belongs to a character by its first letter, the letters of their sample voices (`v9998.ogg`, `l9998.ogg`, ... in page order, listed in the executable); every other letter is the last group.
  - Letting go of a slider plays its sample: `pinpon.wav` for effects, `v0029.ogg` for voice, the character's sample for a character (when it plays is a guess). "Click to stop voices" and the reset button (sound settings only) are there; the hint bar shows lines 87 to 131.
  - A new install starts music at 55 %, effects at 70 % and "click to stop voices" off, as under Wine.
- Headless, `silky-run snap` plays the opening, looks around the hero's room, sleeps an hour, then walks the town to one place after another (the shortest way, without walking through other places), looking at and talking to everything inside; it runs through more than two hundred visits over several days.

## Not done yet

- The schedule and the map jump; the settings screen's fade in, and its classic mode, map speed and "confirm to jump to events" settings, which are kept but change nothing yet; the menu's slide-out animation; save comments. While the dialog is up, the original keeps the button that opened it pressed and its hint shown; here they go back to normal.
- The walking maps: walking to a target (`Game.20(15)`, the program's path search), the place labels of game mode 1 (sub 32) and saving while on a map.
- The extras screen `Game.53` (also reached from the hero's computer) closes at once, returning -1.
- More than four options (the original pages them with arrows), and options grayed once taken.
- The other place screens, the name entry (`Game.18`) and the other screens.
- Chinese text: the layout of full-width characters is assumed (two cells each, a character a word); it is not checked under Wine yet.
- The character picker's sounds (`ses_003.wav`, `ses_004.wav` for the quick buttons and a system sound for a card).
- Calls that change nothing here: `Select.0(-1)` (the engine readies the layers of its selection bars), `Game.20(24)` with a `.pal` file (not a sub command of this build), `Game.27` (the game's own mouse pointer on and off) and `Game.35` (marks for the "skip scenes" setting).
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
| `Game.17` area file, plain area selection (`CAreaSelect`: set up, result) | 0x4e2238, 0x46d960, 0x46dd20 |
| `Game.29` hours dial (`CTimeSelect`): call, set up, drawn | 0x4e250c, 0x4dd5d0, 0x4ddc70 |
| Hours dial: button drawn, clicked | 0x4ddae0, 0x4dd750 |
| Selection bars built, placed (`CNormalSelect`) | 0x4c5f50, 0x4c62a0 |
| `Game.25` calendar written | 0x4e4ca0 |
| Bitmap fonts loaded | 0x4a4340 |
| One glyph drawn with its outline | 0x4a44e0 |
| Message text laid out (words, `\n`, `\i`) | 0x4a6a00 |
| Line breaks before closing punctuation | 0x4a7150 |
| `Game.20` walking maps (`CCps`): sub commands | 0x493c20 |
| Walking maps: walker programs, hero steps, wall slides | 0x495c20, 0x496250, 0x4973f0 |
| Walking maps: input, way to the pointer, places reached | 0x496eb0, 0x4978f0, 0x4960a0 |
| Walking maps: drawn, path search | 0x497e20, 0x4983e0 |
| Status plate (`CStatusPlate`): made, drawn, time text | 0x4d8410, 0x4d8a90, 0x4d9120 |
| Status plate shown, hidden (`Game.10` subs) | 0x4d8610, 0x4d87e0, 0x4e2bb0 |
| Menu button: made, opened, closed, hit, drawn | 0x4c7f40, 0x4c85a0, 0x4c8790, 0x4c8980, 0x4c89d0 |
| Menu buttons available, clicked on the maps | 0x4c8b70, 0x499ce0 |
| `Game.50` to `Game.53` extras screens | 0x4e5ff0 |
| `Game.50` album: opened, drawn (grid, pictures), clicked | 0x473750, 0x474a50, 0x474530, 0x4738d0 |
| Album percentage, ending check | 0x475310, 0x4759c0 |
| Album picture tables (with the update archive, without), page counts | 0x58e900, 0x58ee28, 0x58edec, 0x58f314 |
| `Game.51` memory reel: tables read, opened, drawn, clicked, percentage | 0x4d1630, 0x4d1d50, 0x4d2df0 and 0x4d2950, 0x4d1ed0, 0x4d3770 |
| `Game.52` music mode: init, drawn, element drawn, clicked, cancelled | 0x4bb0e0, 0x4bb6d0, 0x4bc7b0, 0x4bbfc0, 0x4bc3f0 |
| Music mode: open, frame, play, stop, track list | 0x4bbc70, 0x4bbdf0, 0x4bcb40, 0x4bd1a0, 0x4bda00 |
| Music mode: volume drawn, dragged, disc buttons | 0x4bd580, 0x4bd2f0, 0x4bd8e0 and 0x4bddf0 |
| Music mode tables (discs, track names), track flags | 0x58f38c and 0x58f39c, 0x59dc70 to 0x59dd4c |
| Memory reel scene tables (with the update archive, without; 14 per girl), page counts | 0x58f770, 0x58f3b0, 0x58f734, 0x58f6f8 |
| `Anime` class calls (`CFuncAnime`), frame step, update | 0x454b00, 0x427180, 0x4270f0 |
| `Anime` cell drawn over the background (kind 3) | 0x426a00 |
| Speaker icon shown, taken away (`Game.10` 15, 16) | 0x4e2d11, 0x4e2e41 |
| `Game.34` face, `Game.13` pictures, a picture laid in | 0x4e26f5, 0x4e3ae0, 0x48a510 |
| `Game.13` object made, faded out | 0x48a340, 0x4899b0 |
| `Game.27` mouse pointer, `Game.35` scene marks, `Game.36` character picker | 0x4ba9c0, 0x4e5df0, 0x48b610 |
| Character picker: laid out, a card or button drawn, clicked, side button gray | 0x48c370, 0x48c020, 0x48b980, 0x48c790 |
| Save screen: set up, a card drawn, the newest quick and auto saves drawn | 0x4c8ea0, 0x4ca7e0, 0x4cb2c0 |
| Save card digits: number, date made, day, time, money | 0x4cc1c0, 0x4cc3f0, 0x4cc670, 0x4cc800, 0x4cca40 |
| System menu side buttons and strip (`SystemParts`) | 0x4dbb30 |
| Yes/no dialog drawn | 0x49af00 |
| Backlog entry drawn (name and text layers, buttons) | 0x4711c0 |
| Backlog scroll thumb sized and placed | 0x471700, 0x471850 |
