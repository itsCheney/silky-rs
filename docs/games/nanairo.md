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
- `13.0` places a figure, `13.1` removes one, `13.7` removes all, `13.5` sets the expression layer (its first value is not read by the program; a face put on a figure that stands on the screen blends in over 12 frames, 22 of 255 more at each frame, and the commit waits for it, while a figure put in the same commit brings its face with its body; a face picture is a piece of the figure that holds the head with the same outline and its soft rim, and the program copies the rows of the file's rectangle over the same rows of the body across the whole width, it does not blend them, so the rim is not counted twice), `13.9` zooms (at once here), `13.2` crossfades every change over the longest time given and waits until the motions below are over.
- `13.6(side, slot)` sets the order the figures overlap in: the figure's layer takes depth side + 5, so a higher side is in front. A new figure starts with side 2 in the centre, 1 on the left and 0 on the right; the scripts bring the speaker forward with 3. It is kept while the same character stands there.
- `13.8(place, slot)` slides the figure to the place of another slot (0 centre, 1 left, 2 right; the left edge as for a slot) over 12 frames; the script uses `13.8(1, 0)` to make room for a second figure. The slide is linear (not confirmed).
- `13.10(slot)` and `13.11(slot)` move the figure over 12 frames of 16 ms: `13.10` up 78 pixels and back, `13.11` (the one the scripts use, after a figure appears) down 78 pixels and back. Both follow a fixed table of offsets.
- `13.12(pixels, axis, slot)` is a hop (axis 0: up by half, the full amount, half, then at rest) or a sway (axis 1: left, right, at rest), one step each frame.
- `13.13(kind, 1, 0, slot)` starts a glow and `13.13(kind, 1, 1, slot)` lets it go; `13.14` takes it away at once (the scripts send it before each commit). The glow darkens the background and what lies under the figures (black, three quarters), and fades in over 24 frames: kind 0 is `teffect.akb` (green light with sparks) behind the figure, squeezed to the figure's width and moved 80 x size to the right in the centre slot (376 in the right slot); kind 1 is `tai-effect.akb` (white petals) over the whole screen. When movies are on the petals are `HG060002_eff.mov` (white, its green as the opacity) instead of the picture. Letting it go fades it out over 24 frames. The curve of the fades is not confirmed. Compared with the original under Wine, the steady glow (colour and the dark layer) is within three levels at six points across the picture; the figure and the message box are drawn as in the original.
- `13.15(a, 0, slot, slot)` is the same glow for two figures at once: the green glow for 葵, petals for the others.
- Backgrounds and pictures are in sprite 2 and lie under the figures. Sprite 3 is a sheet store that is never shown.

### CG registry

- The scripts name the picture (`14.10`) and its layers in slot order (`14.1`). A name is kept only if it begins with a capital `H`. A layer goes to the first free place up to its slot, or replaces the one at the slot. When the scene is committed, the combination is looked up in the game's CG table of 644 rows: the picture and the first six layers must equal a row's, place by place. A match sets the byte flag of that row and byte 3001. Any registered picture with a name also sets byte 3002 (the CG tab of the extras screens).
- The flags of the table run from 3200 to 3845 and are 3200 + row only up to row 492; row 493 holds 3694. The table is in `cg_table.rs`, the rows in the game's order, so the flag comes from the row and not from its number.
- Names are compared without case and without `.akb`.
- Two tables of pictures lie over the background: five places (`14.1(slot, name)` sets one; `14.2(slot)` frees one) and three (`14.12(slot, name)`; `14.3(slot)`). The picture is drawn only when the script composes: `14.4`, `14.5` and `14.7(n)` copy the sprite 2 picture onto the screen and draw the named pictures of both tables over it, the first table first, pure green in them clear; `14.5` and `14.7` also fade over a time that is not kept here, and all three register the combination. `14.11` only registers. The scripts free every place, load the picture into sprite 2 and copy it onto the screen, then set the expression parts one by one and compose, so the parts change on the picture. Copying the whole sprite 2 picture onto the screen again takes the pictures of the tables away.
- Sprite 6 holds an event still loaded by `Sprite.1`, and the fade helpers copy the screen background into it (`Sprite.3` from sprite 2 into sprite 6) before they copy it onto the screen. That copy replaces the still: no script copies sprite 6 onto the screen again without loading a new still into it or filling it from the background first, so the still leaves the scene when the whole background is copied onto the screen or into sprite 6.
- While a scene is replayed from the scene mode the registry does nothing (see Scene mode); the settings field the original uses for this was not found, so the replay is marked in the registry itself.

### Event pictures and effect pictures

The effects are sprites of the shared effect stage: keyframes of fades, zooms and turns stepped once a frame (1/60 s). Where a call makes the script wait, it waits as long as the original's keys last. Times are scaled by `/Config/DisplayEffect/Speed` (half at 1, almost nothing at 2), as for the other transitions. With `/Config/SceneSkip` at 1 these calls do nothing. Effects are not saved; the clean up calls, the end of a chapter and the title menu take them away.

- `Game.22.1` puts up `kirara.akb`, clear. `22.2` makes it flash out: 16 frames, opacity 255 to 0 while it grows to 1.2 times about (640, 360); the script waits.
- `Game.1001.0` grows the picture on screen to 1.2 times over 16 frames (the script waits), then fades `HG091010_h05_eff.akb` in, holds it 24 frames and starts again, for ever. `1001.1` fades the effect and the enlarged picture out over 24 frames (the script waits) and ends the effect. The fade in's length and the zoom's centre are guesses (see below).
- `Game.1002.0(name, frames)` fades the scene movie in over `frames` frames and the script waits; `1002.1(frames)` fades it out and stops it. The movie (`HG070502.mov`; a descriptor in `movie.arc` naming a `.vsd` stream, MPEG-1 1280 by 720 with MP2 sound) plays in a layer of 1280 by 720 at the screen's corner as the layer fades in, with its sound, and goes on (it starts again at its end) until `1002.1`. With `/Config/Movie/Play` 0, or a movie that is not in the archive, the picture of the same name stands in when the game has one (`HG070502.akb`, `HG030603.akb`), otherwise nothing is drawn and only the time passes. `1002.3` and `1002.4` use the movie's second drawing mode (HG011701.mov): the opacity is the mean of the movie's three colours, and where that is below 255 the colour is a fixed pale peach (bytes b0 c8 ff, blue first). `Movie.1(_, name)` (the moving backgrounds `BG020002.mov`, `HG010101.mov`, `HG040002_h01.mov`, `HG060002.mov`) plays the movie opaque over the screen for ever under the standing figures until `Movie.2` closes it (the picture the scene had put up in layer 6 goes, as the script's own movies-off branch replaces it); with movies off the script takes that branch and shows the `_mv.akb` picture. `Game.1001.0` also plays `HG091010_H05.mov` for ever over the picture in the first mode: the movie's green is the opacity of a layer filled with 0xff0000, and `1001.1` takes it away. The glow's petals (`13.13` kind 1, `13.15`) are, with movies on, the layer filled white with the opacity from `HG060002_eff.mov` instead of `tai-effect.akb`. The opening, the endings and the staff movies (`op.vsd`, `ed01.vsd` to `ed05.vsd`, `HG090725.vsd`) go through `Movie.0`, `Movie.3`, `Movie.2` and the full-screen player.
- `Game.1003.0(name, frames)` fades the effect picture (`HG091013_eff.akb`) in over `frames` frames and the script waits; it stays. `1003.1(frames)` fades it out and releases it.
- `Game.1005.0` fades `HG080403_eff.akb` in over 64 frames, then the script waits one round of its slow pulse (24 frames held, 96 down, 80 up, 200 in all); the pulse goes on for ever. `1005.1` takes it away at once.
- `Game.1006` is the event animator, two figure pictures (A, `HG090201_h01.akb`, and B over it, `HG090202_h01.akb`) and sometimes a third. Each sub changes what they show and starts motion; the script waits only for 4, 5 and 6. Times are in frames (16 ms) and scaled by the effect speed. 0 and 10: A at (0, 100) rises 20 pixels (fading in for 0), then bobs 20 pixels up and down for ever. 1 and 11: A pans and fades out and in, 64 frames each way. 2: B fades in over 8 frames and bobs. 3 and 13: the figure and `HG090203.akb`, whose lines grow to twice their size over 8 frames (fading in for 3) and swing 30 degrees either way, 12 frames a step. 4: A dissolves into the picture the script put on sprite 2, in 12 steps (about 0.2 s, the script waits). 14: A takes that picture at once and bobs. 5 and 6: the animator fades away (5) or wipes away through rule 18 (6) over 16 frames, the script waits, and it is gone. 7, 12 and 17: A bobs 20 pixels down and up. 8 and 18: `HG090204.akb` at 2.5 times its size turns round for ever over 14-frame turns, with the figure fading in over it. 9: stops the motion and shows only the big picture. 19: stops and shows the sprite 2 picture still. 20 and 30: a beam (`HG090205.akb`) wipes in through rule 18 over 128 frames while the figure over it fades from 52 to 169 and back over 128 frames each way; 20 hides A and B first. 100 takes the animator away, as do the other clean up calls (`1001.3`, `1002.2`, `1003.2`, `1005.2`). Other subs do nothing. A call of one sub goes on from what the last one left; it does not take it away.
- `Game.23.0(from, to, _)` moves the picture in sprite 2, which is wider than the screen (2100 pixels, 820 more), sideways one pixel a frame from window position `from` to `to`. A quarter as many steps at speed 1, none at speed 2. The script waits (about 13 s).

