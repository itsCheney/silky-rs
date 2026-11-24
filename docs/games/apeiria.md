# 景の海のアペイリア

English | [简体中文](apeiria.zh-CN.md)

Japanese build (SILKY'S PLUS WASABI, 2017) with the official update. The runtime knows the game by `apeiria.exe` in its folder; the build described here has SHA-256 `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e`. Program version 1.0.3.0, linked 2017-11-09. Engine profile: [engine-2017](../builds/engine-2017.md).

All 171 scripts decode; the story text is in the scripts (no MAP tables). Headless, `silky-run snap` runs the whole story to its end with every engine and `Game` call carried out.

## What works

### Text, window and menus

- Ruby: `text 1` opens a ruby; its string is drawn at half size above the text that follows.
- Selections are `Select.2`, which returns the option picked, from 1.
- Title menu: six entries from `title.area`, the extras page (CG, scene, music) from `title2.area`, cells of `title_p.akb` in four 42-pixel state rows. Continue and load are grey without saves; CG, scene and music need bytes 3002, 3003, 3004.
- Message window: a band at y = 508 (rows 212..424 of `message_p.akb`, an alpha mask filled with the window colour) under faint lines (rows 0..212). Defaults: red 0, green 25, blue 46 of 100, 30 % transparent.
- Window buttons: a ring around the `MENU` ball (the ball folds the ring away). Ball and ring backdrop are 302-pixel cells of `menu_p00.akb`; the eleven buttons and the hide button are 40-pixel cells of `message_p.akb`. `mes_o.area` ids 1..11: save, load, quick save, quick load, voice, previous choice, backlog, auto, skip, next choice, settings. The hide button and window are relative to the window top.
- Click wait: hourglass of `message_p.akb` (ten 76 x 80 frames down from (1280, 0)) at (1204, 642), a frame every tenth tick, 200 ms.
- Window parts are placed relative to `/Config/MesWnd/Pos` (hourglass at Pos + 132), which the program only reads; the config screen has no control for it.
- Name plate: left cap, 14-pixel middle pieces for the name's width, right end with flourish, from row 584 of `message_p.akb`, each over the same piece of row 643 filled with the window colour. Plate at (47, 498); the name 69 in and 17 down.
- Text: glyphs about 24 pixels tall every 28.3 pixels from x = 152, lines 41 apart, bold with a one-pixel black edge.
- Window portrait (`Game.10` subs 6, 8, 11 to 13): the face of the body picture, cut by a window per character, drawn at 45 % at (-144, 440), cheeks over it; the name plate moves to x = 210 while it is up.
- Standing characters: the slot gives the centre (slot 1 at -400, 2 and 3 at +400 from the middle); top edge = (1 - size) · a - size · k, with a and k by character (`chNN`). `Game.13.6` takes cheeks, 13.8 moves at once, 13.13 changes body in place.
- `Game.961`: a voice the next message does not cut. `Game.14`: auto save.

### Voice-timed picture changes (`Game.1007`, `.vo` files)

A set time after the voice starts, the window portrait (0), a standing character (1), the event picture and its difference picture (2) or Apeiria on the hologram (3) change to the face the file names, with cheeks if its flag is set. If the next message comes first, the change is made at once. The original fades the new picture in; here it switches at once. `.vo` layout (older than ふゆから、くるる。's): head of three u32 (count, what changes, milliseconds), then 792-byte records (slot, size, flag, three 260-byte names).

### Movies

With movies on (the starting setting), scripts ask for clothes-change movies (`Game.999`, `chNN_chg.vsd`) and move the screen copy a shake uses (`Sprite.13`). The movies are not played, so scenes look as with movies off, but `Movie.3` answers 1 after one, as the original does.

### Screen effects

- Shake (`Game.993`): sub 0 sets up, 2 picks one of two tables of eight offsets (sideways only, or both ways, up to 16 pixels), 3 starts, 1 stops. One offset per step (a sixtieth of a second).
- Zoom flash (`Game.1005`): zoom to 120 %, turn white, fade back over 16 frames (as in ふゆから、くるる。). With the scene-skip setting on the original does nothing here; the setting is not offered, so it reads as off.
- Attacks and spells play as the executable's keyframe animations (16 ms steps); the script waits while an effect blocks.
  - `Game.998` attacks: kinds 0 to 17 (sub 0) build sprites from the attack sheets with moves, fades, turns, scales and wipes. Bullet streaks, fireball, sword rain, slashes and white flashes use the executable's per-frame code, ported with its random numbers. Sub 1 keeps an attack up until sub 4 takes it down. Kind 14 shows the ev1701 pictures in turn, each drawn into the background as its fade ends, so they stay.
  - `Game.1000`: specks at the top of the screen (subs 0 to 2).
  - `Game.1001`: screen effects (subs 0 to 21, 255), including the random slashes of sub 6.
  - `Game.1002` spells: mosaic in/out (0, 1; block size to another over a number of frames), cross fade with a rule (2, 3), dark red screen (4), flash (5), cut between two event pictures (6; the second goes into the background), glitch bars (9), rule reveal (10).
- What effects draw into the background (`Game.998` kind 14; spells 4 and 6) is kept with the scene like the conclusions list: under the characters, saved with the scene, gone when the script sets a new background. A screen left covered by spell 0 or an increasing spell 1 is a sprite over everything and likewise stays until the background changes.

### αMAIL (`Game.1012`)

Phone frame (`SLD.akb`); mail list of `mail01_p.akb` with tray label (sub 0, list A or B); newest mail row (byte 1200 or 1400, set by sub 11); "new mail" notice with count (subs 5, 6); selected row (8, 9); open mail on `mail02_p.akb` with body text (3, 4), scrolled by pages of 381 pixels (10).

- Sub 7 puts the Apeiria system window in the panel instead of the list, at (309, 36). Sub 14 puts a mail in the first free byte of 1400 to 1499.
- The white list shows 447 pixels below its top; its last 42 rows fade linearly into the glass.
- Open (sub 1): wipe in from the middle through rule 46 over 64 frames while 50 specks of `light.akb` rise into it. Close (sub 2): shrink to a tenth of its height about its middle over 16 frames. The script waits for both.

### Apeiria on the hologram (`Game.1013`)

Top or bottom half of `sld_mini.akb` under the characters; over it a window of her picture at half size with expressions drawn over it; at 230 / 255 opacity on the top half.

- Sub 2 way 0 on the top half: wipe in from the middle through rule 47 (`47.bin`, laid over the panel) over 12 frames while 50 specks of `light.akb` rise from the bottom of the screen into the panel (`CMiniLightAnime`).
- Way 0 on the bottom half, and way 2 on either: fade in from nothing over 12 frames.
- The script waits for both (the 12 frames, about 0.2 s at the engine's 15 ms tick), and while sub 3 fades her and the panel out over the given frames. Shown again while up, she comes again the same way; shown while still coming, the script waits.

### Pictures over the scene

- Copies of one picture at fixed spots (`Game.995`, the virus swarms; tables for 1, 2, 4, 8, 16, 32 copies, scaled about their centres). Subs 1 and 2 (time, wave, to, from) fade them in from nothing to their opacity, or out, over `time` frames while the script waits. Wave 1 wavers the first copy row by row (from 256 pixels and 20 degrees a row to nothing, or back); scripts never use it.
- Deduction board, 1016 x 424 at (132, 15) (`Game.994`). Sub 0 sets it up empty and sub 1 drops it, both at once. The others take 24 frames (12 for sub 5) and the script waits for each; a click does not cut them short.
  - Sub 2 (file) opens it: the picture grows from a line in the middle to its full height. Sub 3 closes it the other way.
  - Sub 4 (file) puts a new picture over the old one: in Apeiria it fades in, in Casablanca it wipes in through rule 1 (`01.bin`, over the whole screen).
  - Sub 5 (y, x, file) shows the new picture at once and a marker, `d00_00.akb`, zooms from double size to its own over 12 frames at (x, y) of the screen, then goes.
  - With `/Config/SceneSkip` at 1 the engine skips subs 0, 2, 3, 4 and 5 (no board change, no animation, no wait) and the runtime does the same; sub 1 always runs; the setting is not offered here, so it reads as off.
- Notes panel `k01_0N.akb` at (145, 0) and explanation screen over `kbg.akb` (`Game.996`), the scene with the frame and picture drawn in:
  - Subs 13, 14: a new one fades in over the old over `time` frames while the script waits (sub 13's rule is unused).
  - Sub 15: the new one blinks over the old (after 42 frames fades out over 8, after 128 more back in over 8).
  - Sub 19: fades in again and again over `time` frames until sub 16 stops it where it is.
  - Sub 17: fades the screen out over 12 frames, script waits. Sub 18: draws it into the background or picture layer, staying until the background changes.
- Item panel at the left, bottom at y = 495 (`Game.997`): opens like a screen switched on, a line 1 % high growing to full width over 8 frames, then to full height over 4; closes by height to nothing over 12. The script waits for both.

### Explanation slides (`Game.991`)

Lecture slides: a background (`tbg.akb`, kind 0, or `wbg.akb`, kind 1) with a 1280 x 3880 layer over it, which the script moves up and down so the screen shows a 720-line window. The pictures (`tNN_NN_NN.akb`, `wNN_NN_NN.akb`) are 1280 x 720 or taller, mostly transparent, and drawn over the screen's scene (the backgrounds are translucent at the edges). The panel is above the characters and under the message window.

- Sub 0 (kind) makes it, sub 1 drops it (both at once; sub 0 does nothing when it is there). Sub 2 (file) copies a picture into the layer's top left corner at once and shows the panel. A copy replaces what was there, so a slide drops the bubbles of the one before; a shorter picture leaves the lower lines of a taller one.
- Sub 4 (frames, file): the picture fades in over `frames` frames, the old one fading out under it, and the script waits. Sub 3 (frames, rule, file) fades it in over the layer on the screen's corner (the rule is not used) and then copies it in; the script waits. Scripts use 12 or 24 frames.
- Sub 10 (frames, y, x) moves the layer in a straight line from where it is, and the script waits. Sub 12 does the same while the script goes on (a slow scroll through several messages). Sub 11 (y, x) puts it there at once. Sub 6 stops a scroll where it is.
- Sub 7 stops it and fades the layer out over 12 frames while the script waits; the background stays until sub 1. Sub 8 stops it and leaves the background and the 720 lines that show in the scene's background, which stay until the background changes; the lecture scripts do this before they drop the panel, so the new background wipes in from the slide.
- Subs 5 and 9 are only called by the library script when it rebuilds the panel after a load, from the values it saved under Save key 65 (0: kind and picture, 3: position; scripts never write 1 and 2). Sub 5 (file) puts a second picture over the screen that blinks (shown, after 42 frames out over 8, after 128 more back in over 8) and sub 9 (frames, _, file) one that fades in over `frames` frames again and again, until sub 6. They draw at the screen's corner, not in the layer.
- The panel is kept with the scene in saves, so a load brings it back at once; the library script's rebuild then does the same again with no effect.
- With `/Config/SceneSkip` at 1 the engine skips every sub but 1 (no picture, no scroll, no wait) and the runtime does the same; the setting is not offered here, so it reads as off. Casablanca has the code but no slides and never calls more than sub 1, which does nothing there.
- Not known: the exact z-order against the characters (the layer is added on top, the background picture and the second picture have priorities 0 and 2); how a sub 12 scroll that is still running meets sub 11 or the next scroll (here sub 11 ends it, a new scroll starts from where the layer is); whether a pixel of a fading picture is blended linearly (here the old picture fades out as the new one fades in).

### DNA PC (`Game.1014`)

Its screen goes into the background, the monitor over it; the system window at (305, 37) is pieced from `apeirasystem.akb`.

- Sub 1 opens the window (height grows from nothing about its middle over 16 frames); sub 2 closes it the other way; the script waits. Sub 3 hides the strand with the window.
- Sub 4 brings in a DNA strand: `RedLine.akb` and `BlueLine.akb`, 16 x 430 each at (330, 15) in the window, rows shifted sideways along a sine wave 85 pixels wide, half a degree a row, the wave advancing 2 degrees a frame with blue half a turn behind. Blue over red in the top 200 rows, red over blue in the next 200.
- It fades in over 16 frames; sub 5 fades it out over 16; the script waits. Meanwhile the window shows only its background, without emblem and bar.

### HP gauges (`Game.1015`)

- Five gauges cut from `hp.akb`: green from half, orange from a tenth, red below, a skull at 0. Steps queued with sub 4 run with sub 9.
- Slide in to x = 8 (sub 1) or out to x = -424 (sub 2) over 16 frames; sub 3 moves HP over 128 frames; the script goes on meanwhile.
- A gauge whose HP drops shakes for 16 frames, going right, down, left, up by 2 pixels (drop up to 20), 8 (up to 40) or 16.
- Sub 6 (id, interval, count, delta, gauge) moves a gauge's HP by delta every interval ms, count times, kept between 1 and 100, ending at 1; sub 7 (id, gauge) stops it.
- The original also writes HP to word variables 100 to 104; scripts do not read them.

### Conclusions list and mail screens

- Conclusions list (`Game.992`): 250 x 51 cells of `dnapcA_mailparts.akb` (column v / 100, row v % 100) for bytes 1300 to 1306, as many as byte 1320 says, last first, drawn into the background from (356, 103) down; a value under 100 gets 100 added once drawn. Sub 1 (text, page, cell) draws a mail into the background the same way: header cell of the same sheet at (655, 73), black text wrapped at 276 pixels in a 300 x 272 view at (641, 161), scrolled by page.
- Mail forward screen (`Game.1016` sub 0, with the mode): a modal selection over the DNA PC background.
  - Lists the mails whose byte is set (1001, 1002, 1003, 1100 twice, 1101 twice, 1102 four times; mode 1 only the first three), packed from (353, 95) down in 32-pixel rows of `dnapc_mailparts.akb`, each with a check box at (860, 100); forward button (メール転送) at (929, 389). Hit areas in `mailselect.area` (ids 0 to 10 for rows, 20 for the button).
  - The checked row starts as byte 0 (255 for none). Clicking a row checks it and wakes the button when its number is byte 1 or 2 (or in mode 1); otherwise the screen ends at once with that number. The button ends it with the checked row's number; clicking the checked row unchecks it.
  - Opening fades the screen in over 12 frames, lifting the script's cover; input waits for the fade. Story scripts use mode 1; sub 1 redraws the rows. Frontends answer it as a selection over the listed rows. No close button.
  - Looks: a row under the pointer comes from x 501 of the sheet; the check box shows the check (y 19). The active button under the pointer is y 60 (y 0 normal, y 180 off).
  - Right click, Escape and Delete do what the settings say for a right click: hiding the message window (the rows stay; a click brings it back), skip or auto mode, the settings or loading.

### Extras (`Game.30` to `Game.32`)

Seen state is kept across games in byte variables from 3000. A CG bit (one per picture, from byte 3300) is set when the background and layers put up by `Game.12` (subs 0, 1, 10, 12) match a row of the executable's table once the screen is shown (subs 2, 3, 4, 6, 11), or directly (sub 13). Scenes from byte 3101 and music from byte 3050 are set by scripts; bytes 3002 to 3004 open the three modes.

- CG mode: eight pages of twelve entries (`cgmode_thmNN.akb`), seen count on each, completion of all three modes at the bottom. An entry opens a grid of its pictures (from its `cg_evNNNN.akb` strip, twelve a page) over the dimmed page; a picture shows full screen, clicks walk through every seen picture to the last, then back to the grid. The wheel walks back and forth.
- Scene mode: six pages of twelve scenes; under the pointer a scene shows a slide show of `scene_thumbNN.akb` (a second each, a second's fade). Click plays the scene.
- While a scene plays (variable H 69 is 1), a right click, Escape or Delete on its text asks 「シーン回想選択に戻りますか？」 instead of the right-click setting. Yes ends the text with result 1, which the scene script takes as the sign to return to the scene list.
- Yes/no dialogs: the plain box of `dialog_p.akb` at (391, 254) over black at 127 / 255, the question's strip at (460, 301), the yes and no buttons and the "don't show again" box at their `dialog.area` corners.
- Music mode: 41 tracks in eight slanted rows of `music_title.akb`, scroll bar, music volume. CG and scene modes play `ape_27.ogg` when nothing plays; music mode stops the music it finds. Scroll bar arrows scroll one row on press, not again while held.
- Nothing is remembered between openings: each time a mode opens (also when switching between modes) it starts on its first page or the top of the list.
- Modes fade in from black and out to black: 12 steps of the 20 ms tick (240 ms); input waits for the fade. The title menu fades out with the screen.
- Arrow keys move the pointer to the middle of another button by each screen's own table (a key from no button goes to the first cell); Enter clicks. On a full-screen picture, left and right walk back and forth.

## Where things are in the executable

Addresses in `apeiria.exe` of the build above. The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` command dispatch | 0x48bf50 |
| Title menu (`Game.110`) | 0x490250 |
| Message window: sheets, ring button places (0x529b08) | 0x45eac0 |
| Click-wait hourglass: sprite, frame step | 0x463720, 0x46fc80 |
| Message window: band and window colour | 0x4639c0 |
| Message window: one button drawn | 0x460350 |
| Message window: button states | 0x45ffd0 |
| Name plate | 0x465e00 |
| αMAIL (`Game.1012`) handler | 0x4912d0 |
| αMAIL object: sheets loaded | 0x477ba0 |
| αMAIL subs 0, 1, 2, 3, 4, 5, 6, 8/9 | 0x4783b0, 0x477db0, 0x478060, 0x4788a0, 0x478bf0, 0x478e60, 0x479200, 0x479480 |
| Hologram (`Game.1013`) handler; her place, the panel's place | 0x491550; 0x468410, 0x468480 |
| Hologram: show (sub 2), fade out (sub 3), take away | 0x467eb0, 0x4681c0, 0x467720 |
| Light specks (`CMiniLightAnime`): set up, first specks, step, start again | 0x468660, 0x4689b0, 0x4688a0, 0x468c90 |
| Window portrait: face windows | 0x4625b0 |
| `Game.12`, `Game.13`, `Game.14` handlers | 0x48c710, 0x48ca40, 0x48d0e0 |
| Copies of a picture (`Game.995`): handler, set up, show, hide; spot tables | 0x48dad0, 0x487700, 0x487950, 0x487ce0; 0x574c60 (by count through 0x4891b0) |
| Deduction board (`Game.994`): handler, open (sub 2), close (sub 3), new picture (sub 4), marker (sub 5) | 0x48ddf0, 0x453fd0, 0x454250, 0x4544e0, 0x454790 |
| Notes and explanation screen (`Game.996`) | 0x48e060, 0x46d990, 0x46e000, 0x46e170 |
| Explanation screen: fade in (subs 13, 14), blink (15), pulse (19), fade out (17), into a layer (18) | 0x46e4c0, 0x46e6a0, 0x46e880, 0x46eba0, 0x46e260, 0x46e430 |
| Wavering picture (`CRasterAnime`): step, draw | 0x46fd50, 0x46ffa0 |
| Item panel (`Game.997`): handler, open, close | 0x48e4f0, 0x45a770, 0x45ab90 |
| Explanation slides (`Game.991`): handler, make (sub 0), drop, copy a picture (2), fade in (3, 4), blink (5), fade out (7), into the scene (8), repeat (9), scroll (10, 12) | 0x48d5d0, 0x484c60, 0x46e100, 0x484d50, 0x484fc0, 0x485200, 0x485410, 0x46e260, 0x484e10, 0x485a50, 0x485790, 0x485910 |
| DNA PC (`Game.1014`) | 0x4917e0, 0x452130, 0x451ff0 |
| DNA PC system window: open, close | 0x452400, 0x452ae0 |
| DNA strand: set up, fade in, fade out, frame, rows | 0x452d40, 0x452630, 0x452800, 0x4530d0, 0x453410 |
| αMAIL light specks: set up, step, first specks, start again | 0x47a0a0, 0x47a2a0, 0x47a3b0, 0x47a5a0 |
| Sprite animation: start (waits when asked), wait task | 0x4c0460, vtable 0x532a60 |
| HP gauges (`Game.1015`): handler, set up, colours and bar, queue | 0x4918b0, 0x457600, 0x458920, 0x457c80 |
| HP gauges: slide in, slide out, HP move, step of a slide and of an HP move | 0x457890, 0x4579f0, 0x458290, 0x458390, 0x458420 |
| HP gauges: shake step, its offset tables | 0x458480, 0x534770, 0x534b50 |
| Long voice (`Game.961`) | 0x491d70 |
| Voice-timed picture changes (`Game.1007`): start, timer, the four kinds (set up, step, end) | 0x48fde0, 0x454bc0, 0x454dd0, 0x454eb0..0x4569c0 |
| Extras (`Game.30` to `Game.32`) | 0x490990 |
| CG mode: open, page, buttons, click | 0x444640, 0x445370, 0x445000, 0x444780 |
| CG entries, pictures (and the copy the seen check reads) | 0x55cc70, 0x55d1e0 (0x562d00) |
| CG seen check, set by number (`Game.12.13`) | 0x446150, 0x4465a0 |
| CG entry grid: page, picture, next, previous | 0x448420, 0x448c00, 0x448ea0, 0x4491a0 |
| Completions; mode buttons open | 0x46eea0, 0x46f050, 0x46f110; 0x46ee30 |
| Scene mode: open, page, buttons, click, slide show | 0x4593a0, 0x459dc0, 0x459ae0, 0x459470, 0x45a410 |
| Yes/no dialog: made, drawn, item drawn; question rows | 0x4513d0, 0x451a00; 0x5299d0 |
| Back to the title or the scene list (dialog, then the scene list's end or the title) | 0x4b5260 |
| Scene bytes, slide show lengths | 0x56e8b0, 0x5299f0 |
| Music mode: open, list, scroll bar, play, volume | 0x469740, 0x46ad60, 0x46b1d0, 0x46a490, 0x46aa70 |
| Extras arrow keys (up, down, left, right): CG mode, CG entry grid, scene mode, music mode | 0x444ca0.., 0x447d00.., 0x459770.., 0x469f10.. (+0x50..+0x5c of each screen's vtable) |
| Extras fade in, fade out; key loop | 0x4a2bc0, 0x4a2d90; 0x4a6e60 |
| Standing characters: centre shift by slot, top edge by character | 0x44c690, 0x44c710 |
| Window portrait: place and size | 0x461ca0 |
| αMAIL subs 7, 10, 14 | 0x478700, 0x479790, 0x4786b0 |
| Game object (DNA PC at +0x50, HP gauges at +0x54) | 0x4b4210 |
| HP gauges: place on screen | 0x458140 |
| Timers (`Game.1006`) | 0x48f840 |
| Conclusions list (`Game.992`) | 0x491ea0, 0x453950 |
| HP over time (`Game.1015` subs 6 and 7) | 0x457b80, 0x458310, 0x458690 |
| αMAIL system window, body page, sent mail (`Game.1012` subs 7, 10, 14) | 0x478700, 0x479790, 0x4786b0 |
| Mail forward screen (`Game.1016`): handler, set up, show rows, click handling, row available check | 0x491c50, 0x45d210, 0x45e690, 0x45d7f0..0x45d9a7, 0x4a7600 |
| Mail forward screen: open and fade-in, draw one row, pointer update, right click | 0x45d470, 0x45e460, 0x45d600, 0x45db10 |
| Screen shake (`Game.993`), its step and offset tables | 0x48d380, 0x4779b0, 0x5346c0..0x534b6f |
| Zoom flash (`Game.1005`) | 0x48fa30 |
| Effect handlers: specks (1000), attacks (998), screen effects (1001), spells (1002) | 0x48ecd0, 0x48e610, 0x48f150, 0x48ee60 |
| Attacks: dispatch by kind; kind builders | 0x435130; 0x435400..0x438cc0 |
| Attacks: bullet streaks (set up, step, draw); fireball (set up, step) | 0x439fc0, 0x43a390, 0x43a530; 0x439940, 0x439a00 |
| Attacks: random slashes; sword rain | 0x43bbc0, 0x43c1c0; 0x43a7e0, 0x43aaa0 |
| Specks (`Game.1000`): set up, step, draw, end | 0x45c800, 0x45cab0, 0x45cd50, 0x45ce70 |
| Random slashes (`Game.1001` sub 6): object, step, draw; builder | 0x43b330, 0x43b0c0, 0x43afe0; 0x4822d0 |
| Spells (`Game.1002`): builders for subs 0..6, 9, 10; glitch bars | 0x489250, 0x4898d0, 0x48a000, 0x48a1c0, 0x48a4d0, 0x48a720, 0x48a920, 0x48acc0, 0x48b0f0; 0x489550 |
