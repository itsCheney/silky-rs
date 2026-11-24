# なないろリンカネーション

English | [简体中文](nanairo.zh-CN.md)

Japanese build 2014. The runtime knows the game by `nanairo.exe` in its folder. Program version 1.0.0.2, linked 2014-09-09. Its size and SHA-256 are not recorded yet. Engine profile: [engine-2017](../builds/engine-2017.md). Its own `Game` commands: [game-commands.md](../game-commands.md#なないろリンカネーション).

Work in progress. The title menu, the message window and the pictures on the stage are drawn. Several screens and effects are not.

Measured under Wine: the place and pitch of the message text and the place of the speaker's name. Everything else below was read from the executable or from the scripts. The address table at the end gives where.

## What works

### Title menu

- Five entries (new game, continue, load, settings, quit) sit on a 349 x 449 layer at (145, 255). The round bonus button at (467, 517) switches to the extras page and back. The extras page has CG, scene and music on a 352 x 209 layer at (144, 313), and a back button.
- Cells come from `title.akb`, click areas from `title.area` and `title2.area`. Each cell has state rows for normal, hover and pressed.
- Unlocks are byte variables. Settings and the bonus button need byte 3001. CG needs 3002, scene 3003 and music 3004. Until then they are grey.
- `Game.18` returns 1 for a new game, 5 for quit, and 257, 258, 259 for CG, scene and music. Continue, load and settings open the engine's screens.

### Message window and buttons

- The window is the alpha mask in the top 199 rows of `meswaku.akb`, filled with the window colour. Its top is at 521 (`/Config/MesWnd/Pos` of a new install).
- Default colour is 19, 70, 133 and the default transparency 50, each of 268. The program scales each by 255 / 268.
- Buttons are cells of `meswaku.akb` laid over the click areas of `mes.area`, which are relative to the window. Ids 0 to 12: save, load, quick save, quick load, voice loop, hide, auto, skip, next choice, previous choice, title, backlog, settings. Cells have four state rows: normal, hover, pressed, disabled. Auto, skip and voice loop show pressed while on.
- Click wait: a 29 x 28 mark at (1050, window top + 154). It is shown as one still frame; the sheet holds its frames in a row from x = 691.

### Text

- Size 24, a full-width character advances 28 pixels, text starts at x = 258, the first line's baseline is at y = 637 and lines are 32 apart. These were measured under Wine. Glyphs have an outline.
- The line length is not measured. 784 pixels (28 cells) is used.

### Speaker name and face window

- `Game.15(name)` sets the speaker. The name is white, size 22, from x = 258 with its baseline 80 below the window top, straight on the window with no plate.
- The face window is a 532 x 544 crop of the character's picture, drawn at half size at (0, 448). The crop's corner depends on the character. A face picture is drawn over the body when it differs. `12.5` and `12.6` show and hide it, `12.7` empties it, `12.8` shows a whole picture.

### Standing characters

- Pictures are 1600 x 1600, named `t` + two letters + digits. The two letters give the character: iy 伊予, ai アイリス, ki 桔梗, fu 芙蓉, ko 琴莉, ao 葵, az 梓, yu 由美.
- The figure's left edge is at (1280 - 16 x size) / 2. A side slot is 345 pixels further out. Above 50 % the figure is lifted by (2 x size - 100) x K / 100, with K by character (iy 400, ai 280, ko and ao 200, ki, fu, az and yu 140, others 0).
- `13.0` places a figure, `13.1` removes one, `13.7` removes all, `13.5` sets the expression layer, `13.9` zooms (at once here), `13.2` crossfades every change over the longest time given.
- Backgrounds and pictures are in sprite 2 and lie under the figures. Sprite 3 is a sheet store that is never shown.

### CG registry

- The scripts name the picture (`14.10`) and its layers in slot order (`14.1`). A name is kept only if it begins with `H`. When the scene is committed, the combination is looked up in a table of 493 rows. A match sets byte 3200 + row, and also bytes 3001 and 3002. The CG mode reads these bytes.
- Names are compared without case and without `.akb`.

### Event pictures and effect pictures

The effects are sprites of the shared effect stage: keyframes of fades, zooms and turns stepped once a frame (1/60 s). Where a call makes the script wait, it waits as long as the original's keys last. Times are scaled by `/Config/DisplayEffect/Speed` (half at 1, almost nothing at 2), as for the other transitions. With `/Config/SceneSkip` at 1 these calls do nothing. Effects are not saved; the clean up calls, the end of a chapter and the title menu take them away.

- `Game.22.1` puts up `kirara.akb`, clear. `22.2` makes it flash out: 16 frames, opacity 255 to 0 while it grows to 1.2 times about (640, 360); the script waits.
- `Game.1001.0` grows the picture on screen to 1.2 times over 16 frames (the script waits), then fades `HG091010_h05_eff.akb` in, holds it 24 frames and starts again, for ever. `1001.1` fades the effect and the enlarged picture out over 24 frames (the script waits) and ends the effect. The fade in's length and the zoom's centre are guesses (see below).
- `Game.1002.0(name, frames)` fades the scene movie in over `frames` frames and the script waits; `1002.1(frames)` fades it out and stops it. Movies are not decoded: the picture of the same name stands in when the game has one (`HG070502.akb`, `HG030603.akb`), otherwise nothing is drawn and only the time passes. `1002.3` and `1002.4` are the same pair for the effect movies whose opacity comes from the movie (`HG011701.mov`); they draw nothing.
- `Game.1003.0(name, frames)` fades the effect picture (`HG091013_eff.akb`) in over `frames` frames and the script waits; it stays. `1003.1(frames)` fades it out and releases it.
- `Game.1005.0` fades `HG080403_eff.akb` in over 64 frames, then the script waits one round of its slow pulse (24 frames held, 96 down, 80 up, 200 in all); the pulse goes on for ever. `1005.1` takes it away at once.
- `Game.1006.3` (and `.13`, the extras version) shows a figure picture (`HG090202_h01.akb`) and a picture of lines (`HG090203.akb`). The lines fade in and grow to twice their size over 8 frames (`.3` only), then turn 30 degrees one way and back and the other way and back, 12 frames a step, for ever. `1006.20` shows a beam of light (`HG090205.akb`) with a figure picture (`HG090205_h02.akb`) over it that fades from 52 to 169 and back over 128 frames each way. Any call of `Game.1006` first takes away what the last one showed. `1006.100` and the other clean up calls (`1001.3`, `1002.2`, `1003.2`, `1005.2`) take away their effect.
- `Game.23.0(from, to, _)` moves the picture in sprite 2, which is wider than the screen (2100 pixels, 820 more), sideways one pixel a frame from window position `from` to `to`. A quarter as many steps at speed 1, none at speed 2. The script waits (about 13 s).

### Other commands carried out

`Game.19` fills byte flags, `Game.26` ends the message window before the next script, `Game.27` and `Game.10` start up, `Game.22.0` registers the picture shown. See [game-commands.md](../game-commands.md#なないろリンカネーション).

## What does not work yet

- Calendar screen (`Game.39`, shown at date changes): the script goes on at once.
- Ending overlay (`Game.50`): the script goes on at once.
- Scene movies are not decoded (`Game.1002`): a still stands in or nothing is drawn. The movie laid over the picture by `Game.1001.0` is left out.
- Not read far enough to guess the numbers: the fade in of `1001.0` (24 frames used), its zoom centre (the screen's middle used), the low point of the `1005` pulse (half used), what the `1006.3` figure and lines do exactly (only the lines turn, about the middle of the screen), whether the `1006.20` pulse repeats (it does here) and which way `Game.23.0` moves (from the first value to the second). Effects in progress are not saved.
- `Game.1004` and `1007` to `1009` are accepted and draw nothing. `Game.1006` subs 0 to 2 and 4 to 9 (and 10 to 12, 14, 18, 19 of the extras) are reported; they only take away what the last call showed.
- Staff roll (`Game.34`) is reported.
- `Game.3`, `17` and `40` are reported.
- `Game.30` to `32` use the shared extras screens, not the game's own sheets.
- Standing character calls 6, 8 and 11 to 15 (side, effects, flags, motion) are accepted and not drawn. The flag of `13.5` is not understood.
- Text line length and the click-wait mark's animation are not measured.
- The figure positions are not yet compared with the original.

## Game commands

| Command | Does |
| --- | --- |
| `10` | Start up; nothing to do |
| `12.4(face, body)`, `12.5`, `12.6`, `12.7`, `12.8(picture)` | Face window |
| `13.0`, `13.1`, `13.2`, `13.5`, `13.7`, `13.9` | Standing characters |
| `14.10(name)`, `14.1(slot, name)`, `14.11`, `14.5`, `14.7(n)` | CG registry |
| `15(name)` | Speaker name |
| `18` | Title menu |
| `19(value, end, start)` | Fill byte flags |
| `22.0` | Register the picture shown |
| `26` | End of chapter |
| `27` | Message window start up |
| `22.1`, `22.2` | Sparkle picture: put up, flash out |
| `23.0(from, to, _)` | Sideways scroll of the picture in sprite 2 |
| `1001.0`, `1001.1` | Event picture zoom with an effect picture on top; end |
| `1002.0(name, frames)`, `1002.1(frames)`, `1002.3`, `1002.4` | Scene movie fade in and out (a still stands in) |
| `1003.0(name, frames)`, `1003.1(frames)` | Effect picture fade in and out |
| `1005.0`, `1005.1` | Effect picture with a slow pulse; end |
| `1006.3`, `1006.13`, `1006.20`, `1006.100` | Event animator: swaying lines, glowing figure; clean up |
| `1001.3`, `1002.2`, `1003.2`, `1005.2` | Clean up: take the effect away |
| `39`, `50`, `1004`, `1007` to `1009` | Accepted, nothing drawn |

Calls that return a value, read from the executable: 18, 30 to 32 and 34.

## Where things are in the executable

Addresses in `nanairo.exe` as linked (image base 0x400000). The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` dispatcher; `CFuncExec` vtable | 0x46e9a0; 0x5090a0 |
| Game object (singleton) | 0x494510 |
| `Wait.0` (class 28): dispatcher; wait loop | 0x4918a0; 0x473e60 |
| Game class registration order | 0x47e480 |
| Standing characters: handler, place and zoom | 0x46ee10, 0x443ec0 |
| Face window: handler, crop and draw | 0x46eca0, 0x4590b0 |
| Speaker name (`Game.15`) | 0x46f830 |
| CG registry: handler | 0x46f3a0 |
| CG table (493 rows, flags 3200 + n) | 0x533cf0 |
| `Game.22`: handler; workers of subs 1 and 2 | 0x46fa10; 0x46e380, 0x46e4f0 |
| `Game.23` scroll: handler | 0x4734d0 |
| `/Config/SceneSkip` read; `/Config/DisplayEffect/Speed` read | 0x474ac0; 0x509390 |
| Movie player: open, stop | 0x42a1e0, 0x42b430 |
| Event animator (`Game.1006`): move loop helper | 0x44ca50 |
| `Game.18` handler; title object vtable; its slot 0xb4 reads byte N | 0x46faa0; 0x508e30; 0x487560 |
| Title: buttons drawn, layers placed | 0x46d050, 0x46c940 |
| `Game.19` fill | 0x473430 |
| `Game.26` chapter end | 0x454980 |
| Message window: layer creation (window, wait mark, face layer) | 0x455f60 |
| Message window: colour and transparency (K at 0x50d328) | 0x456160 |
| Window buttons: layout; button layer placed at (828, Pos + 23) | 0x451af0; 0x4529b0 |
| Balloon (nine-slice) | 0x454050 |
| `Game.34` staff roll | 0x46a4a0 |
| Calendar (`Game.39`): constructor, run | 0x4398b0, 0x439a50 |
| Ending overlay (`Game.50`) | 0x4731d0 |
| `Game.1001` to `1006` handlers | 0x46fd50, 0x470750, 0x471370, 0x471b20, 0x471e80, 0x472570 |