### Ending

- `Game.50` is the message overlay of the end card. `50.0` prepares, `50.2` takes it away (and is also called alone as a clean up). `50.1(rule, frames, h, w, y, x)` wipes in the rectangle (x, y, w, h) of `moji.akb` (the handwriting) in its own place through the rule picture `NNN.BIN`, over `frames` frames, and the script waits; the piece stays until `50.2`. The rule picture lies on the piece's top left corner, as the pictures `150.BIN` to `153.BIN` are made for. The four calls write the line piece by piece, and the signature last. With `/Config/SceneSkip` at 1 nothing is done.
- `Game.34(staff picture, strip)` is the staff roll. The music `31.WAV` starts (the scripts do not start it) and the script waits 130.33 s, the length of the music. The ending's `staffNN.akb` fades in and out over 5 s, `staff_p2.akb` follows from 5 s to 10 s, `staff_p3.akb` from 10.33 s to 13.33 s, then the credit strip (`staff_p4.akb` to `staff_p7.akb`, 575 x 3214) scrolls up from below the screen for 106.7 s. At 120 s the logo (`staff_p8.akb`) comes up on black and from 128 s turns into the closing page `staff_p1.akb`. A click, or skip mode, ends the roll at once; the music stops with it. The call leaves 0 on the stack, as the scripts ignore the value. It is not cut short by `/Config/SceneSkip`. The message window is taken away for the roll.

