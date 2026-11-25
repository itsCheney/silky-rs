# なないろリンカネーション

English | [简体中文](nanairo.zh-CN.md)

Japanese build 2014. The runtime knows the game by `nanairo.exe` in its folder. Program version 1.0.0.2, linked 2014-09-09. Its size and SHA-256 are not recorded yet. Engine profile: [engine-2017](../builds/engine-2017.md). Its own `Game` commands: [game-commands.md](../game-commands.md#なないろリンカネーション).

Work in progress. The title menu, the message window and the pictures on the stage are drawn. Several screens and effects are not.

Measured under Wine: the place and pitch of the message text and the place of the speaker's name. Compared with the original under Wine: the calendar's layout and dates, the standing characters' places and sizes (six scenes of the first route, same to the pixel), and the click-wait mark's place, pictures and steps. Everything else below was read from the executable or from the scripts. The address table at the end gives where.

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
- Click wait: a 29 x 28 mark at (1050, window top + 154), shown while a message waits for a click. Its frames lie in two rows of `meswaku.akb` from x = 691, 29 apart: a triangle (11 frames, y = 461) and a speaker (14 frames, y = 433). A timer steps it: frames 0 to 4 play in (100 ms each), then the loop runs at 400 ms a frame, frames 5 and 6 for the triangle and 5 to 8 for the speaker. While a voice line plays the mark is the speaker, otherwise the triangle. When that changes, the picture on screen plays out (triangle frames 7 to 10, speaker frames 9 to 13, 100 ms each) and the other one plays in. Every new wait starts from frame 0 of the triangle (or the speaker if a voice is playing).

### Text

- Size 24, a full-width character advances 28 pixels, text starts at x = 258, the first line's baseline is at y = 637 and lines are 32 apart. These were measured under Wine. Glyphs have an outline.
- Line length: every message starts with the text box at (256, 90) to (1124, 186) on the window layer (the same layer coordinates as the window at y = 521), 868 pixels wide. The program draws a character, moves the pen on by its width (14 a byte, so 28 for a full-width character), and when the pen has reached the right edge the next character starts a new line at the left edge and one pitch (32) lower. So a line holds 31 full-width characters, wrapping is by position and so by cell, and there is no line-start or line-end rule (no such table or character test is in the program). Drawing stops when a wrap would put a line within one pitch of the bottom edge (186). The scripts break their lines by hand (`text 0`) at 28 cells, up to 30 with closing punctuation, and show at most three lines to a message, so the program's own wrap does not occur in the shipped scripts. Line length is read from the program and the scripts, not measured under Wine.
- Text style comes from script variables set at start up (`START.MES`, `LIBLARY.LIB`): cell 28, line pitch 32, white text with a black edge. The program makes its text fonts with `Font.0(28, 28)`.

### Speaker name and face window

- `Game.15(name)` sets the speaker. The name is white, size 22, from x = 258 with its baseline 80 below the window top, straight on the window with no plate.
- The face window is a 532 x 544 crop of the character's picture, drawn at half size at (0, 448). The crop's corner depends on the character. A face picture is drawn over the body when it differs. `12.5` and `12.6` show and hide it, `12.7` empties it, `12.8` shows a whole picture.

### Standing characters

- Pictures are 1600 x 1600, named `t` + two letters + digits. The two letters give the character: iy 伊予, ai アイリス, ki 桔梗, fu 芙蓉, ko 琴莉, ao 葵, az 梓, yu 由美.
- The figure's left edge is at (1280 - 16 x size) / 2. A side slot is 345 pixels further out. Above 50 % the figure is lifted by (2 x size - 100) x K / 100, with K by character (iy 400, ai 280, ko and ao 200, ki, fu, az and yu 140, others 0).
- `13.0` places a figure, `13.1` removes one, `13.7` removes all, `13.5` sets the expression layer (its first value is not read by the program), `13.9` zooms (at once here), `13.2` crossfades every change over the longest time given and waits until the motions below are over.
- `13.6(side, slot)` sets the order the figures overlap in: the figure's layer takes depth side + 5, so a higher side is in front. A new figure starts with side 2 in the centre, 1 on the left and 0 on the right; the scripts bring the speaker forward with 3. It is kept while the same character stands there.
- `13.8(place, slot)` slides the figure to the place of another slot (0 centre, 1 left, 2 right; the left edge as for a slot) over 12 frames; the script uses `13.8(1, 0)` to make room for a second figure. The slide is linear (not confirmed).
- `13.10(slot)` and `13.11(slot)` move the figure over 12 frames of 16 ms: `13.10` up 78 pixels and back, `13.11` (the one the scripts use, after a figure appears) down 78 pixels and back. Both follow a fixed table of offsets.
- `13.12(pixels, axis, slot)` is a hop (axis 0: up by half, the full amount, half, then at rest) or a sway (axis 1: left, right, at rest), one step each frame.
- `13.13(kind, 1, 0, slot)` starts a glow and `13.13(kind, 1, 1, slot)` lets it go; `13.14` takes it away at once (the scripts send it before each commit). The glow darkens the background and what lies under the figures (black, three quarters), and fades in over 24 frames: kind 0 is `teffect.akb` (green light with sparks) behind the figure, squeezed to the figure's width and moved 80 x size to the right in the centre slot (376 in the right slot); kind 1 is `tai-effect.akb` (white petals) over the whole screen. When movies are on the original plays `HG060002_eff.mov` over the petals; movies are not decoded here, so only the picture is drawn. Letting it go fades it out over 24 frames. The curve of the fades is not confirmed.
- `13.15(a, 0, slot, slot)` is the same glow for two figures at once: the green glow for 葵, petals for the others.
- Backgrounds and pictures are in sprite 2 and lie under the figures. Sprite 3 is a sheet store that is never shown.

### CG registry

- The scripts name the picture (`14.10`) and its layers in slot order (`14.1`). A name is kept only if it begins with a capital `H`. A layer goes to the first free place up to its slot, or replaces the one at the slot. When the scene is committed, the combination is looked up in the game's CG table of 644 rows: the picture and the first six layers must equal a row's, place by place. A match sets the byte flag of that row and byte 3001. Any registered picture with a name also sets byte 3002 (the CG tab of the extras screens).
- The flags of the table run from 3200 to 3845 and are 3200 + row only up to row 492; row 493 holds 3694. The table is in `cg_table.rs`, the rows in the game's order, so the flag comes from the row and not from its number.
- Names are compared without case and without `.akb`.
- Not read: what the program's settings block holds in the field that stops registering when it is 1 (the registry does nothing then; probably while a scene is replayed from the scene screen, which is not here yet).

### Event pictures and effect pictures

The effects are sprites of the shared effect stage: keyframes of fades, zooms and turns stepped once a frame (1/60 s). Where a call makes the script wait, it waits as long as the original's keys last. Times are scaled by `/Config/DisplayEffect/Speed` (half at 1, almost nothing at 2), as for the other transitions. With `/Config/SceneSkip` at 1 these calls do nothing. Effects are not saved; the clean up calls, the end of a chapter and the title menu take them away.

- `Game.22.1` puts up `kirara.akb`, clear. `22.2` makes it flash out: 16 frames, opacity 255 to 0 while it grows to 1.2 times about (640, 360); the script waits.
- `Game.1001.0` grows the picture on screen to 1.2 times over 16 frames (the script waits), then fades `HG091010_h05_eff.akb` in, holds it 24 frames and starts again, for ever. `1001.1` fades the effect and the enlarged picture out over 24 frames (the script waits) and ends the effect. The fade in's length and the zoom's centre are guesses (see below).
- `Game.1002.0(name, frames)` fades the scene movie in over `frames` frames and the script waits; `1002.1(frames)` fades it out and stops it. Movies are not decoded: the picture of the same name stands in when the game has one (`HG070502.akb`, `HG030603.akb`), otherwise nothing is drawn and only the time passes. `1002.3` and `1002.4` are the same pair for the effect movies whose opacity comes from the movie (`HG011701.mov`); they draw nothing.
- `Game.1003.0(name, frames)` fades the effect picture (`HG091013_eff.akb`) in over `frames` frames and the script waits; it stays. `1003.1(frames)` fades it out and releases it.
- `Game.1005.0` fades `HG080403_eff.akb` in over 64 frames, then the script waits one round of its slow pulse (24 frames held, 96 down, 80 up, 200 in all); the pulse goes on for ever. `1005.1` takes it away at once.
- `Game.1006` is the event animator, two figure pictures (A, `HG090201_h01.akb`, and B over it, `HG090202_h01.akb`) and sometimes a third. Each sub changes what they show and starts motion; the script waits only for 4, 5 and 6. Times are in frames (16 ms) and scaled by the effect speed. 0 and 10: A at (0, 100) rises 20 pixels (fading in for 0), then bobs 20 pixels up and down for ever. 1 and 11: A pans and fades out and in, 64 frames each way. 2: B fades in over 8 frames and bobs. 3 and 13: the figure and `HG090203.akb`, whose lines grow to twice their size over 8 frames (fading in for 3) and swing 30 degrees either way, 12 frames a step. 4: A dissolves into the picture the script put on sprite 2, in 12 steps (about 0.2 s, the script waits). 14: A takes that picture at once and bobs. 5 and 6: the animator fades away (5) or wipes away through rule 18 (6) over 16 frames, the script waits, and it is gone. 7, 12 and 17: A bobs 20 pixels down and up. 8 and 18: `HG090204.akb` at 2.5 times its size turns round for ever over 14-frame turns, with the figure fading in over it. 9: stops the motion and shows only the big picture. 19: stops and shows the sprite 2 picture still. 20 and 30: a beam (`HG090205.akb`) wipes in through rule 18 over 128 frames while the figure over it fades from 52 to 169 and back over 128 frames each way; 20 hides A and B first. 100 takes the animator away, as do the other clean up calls (`1001.3`, `1002.2`, `1003.2`, `1005.2`). Other subs do nothing. A call of one sub goes on from what the last one left; it does not take it away.
- `Game.23.0(from, to, _)` moves the picture in sprite 2, which is wider than the screen (2100 pixels, 820 more), sideways one pixel a frame from window position `from` to `to`. A quarter as many steps at speed 1, none at speed 2. The script waits (about 13 s).

### Ending

- `Game.50` is the message overlay of the end card. `50.0` prepares, `50.2` takes it away (and is also called alone as a clean up). `50.1(rule, frames, h, w, y, x)` wipes in the rectangle (x, y, w, h) of `moji.akb` (the handwriting) in its own place through the rule picture `NNN.BIN`, over `frames` frames, and the script waits; the piece stays until `50.2`. The rule picture lies on the piece's top left corner, as the pictures `150.BIN` to `153.BIN` are made for. The four calls write the line piece by piece, and the signature last. With `/Config/SceneSkip` at 1 nothing is done.
- `Game.34(staff picture, strip)` is the staff roll. The music `31.WAV` starts (the scripts do not start it) and the script waits 130.33 s, the length of the music. The ending's `staffNN.akb` fades in and out over 5 s, `staff_p2.akb` follows from 5 s to 10 s, `staff_p3.akb` from 10.33 s to 13.33 s, then the credit strip (`staff_p4.akb` to `staff_p7.akb`, 575 x 3214) scrolls up from below the screen for 106.7 s. At 120 s the logo (`staff_p8.akb`) comes up on black and from 128 s turns into the closing page `staff_p1.akb`. A click, or skip mode, ends the roll at once; the music stops with it. The call leaves 0 on the stack, as the scripts ignore the value. It is not cut short by `/Config/SceneSkip`. The message window is taken away for the roll.

### Quit, auto save and the trial notice

- `Game.3` is the quit box. It asks "Do you want to end the game?" (`ExitDialog.akb`: a dim layer over the screen, a band with the question, yes and no buttons and a check box "do not show next time"). Yes (area 0) ends the game; no (area 1) goes back to the script. With the check box ticked the setting is kept and the next `Game.3` ends the game without asking. The same box, with the other questions of the sheet, is the one the window bar and the other quit and confirm buttons use. The result of the box (0 yes, 1 no) was read from the program.
- `Game.17` is the auto save. No script calls it. It saves unless `/Config/AutoSave/Enable` was written as 0 (a new install has it on). Here the save is written at the next message by the same path as the other games' auto save points, and only while the player's auto save setting is on. The original writes the save to its slot 100 at once with the screen as the picture.
- `Game.40(kind)` is the notice of the trial version: it loads `trial.area` or `trial2.area` and opens a web page when the first area is clicked. Those files are not in the archives of this game, so nothing is shown. The call is accepted and does nothing.

### Calendar

- `Game.39(days)` is the calendar of a date change. `days` is how many days the date column scrolls (word variable 100 holds the new day's row in the 31-row date table, August 18 to September 26). White curtains open over the new background, which drifts in behind a fine white veil (`InterludeBG.akb`); the month name (`Interlude.akb`) fades in, the column of dates fades in, a grey bar slides in, the disc lands (12 frames, 1.2 to 1.0 size), and the column scrolls down `days` cells in 32 frames so that the new day ends under the disc. The part of the column inside the disc's window is drawn in yellow while it scrolls. After a hold the curtains leave and everything fades out, about 280 frames in all (16 ms each, scaled by the effect speed). The script waits; a click ends it at once. With `/Config/SceneSkip` at 1 nothing is done. The white fade-out the script puts before it is lifted when the calendar starts, so the new background is up when it ends. Moves and fades are taken to be even and the layers that come in later start hidden. Compared with the original on four calendars of the first route, among them one that scrolls three days: the days shown, the month, the column, the bar and the disc are in the same places, and the column scrolls the same distance in an even move. The original under Wine ran too slowly to compare timing.

### Other commands carried out

`Game.19` fills byte flags, `Game.26` ends the message window before the next script, `Game.27` and `Game.10` start up, `Game.22.0` registers the picture shown. See [game-commands.md](../game-commands.md#なないろリンカネーション).

## What does not work yet

- Calendar (`Game.39`): timing is not compared with the original; the disc's scale centre (62, 62), even moves and fades, and the one frame between its states are assumed.
- Scene movies are not decoded (`Game.1002`): a still stands in or nothing is drawn. The movie laid over the picture by `Game.1001.0` is left out.
- Not read far enough to guess the numbers: the fade in of `1001.0` (24 frames used), its zoom centre (the screen's middle used), the low point of the `1005` pulse (half used) and which way `Game.23.0` moves (from the first value to the second). In the animator: whether the 0.25 size of subs 0 and 10 is a quarter size on screen (it is drawn so, on A), which sides the pans of 1 and 11 move (100 pixels sideways and 5 down are used), what sub 30 does beside leaving A and B, whether the beam's wipe repeats (once), the pivot of the lines of 3 and 13 (the one point of their growth) and the units of the 128 frame fade of 20. In the staff roll the curves inside the stages are not known (pages fade in and out, the strip moves evenly, the logo fades into the closing page), nor the strip's real pace, and the sequence of the last pages is the plainest reading. The wipe of `Game.50.1` uses an even ramp. Effects in progress are not saved.
- `Game.1004` (no script calls it) is carried out as removing every standing character at once, with nothing drawn; the original's exact effect is not read. `1007` to `1009` draw nothing and are accepted: they only tidy caches and the save folder's thumbnail files at start up.
- `Game.30` to `32` use the shared extras screens, not the game's own sheets.
- Standing characters: the movie the glow of `13.13` and `13.15` plays over the petals is not drawn. A face that is put on a figure standing without one fades in over 12 frames in the original; here it appears with the crossfade of the commit. The curves of the slide and of the glow's fades are guessed linear; the exact frame rate of the motions (16 ms steps) and the glow's place for the left slot are not confirmed.

## Game commands

| Command | Does |
| --- | --- |
| `10` | Start up; nothing to do |
| `12.4(face, body)`, `12.5`, `12.6`, `12.7`, `12.8(picture)` | Face window |
| `13.0` to `13.2`, `13.5` to `13.15` | Standing characters |
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
| `1006` subs 0 to 14, 17 to 20, 30, 100 | Event animator; the other subs do nothing |
| `34(staff picture, strip)` | Staff roll (returns 0) |
| `50.0`, `50.1(rule, frames, h, w, y, x)`, `50.2` | Ending message overlay |
| `1001.3`, `1002.2`, `1003.2`, `1005.2` | Clean up: take the effect away |
| `1004` | Remove all standing characters (not used by the scripts) |
| `1007` to `1009` | Accepted, nothing drawn (cache and thumbnail file upkeep) |
| `39` | Calendar |
| `3` | Quit box; yes ends the game |
| `17` | Auto save |
| `40` | Trial notice (accepted, nothing shown) |

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
| Standing characters: side (`13.6`), slide (`13.8`), bow (`13.10`, `13.11`), hop (`13.12`) | 0x444290, 0x444910 and 0x442180, 0x444970, 0x4449e0 |
| Standing characters: per-frame update, bow, hop and fade steps | 0x444f40, 0x4451d0, 0x445300, 0x445080 |
| Standing characters: commit loop; offset tables of bow and hop | 0x444d60, 0x444eb0; 0x50d840, 0x50d830, 0x50d420, 0x50d580, 0x50d590, 0x50d3d0 |
| Standing characters: glow start (`13.13`), stop, remove (`13.14`), two figures (`13.15`) | 0x444550 and 0x4422e0, 0x442cb0, 0x444710, 0x4445d0 and 0x443010, 0x443190 |
| Standing characters: expression layer (`13.5`) | 0x444b90, 0x441ae0 |
| Glow pictures; movie name | 0x50673c, 0x506748; 0x506758 |
| Face window: handler, crop and draw | 0x46eca0, 0x4590b0 |
| Speaker name (`Game.15`) | 0x46f830 |
| CG registry: handler | 0x46f3a0 |
| CG table (644 rows of 36 bytes: flag, then up to eight picture names; ends with -1) | 0x533cf0 |
| CG registration: match and set the flags; start (`14.10`); layer (`14.1`) | 0x43d650; 0x43d840; 0x43d9f0 |
| `Game.3` quit: handler; yes or no box constructor; click; button draw | 0x494c50; 0x449650; 0x449a40; 0x449c90 |
| Box setting read (`/Config/Dialog/%02u/Enable`, default on); table of question to setting | 0x474c40 (path at 0x509490); 0x50d440, 0x50d3c0 |
| `Game.17` auto save: handler; setting read (`/Config/AutoSave/Enable`, path at 0x5094c0); slot writer | 0x46f970; 0x4750d0; 0x463c20 |
| `Game.40` trial notice: handler; object run | 0x472890; 0x46dc60 |
| `Game.22`: handler; workers of subs 1 and 2 | 0x46fa10; 0x46e380, 0x46e4f0 |
| `Game.23` scroll: handler | 0x4734d0 |
| `/Config/SceneSkip` read; `/Config/DisplayEffect/Speed` read | 0x474ac0; 0x509390 |
| Movie player: open, stop | 0x42a1e0, 0x42b430 |
| Event animator (`Game.1006`): dispatcher on the animator object; move loop helper | 0x44a190; 0x44ca50 |
| `Game.18` handler; title object vtable; its slot 0xb4 reads byte N | 0x46faa0; 0x508e30; 0x487560 |
| Title: buttons drawn, layers placed | 0x46d050, 0x46c940 |
| `Game.19` fill | 0x473430 |
| `Game.26` chapter end | 0x454980 |
| Message window: layer creation (window, wait mark, face layer) | 0x455f60 |
| Wait mark: step function (frame, phase, row; timer 100 / 400 ms) | 0x458750 |
| `Game.1004` handler; `1007` (clears object map); `1008`, `1009` (thumbnail files `thm%03u`) | 0x471b20; 0x497fc0; 0x4727c0, 0x472600 |
| Message window: colour and transparency (K at 0x50d328) | 0x456160 |
| Message window: start-of-message text box reset (box 256, 90 to 1124, 186; cursor 256, 90), called through slot 0xe8 | 0x456370, 0x457b34 |
| Text draw: sets the box and pen from the script variables; pen loop (advance per byte, wrap at the right edge, stop near the bottom) | 0x47f6d0, 0x490950 |
| Text fonts: `Font.0` handler; font set made (heights size and size - 2, weights 400 and 700); glyph draw with edge | 0x48b710, 0x483db0, 0x484070, 0x483e80 |
| Window buttons: layout; button layer placed at (828, Pos + 23) | 0x451af0; 0x4529b0 |
| Balloon (nine-slice) | 0x454050 |
| `Game.34` staff roll: handler; set up; worker thread | 0x46fbc0; 0x46a4a0; 0x46a960 |
| Calendar (`Game.39`): handler; constructor; init; screen loop; clean up | 0x4729e0; 0x4398b0; 0x439c20; 0x486d40; 0x439a50 |
| Calendar: cut-out of the date cells; key builder; start; stop for skip; per-frame state function (jump table 0x43b36c) | 0x43a1d0; 0x43a620; 0x43ace0; 0x43ac30; 0x43ae10 |
| Calendar: date table (12-byte rows: month, day, weekday); picture name `InterludeBG.akb` | 0x52db40; 0x503148 |
| Ending overlay (`Game.50`) | 0x4731d0 |
| `Game.1001` to `1006` handlers (the last is a thin wrapper that calls the animator's dispatcher) | 0x46fd50, 0x470750, 0x471370, 0x471b20, 0x471e80, 0x472570 |