### Quit, auto save and the trial notice

- `Game.3` is the quit box. It asks "Do you want to end the game?" (`ExitDialog.akb`: a dim layer over the screen, a band with the question, yes and no buttons and a check box "do not show next time"). Yes (area 0) ends the game; no (area 1) goes back to the script. With the check box ticked the setting is kept and the next `Game.3` ends the game without asking. The same box, with the other questions of the sheet, is the one the window bar and the other quit and confirm buttons use. The result of the box (0 yes, 1 no) was read from the program.
- `Game.17` is the auto save. No script calls it. It saves unless `/Config/AutoSave/Enable` was written as 0 (a new install has it on). Here the save is written at the next message by the same path as the other games' auto save points, and only while the player's auto save setting is on. The original writes the save at once with the screen as the picture. Its auto saves are a list of twenty (slots 100 to 119): each new one is written to slot 100 after the others have moved one place down, so the oldest falls off the end. The quick saves are a second list of ten (slots 120 to 129) written the same way. Auto saves are also written when a selection comes up and when the calendar starts, each under the same `/Config/AutoSave/Enable` test. The load screen has a page for every ten slots, so it lists the auto saves and quick saves on the pages after the ordinary ones; the save screen offers only the ordinary pages. Here the save screen's auto tab holds the twenty (`AUTO_SLOTS` in `games/nanairo/mod.rs`, the oldest is replaced), a selection writes one as in the other games, and the calendar asks for one the way `Game.17` does (written at the next message). What the original's record holds beyond the game state (a slot table of 54 bytes a slot and the last slot used) is not kept.
- `Game.40(kind)` is the notice of the trial version: it loads `trial.area` or `trial2.area` and opens a web page when the first area is clicked. Those files are not in the archives of this game, so nothing is shown. The call is accepted and does nothing.

### Calendar

- `Game.39(days)` is the calendar of a date change. `days` is how many days the date column scrolls (word variable 100 holds the new day's row in the 31-row date table, August 18 to September 26). White curtains open over the new background, which drifts in behind a fine white veil (`InterludeBG.akb`); the month name (`Interlude.akb`) fades in, the column of dates fades in, a grey bar slides in, the disc lands (12 frames, 1.2 to 1.0 size), and the column scrolls down `days` cells in 32 frames so that the new day ends under the disc. The part of the column inside the disc's window is drawn in yellow while it scrolls. After a hold the curtains leave and everything fades out, about 280 frames in all (16 ms each, scaled by the effect speed). The script waits; a click ends it at once. With `/Config/SceneSkip` at 1 nothing is done. The white fade-out the script puts before it is lifted when the calendar starts, so the new background is up when it ends. Moves and fades are taken to be even and the layers that come in later start hidden. Compared with the original on four calendars of the first route, among them one that scrolls three days: the days shown, the month, the column, the bar and the disc are in the same places, and the column scrolls the same distance in an even move. The original under Wine ran too slowly to compare timing.

### Music mode

- `Game.32` is the game's own music mode, in `games/nanairo/music.rs`; the frame and the completion counter it shares with the CG and scene modes are in `mode.rs`. The script draws `SystemBG2.akb` first; the screen draws the frame and its own parts from `MUSICmode.akb` over it and answers when it is left.
- 35 rows in three columns (13, 13 and 9), each 388 x 27 at x = 41, 476 and 880 and y = 128 + 37 x row. A row is open when its flag byte is 1 (3050 to 3084, set by the scripts); a closed row shows `???`. The rows are drawn in the order of the table of tracks in `music.rs` (flag and file: row 0 is flag 3074 with `25.wav`, row 2 is 3050 with `01.wav`, and so on to `35.wav`); the picture of row k is the strip of title k + 1, in four states (normal, hover, playing or pressed).
- A press on an open row that is not playing plays the confirm sound `ses_001.wav` and its file once at the music volume. The row stays yellow while it plays, a speaker mark is drawn beside it, its title strip is drawn at (307, 630) and a stop button at (233, 615). The stop button, or a right click, stops the track (the right click only stops it when one plays; with none it leaves with `ses_002.wav`). When the track ends the screen stops it; no next track starts.
- The volume bar is at (761, 630); a press or drag sets the volume to the pointer's x - 775, 0 to 267, at once. With the music switched off the bar is drawn empty with no knob and does not move.
- The tabs (CG, music, scene) are open when their bytes (0xbba, 0xbbb, 0xbbc) are not 0; a closed tab is grey and does nothing, the music tab does nothing. A press plays the confirm sound and the screen slides out; the mode loop then goes to the other mode. The back button leaves with -1, the quit button opens the quit dialog (`Game.3`).
- The frame slides in over 14 frames of 16 ms (tabs from the left, banner, buttons from the right, bar from the left) and out over 12 frames (read from the program: one move of 14 or 12 steps for each of the four pictures, one step a frame, even; the 0.4 s seen under Wine is that emulation running slow); the page fades in over 10 frames and out over 8. The mode loop goes by the results 0x100 (CG), 0x101 (music) and 0x102 (scene) in `mode::next`: the three modes open each other's own screens.
- The BGM: when the screen opens, what plays is faded out (the original fades to near silence over 32 frames and leaves it playing; here it fades out and stops), or stopped when the music is switched off. The fade is 32 steps down to -50 dB and then the channel stops. The mode loop looks at the BGM channel at the top of every pass and, when the music is on and nothing plays, starts `25.wav` (`music::BGM`): so a mode opened over silence, a tab switched to, a picked track that has ended and a stop button all end with `25.wav` playing (here the same, looped). The music mode's own start comes first, so a BGM it fades out still counts as playing and is not replaced. On leaving, nothing stops the music: a track picked keeps playing (a right click stops it first), as in the original.
- Keyboard (the same on the CG, scene and music screens, see `games/nanairo/keys.rs`): the four arrow keys move the mouse pointer to the middle of another button and the hover follows, so there is no cursor of its own; Enter (and keypad Enter) clicks the button under the pointer; Escape, Delete and keypad 0 do what a right click does; Space, Tab, Home, End, the page keys and the wheel do nothing. A key held down moves once. With the pointer on no button an arrow goes to the first row. Up and down walk a column (up from the top row goes to the first tab, from the first row of the other columns to the scene tab), down from the last row goes to the stop button, the volume bar or back; left and right change column and tab; the volume bar answers with the knob's place.
- Not done: the extra -1 the mode loop pushes on the result vector before switching from the music mode; the exact curves of the slide and the fades (even ramps are used); the volume curve (the bar position is used as the volume 0 to 1). The volume bar runs 0 to 267 and a new install starts the music volume at 200 (knob about three quarters along; the effect volume of a new install is 267, the full bar, and the shipped config says the same), so the knob rests at 975 on a fresh game.

### CG mode

- `Game.30` is the game's own CG mode, in `games/nanairo/cgmode.rs`; it uses the frame, the slide and the completion counter of `mode.rs`, and the sheet `CGmode.akb`. The script draws `SystemBG2.akb` first, and the mode loop goes by the results 0x100, 0x101 and 0x102 as for the music mode.
- 86 pictures on 6 pages of 15 thumbnails (5 columns by 3 rows, 250 by 156 apart from (26, 138)). A picture has one to 31 variants; the variants are consecutive rows of the CG table (`cg_table.rs`, 644 rows), and the item table in `cgmode.rs` gives the count and the thumbnail sheet of each. A picture is open when the flag byte of its first variant is not 0; a closed slot shows the plain frame and does nothing. An open slot shows its thumbnail in a frame that lights up under the pointer and darkens when pressed.
- The counters: the percent of the table's 644 rows whose flag is set (the same counter as the music and scene modes) in big digits (cut down, 100 only when all are set, never under 1, so a fresh game reads 1 %; a row counts when its flag is not 0), a small counter of the rows seen, and the page number over the slash, with two page arrows that are drawn grey at the first and last page. The arrows turn the page with the confirm sound `ses_001.wav`; the page fades in over 10 frames and out over 8.
- A press on an open slot plays `ses_001.wav` and opens the variant picker: the page is dimmed by a black layer (alpha 0xc0) that fades in over 10 frames, and the variants are laid out as small frames, 5 per row, centred on the screen (a block as wide as the count, up to 5 columns, and as high as the rows). More than 15 variants are paged by 15 with their own arrows. A variant that is closed is shown as a plain frame and does nothing. A press outside the block closes the picker with the confirm sound; a right click closes it silently.
- A press on an open variant leaves the mode with the item number as its result and the script variables 8 and 16 set to 0 and 17 to the variant number; the script `cgmode.mes` then shows the picture, steps through the variants and ends the viewing. In the viewer a left click or the key poll (259) goes to the next variant and the right click (poll 257) closes it; the mode loop then reopens the screen on the same item and page (script variables 9 and 25 say so) with the picker open.
- On the grid, a right click or the back button leaves with -1 (the right click with `ses_002.wav`), the quit button opens the quit dialog (`Game.3`), and the music and scene tabs go to the other modes with the confirm sound after the slide out. The tabs are open when their bytes are not 0.
- Read from the executable: the layout of the grid, the picker and the counters, the tables, the results of the presses and the sounds. Guessed: the base of the script variables the screen writes (taken as the consecutive variables 8, 16 and 17, which the script reads), the order of the picker over the frame, and the curves of the fades (even ramps).
- Keyboard: as on the music screen (`games/nanairo/keys.rs`). The grid moves by cell inside the page; down from the bottom row goes to the next page arrow (the previous one on the last page); the page arrows and tabs are reached the same way and a page is turned by Enter on its arrow (the page keys do nothing). The picker moves by cell, its arrows and the area outside it. The picker's first cell of the second row does not go up, as in the original. Not done: the script variables the original resets when a tab is pressed (their bank is not known); the scene tab still opens the shared scene screen.

### Scene mode

- `Game.31` is the game's own scene mode, in `games/nanairo/scene.rs`, on the frame of `mode.rs`. The script draws `SystemBG2.akb`; the screen draws the frame and its page from `SCENEmode.akb` (cards, arrows, digits, counter), `SCENEmode2.akb` (the titles, 222 x 25 each) and `scene_thumb.akb` (the thumbnails, 222 x 125 each, five to a row) over it.
- 78 scenes on six pages of 15 cards (the last page has three), five columns by three rows. Scene i = page x 15 + card. Scene i is open when byte 3100 + i is 1. A card is drawn from its frame (232 x 149 at 24 + 250 x column, 129 + 156 x row, with the filler pieces between cards); an open card also shows its thumbnail at (+5, +12) and its title bar at (+5, +108). A closed card shows only the frame and does nothing. The areas are 225 x 127 at (28 + 250 x column, 140 + 156 x row).
- Page arrows (32 x 32 at (551, 609) and (697, 609)) turn the page with the confirm sound `ses_001.wav`; the one that has no page to go to is drawn grey and does nothing. Between them the page number, a slash and the page count (6) are drawn from the sheet. The completion counter is the share of open scenes in per cent, cut down (0 with none open).
- Hover: when the pointer is on an open card, the card's film (`scene_thumbNNN.akb`, 1 to 3 pictures of 222 x 125 side by side; the count for each scene is the table in `scene.rs`) is drawn over the thumbnail, the title bar again, and a yellow frame (224 x 127). A film of several pictures plays: the first change starts 500 ms after the pointer came onto the card, each picture holds for a second and then fades into the next over a second, going round to the first after the last.
- Pressing an open card plays the confirm sound, the frame slides out, and the screen answers the scene number (0 to 77). The mode loop gives the number to the script, which runs `nanairo_scene%03d.mes` with the message options forced off around it (`scmode.mes`). The scene's page is kept in script variable 17 so the screen opens on the same page when the script calls it again, which it does at the end of the scene: the replay ends where it started.
- A scene replayed from this screen does not register CG: the registry (`cg.rs`) is switched off from the pick until the next mode screen opens or the title is reached. The original does this with a field of its settings block, which the CG registry reads; what sets it was not found, so this is the reading that fits (the field is read by the registry and by other message-window code, and nothing else was seen to run only in a replay).
- Tabs, back, quit and right click are as in the music mode: a tab that is open answers 0x100 (CG) or 0x101 (music) after the slide out, the scene tab does nothing, back answers -1, quit opens the quit dialog, a right click leaves with `ses_002.wav`. On a tab press the script variables 16 to 18 are set to 0 and variable 19 (the script's screen number: 0 CG, 1 scene, 2 music) follows the screen that opens; which of the original's variables these correspond to is a reading, not confirmed.
- Keyboard: as on the CG screen; only open cards take the pointer. Not done: the exact curves of the slide and fades (even ramps); whether the original also slides the frame out when a card is picked (it is done here); the meaning of the BGM fade the scenes' skip setting starts.

### Other commands carried out

`Game.19` fills byte flags, `Game.26` ends the message window before the next script, `Game.27` and `Game.10` start up, `Game.22.0` registers the picture shown. See [game-commands.md](../game-commands.md#なないろリンカネーション).

## What does not work yet

- Calendar (`Game.39`): timing is not compared with the original; the disc's scale centre (62, 62), even moves and fades, and the one frame between its states are assumed.
- Movies play from the stage's clock, not the sound's. That a layer movie starts again at its end is read from the player's end-of-stream path and from the 1 to 2 second clips used as backgrounds, not tested against the original. The red of `1001.0` is read as 0xRRGGBB (the engine's colours elsewhere are).
- Not read far enough to guess the numbers: the fade in of `1001.0` (24 frames used), its zoom centre (the screen's middle used), the low point of the `1005` pulse (half used) and which way `Game.23.0` moves (from the first value to the second). In the animator: whether the 0.25 size of subs 0 and 10 is a quarter size on screen (it is drawn so, on A), which sides the pans of 1 and 11 move (100 pixels sideways and 5 down are used), what sub 30 does beside leaving A and B, whether the beam's wipe repeats (once), the pivot of the lines of 3 and 13 (the one point of their growth) and the units of the 128 frame fade of 20. In the staff roll the curves inside the stages are not known (pages fade in and out, the strip moves evenly, the logo fades into the closing page), nor the strip's real pace, and the sequence of the last pages is the plainest reading. The wipe of `Game.50.1` uses an even ramp. Effects in progress are not saved.
- `Game.1004` (no script calls it) is carried out as removing every standing character at once, with nothing drawn; the original's exact effect is not read. `1007` to `1009` draw nothing and are accepted: they only tidy caches and the save folder's thumbnail files at start up.
- All three modes (`Game.30` to `32`) are the game's own screens.
- Standing characters: a face that is put on a figure standing without one fades in over 12 frames in the original; here it appears with the crossfade of the commit. The curves of the slide and of the glow's fades are guessed linear; the exact frame rate of the motions (16 ms steps) and the glow's place for the left slot are not confirmed.

## Game commands

| Command | Does |
| --- | --- |
| `10` | Start up; nothing to do |
| `12.4(face, body)`, `12.5`, `12.6`, `12.7`, `12.8(picture)` | Face window |
| `13.0` to `13.2`, `13.5` to `13.15` | Standing characters |
| `14.10(name)`, `14.1(slot, name)`, `14.12(slot, name)`, `14.2`, `14.3`, `14.4`, `14.5`, `14.7(n)`, `14.11` | CG registry; pictures over the background |
| `15(name)` | Speaker name |
| `18` | Title menu |
| `19(value, end, start)` | Fill byte flags |
| `22.0` | Register the picture shown |
| `26` | End of chapter |
| `27` | Message window start up |
| `22.1`, `22.2` | Sparkle picture: put up, flash out |
| `23.0(from, to, _)` | Sideways scroll of the picture in sprite 2 |
| `1001.0`, `1001.1` | Event picture zoom with an effect picture on top; end |
| `1002.0(name, frames)`, `1002.1(frames)`, `1002.3`, `1002.4` | Scene movie fade in and out (the movie plays; a still stands in with movies off) |
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
| Standing characters: expression fade-in step (`13.5`) | 0x445080, 0x441ca0 |
| Standing characters: the face's rows copied over the body's (row range of the file's rectangle, width 1600) | 0x441ca0 |
| Glow pictures; movie name | 0x50673c, 0x506748; 0x506758 |
| Face window: handler, crop and draw | 0x46eca0, 0x4590b0 |
| Speaker name (`Game.15`) | 0x46f830 |
| CG registry: handler | 0x46f3a0 |
| CG table (644 rows of 36 bytes: flag, then up to eight picture names; ends with -1) | 0x533cf0 |
| CG registration: match and set the flags; start (`14.10`); layer (`14.1`) | 0x43d650; 0x43d840; 0x43d9f0 |
| Picture tables (five, three places): set; compose onto the screen | 0x45fc60, 0x45fce0; 0x45fe50 |
| `Game.3` quit: handler; yes or no box constructor; click; button draw | 0x494c50; 0x449650; 0x449a40; 0x449c90 |
| Box setting read (`/Config/Dialog/%02u/Enable`, default on); table of question to setting | 0x474c40 (path at 0x509490); 0x50d440, 0x50d3c0 |
| `Game.17` auto save: handler; setting read (`/Config/AutoSave/Enable`, path at 0x5094c0); slot writer (slots 100 to 119, calls the worker with slot 0x64) | 0x46f970; 0x4750d0; 0x463c20 (worker 0x4644b0, file copy 0x464ac0) |
| Other auto save points: calendar (vtable slot 0x14 of `CCalender`); selection start (`CNormalSelect` slot 0x18) | 0x43ac30; 0x45c5d0 |
| Quick save list writer (slots 120 to 129); save and load dialog pages (13 pages of 10, 54 byte slot table) | 0x463b00 region (worker call with 0x78); 0x4607c0 |
| Mode loop: key map (Enter 0x1c and 0x9c; Escape, keypad 0 and Delete; arrows 0xc8, 0xd0, 0xcb, 0xcd; the rest do nothing) and BGM check at the top of a pass | 0x486e10 |
| Mode vtables: CG; scene; picker (`CCGView`); music; arrow slots 0x50, 0x54, 0x58, 0x5c, click 0x74, cancel 0x7c | 0x506428; 0x5070d4; 0x506660; 0x507960 |
| Arrow handlers' pointer move to the middle of an area | 0x473da0 |
| Music mode start (fade 32 steps to -50 dB, or stop with the music off); mode BGM name (`25.wav`) | 0x45a840; 0x5077b8 |
| Music channel: stop, fade; fade worker and step | `CFuncMusic` vtable 0x507a5c (slots 0x5c, 0x64: 0x48cb50, 0x48cb80); 0x433e10, 0x4339e0 |
| `Game.40` trial notice: handler; object run | 0x472890; 0x46dc60 |
| `Game.22`: handler; workers of subs 1 and 2 | 0x46fa10; 0x46e380, 0x46e4f0 |
| `Game.23` scroll: handler | 0x4734d0 |
| `/Config/SceneSkip` read; `/Config/DisplayEffect/Speed` read | 0x474ac0; 0x509390 |
| Movie player: open, stop | 0x42a1e0, 0x42b430 |
| Movie layer: rectangle 0, 0, 1280, 720, drawing mode (`engine+0x70`) and key colour (`engine+0x74`); 1002.0, 1002.3, 1001.0 | 0x4708e0, 0x471060, 0x470511 |
| Movie frame copy: mode 0 opaque, mode 1 green as opacity, mode 2 mean as opacity with key colour | 0x42cce0, 0x42cd70, 0x42ce00 |
| Glow petals with movies on: white fill, movie opacity; movie play config | 0x442b9b, 0x509368 |
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
| Mode loop of the three modes (`Game.30` to `32`) | 0x472ad0 |
| Music mode (`Game.32`): start, per frame, press, volume, stop, draw | 0x45a020, 0x45a310, 0x45a910, 0x45aab0, 0x45ad70, 0x45b1e0, 0x45b400, 0x45b750, 0x45b860, 0x45bac0, 0x45bc70 |
| Frame of the three modes (tabs, buttons, banner, bar): layers, slide in and out | 0x45f120 to 0x45fa10 |
| Music mode: flag table (3050 to 3084 by row); wav table | 0x5078d0; 0x5077b8 |
| CG and scene modes: completion counter | 0x43cde0 |
| Settings defaults read at start (music volume 200, effect 267, master 267 of 267); the music screen's knob range 0 to 267 | 0x475a70; 0x45b860 |
| Frame of the three modes: slide in (14 steps) and slide out (12 steps) timelines | 0x45f400; 0x45f780 |
| Scene mode (`Game.31`): constructor, init, click, right click, hover, page draw, filmstrip hover | 0x44f1d0, 0x44f370, 0x44f5d0, 0x44f760, 0x44fd30, 0x44fec0, 0x450490, 0x450940 |
| Scene mode: flag table (3100 + scene); frame count table | 0x53f7e0; 0x5026d8 |
| CG mode (`Game.30`): constructor, init, press, right click, hover, page draw, picker draw | 0x43b5e0, 0x43b6d0, 0x43ba70, 0x43bf00, 0x43c250, 0x43c470 |
| CG mode: viewer (constructor, step, close, draw) | 0x43e8d0, 0x43ebc0, 0x43ede0, 0x43f570 |
| CG mode: page table; row table | 0x52dcc0; 0x52e230 |
| CG mode: thumbnail sheet parts (`IOmakeParts`) | 0x45f170 to 0x45f780 |
| `Game.1001` to `1006` handlers (the last is a thin wrapper that calls the animator's dispatcher) | 0x46fd50, 0x470750, 0x471370, 0x471b20, 0x471e80, 0x472570 |
