# 景の海のアペイリア

English | [简体中文](apeiria.zh-CN.md)

Japanese build (SILKY'S PLUS WASABI, 2017) with the official update. The runtime knows the game by `apeiria.exe` in its folder; the build checked is the one whose `apeiria.exe` has SHA-256 `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e`. Program version 1.0.3.0, linked 2017-11-09; its engine profile is [engine-2017](../builds/engine-2017.md).

All 171 scripts decode; the story text is in the scripts (no MAP tables). Headless, `silky-run snap` runs the whole story to its end with every engine and `Game` call carried out. The screens were compared with the game running under Wine.

## What works

- Ruby: `text 1` opens a ruby, its string goes above the text that follows. The runtime draws it at half size above that text.
- Selections are the engine's `Select.2`, which returns the option picked from 1.
- Title menu, read from the executable: six entries from `title.area`, the extras page (CG, scene, music) from `title2.area`, cells of `title_p.akb` in four 42-pixel state rows. Continue and load are grey without saves; CG, scene and music need bytes 3002, 3003 and 3004.
- Message window: a band at y = 508 (rows 212..424 of `message_p.akb`, the alpha mask the executable fills with the window colour) under faint lines (rows 0..212). The game's own defaults: colour red 0, green 25, blue 46 of 100, 30 % transparent.
- Window buttons, read from the executable: a ring around the `MENU` ball. The ball and the ring backdrop are 302-pixel cells of `menu_p00.akb`; the eleven buttons and the hide button are 40-pixel cells of `message_p.akb`. `mes_o.area` holds the ring buttons as ids 1..11 (save, load, quick save, quick load, voice, previous choice, backlog, auto, skip, next choice, settings); the hide button and the window are relative to the window top. The ball folds the ring away.
- Click wait: the hourglass of `message_p.akb` (ten 76 x 80 frames down from (1280, 0)) at (1204, 642), a frame every tenth tick, 200 ms as measured under Wine.
- The window position: the program places the window parts relative to `/Config/MesWnd/Pos` (the hourglass at Pos + 132) but only reads it; the config screen has no control for it, so the window stays where it starts.
- Name plate, read from the executable: a left cap, 14-pixel middle pieces for the width of the name and a right end with its flourish, from row 584 of `message_p.akb`, each over the same piece of row 643 filled with the window colour. The name goes 69 pixels in and 17 down; the plate's place (47, 498) was measured under Wine.
- Text: glyphs about 24 pixels tall every 28.3 pixels from x = 152, lines 41 apart, bold with a one-pixel black edge, measured under Wine.
- Screen shake (`Game.993`), read from the executable: sub 0 sets it up, 2 picks one of two tables of eight offsets (sideways only, or both ways, up to 16 pixels), 3 starts it, 1 stops it. The screen takes the next offset each step; the step time (a sixtieth of a second) is a guess.
- The white flash of a zoomed screen (`Game.1005`), read from the executable: the screen zooms to 120 %, turns white and fades back over 16 frames, the same steps as ふゆから、くるる。's.
- Pictures that change during a voice line (`Game.1007`, `.vo` files), read from the executable: a set time after the voice starts, the window portrait (0), a standing character (1), the event picture and its difference picture (2) or Apeiria on the hologram panel (3) change to the face the file names, with cheeks when its flag is set. The next message makes the change at once if its time has not come. The `.vo` layout is older than ふゆから、くるる。's: a head of three u32 (count, what changes, milliseconds), then 792-byte records (slot, size, flag, three 260-byte names). The original fades the new picture in; here it changes at once.
- With movies on (the setting the program starts with), the scripts ask for movies of characters changing clothes over the scene (`Game.999`, `chNN_chg.vsd`) and move the copy of the screen a shake uses (`Sprite.13`); the movies are not played, so the scene stays as with movies off, but `Movie.3` answers 1 after one as the original does.
- αMAIL, the in-game mail screen (`Game.1012`), read from the executable: the phone frame (`SLD.akb`) and the mail list of `mail01_p.akb` with the tray label (sub 0, list A or B), the newest mail row (byte 1200 or 1400, set by sub 11), the "new mail" notice with its count (subs 5 and 6), the selected row (subs 8 and 9) and the open mail on `mail02_p.akb` with its body text (subs 3 and 4), scrolled by pages of 381 pixels (sub 10). Sub 7 puts the Apeiria system window in the panel instead of the list, at (309, 36); sub 14 puts a mail in the first free byte of 1400 to 1499. Places were checked against Wine; the white list is seen down to 447 pixels below its top, its last 42 rows fading out linearly into the glass; the body wrap width is a guess. The panel opens (sub 1) by wiping in from its middle through rule 46 over 64 frames while 50 specks of `light.akb` rise into it, and closes (sub 2) by shrinking to a tenth of its height about its middle over 16 frames; the script waits for both.
- Standing characters, read from the executable: the slot gives the centre (slot 1 at -400, 2 and 3 at +400 from the middle), and the top edge is (1 - size) · a - size · k with a and k by character (`chNN`). Checked against Wine.
- The window portrait (`Game.10` subs 6, 8, 11 to 13): the face of the body picture, cut by a window per character and drawn at 45 % at (-144, 440), with cheeks over it; the name plate moves to x = 210 while it is up. Characters take cheeks (`Game.13.6`), move at once (13.8) and change body in place (13.13).
- Apeiria on the hologram panel (`Game.1013`): the top or bottom half of `sld_mini.akb` under the characters and, over it, a window of her picture at half size with the expressions drawn over it; on the top half she is at 230 / 255 opacity. Checked against Wine. Her coming and going, read from the executable: way 0 of sub 2 wipes her picture in from the middle on the top half through rule 47 (`47.bin`, laid over the panel) over 12 frames while 50 specks of `light.akb` rise from the bottom of the screen into the panel (`CMiniLightAnime`); on the bottom half way 0, and way 2 on either, fade her in from nothing over 12 frames; the script waits for both, as it does while sub 3 fades her and the panel out over the frames it is given. Shown again while she is up, she comes again the same way; shown while still coming, the script waits for that. The wipe and specks were not seen under Wine yet: in the stretch traced she first comes while the screen changes.
- Pictures over the scene: copies of one picture at fixed spots (`Game.995`, the virus swarms; tables for 1, 2, 4, 8, 16 and 32 copies, scaled about their centres), the deduction board at (132, 15) (`Game.994`), the notes panel `k01_0N.akb` at (145, 0) and the explanation screen over `kbg.akb` (`Game.996`), the item panel at the left, its bottom at y = 495 (`Game.997`). Their animations, read from the executable:
  - Copies (995 subs 1 and 2, time, wave, to, from) fade in from nothing to their opacity, or out, over `time` frames while the script waits. With wave 1 the first of them wavers row by row as it goes (from 256 pixels and 20 degrees a row down to nothing, or back); the scripts never ask for it.
  - The explanation screen is the scene with the frame and the picture drawn in. A new one (996 subs 13 and 14) fades in over the old one in `time` frames while the script waits; the rule sub 13 is given is not used. Sub 15 makes the new one blink over the old one (after 42 frames it fades out over 8, after 128 more back in over 8) and sub 19 fades it in again and again over `time` frames, until sub 16 stops it where it is. Sub 17 fades the screen out over 12 frames while the script waits; sub 18 draws it into the background or the picture layer, where it stays until the background changes.
  - The item panel (997) opens like a screen switched on, a line 1 % high growing to its width over 8 frames, then to its height over 4; it closes by its height going to nothing over 12. The script waits for both.
- The DNA PC (`Game.1014`): its screen goes into the background, the monitor over it, and the system window at (305, 37) is pieced from `apeirasystem.akb`. The window opens (sub 1) with its height growing from nothing about its middle over 16 frames and closes (sub 2) the other way; the script waits for both. Sub 4 brings a strand of DNA into the window: `RedLine.akb` and `BlueLine.akb`, 16 x 430 each at (330, 15) in the window, their rows moved sideways along a sine wave 85 pixels wide, half a degree a row, the wave going on 2 degrees a frame with the blue line half a turn behind. Blue lies over red in the top 200 rows and red over blue in the next 200. It fades in over 16 frames and sub 5 fades it out over 16, both while the script waits; meanwhile the window shows only its background, without the emblem and the bar. Sub 3 hides it with the window.
- The HP gauges of the battles (`Game.1015`): five gauges cut from `hp.akb`, green from half, orange from a tenth, red below, a skull at 0; steps queued with sub 4 run with sub 9. A gauge slides in to x = 8 (sub 1) or out to x = -424 (sub 2) over 16 frames, and sub 3 moves its HP over 128 frames; the script goes on meanwhile. A gauge whose HP drops shakes for 16 frames, going round right, down, left and up by 2 pixels (a drop up to 20), 8 (up to 40) or 16. Sub 6 (id, interval, count, delta, gauge) moves a gauge's HP by delta every interval milliseconds, count times, kept between 1 and 100 and ending at 1; sub 7 (id, gauge) stops it. The original also writes the HP to word variables 100 to 104, which the scripts do not read.
- The list of conclusions (`Game.992`): 250 x 51 cells of `dnapcA_mailparts.akb` (column v / 100, row v % 100) for bytes 1300 to 1306, as many as byte 1320 says, the last first, drawn into the background from (356, 103) down; a value under 100 gets 100 added once drawn. Sub 1 (text, page, cell) draws a mail into the background the same way: the header cell of the same sheet at (655, 73) and the text, black, wrapped at 276 pixels, in a 300 x 272 view at (641, 161) scrolled by page.
- Attack and spell effects, read from the executable and played as its keyframe animations (16 ms steps):
  - `Game.998`, the battle attacks: kinds 0 to 17 (sub 0) build sprites from the attack sheets with their moves, fades, turns, scales and wipes; the bullet streaks, the fireball, the sword rain, the slashes and the white flashes are drawn by the executable's own per-frame code, ported with its random numbers. Sub 1 keeps an attack up until sub 4 takes it down; kind 14 shows the ev1701 pictures one after another, each drawn into the background as its fade ends, so they stay when the attack is taken off.
  - `Game.1000`, the specks at the top of the screen (subs 0 to 2).
  - `Game.1001`, the screen effects (subs 0 to 21 and 255), among them the random slashes of sub 6.
  - `Game.1002`, the spells: a mosaic in or out (0 and 1, from a block size to another over a number of frames), the cross fade of a picture with a rule (2, 3), the dark red screen (4), the flash (5), the cut between two event pictures (6; the second is drawn into the background), the glitch bars (9) and the rule reveal (10).
  - The script waits while an effect blocks, as in the original.
  - What the effects draw into the background (kind 14 of `Game.998`; spells 4 and 6 of `Game.1002`, which leave the screen dark red or the second picture) is kept with the scene like the conclusions list: it lies under the characters, is saved with it, and goes when the script sets a new background. A screen left covered by spell 0 or an increasing spell 1 is a sprite over everything and likewise stays until the background changes.
- A voice the next message does not cut (`Game.961`) and the auto save (`Game.14`).
- CG, scene and music modes (`Game.30` to `Game.32`), read from the executable and compared with the original under Wine. What was seen is kept in the byte variables from 3000, across games: a CG bit set from byte 3300, one bit per picture, set when the background and layers `Game.12` put up (subs 0, 1, 10, 12) match a row of the executable's table once the screen is shown (subs 2, 3, 4, 6, 11), or directly (sub 13); scenes from byte 3101 and music from byte 3050, set by the scripts; bytes 3002 to 3004 open the three modes.
  - CG mode: eight pages of twelve entries (`cgmode_thmNN.akb`), the count of seen pictures on each, and the completion of all three modes at the bottom. An entry opens a grid of its pictures over the dimmed page (from its `cg_evNNNN.akb` strip), with pages of twelve; a picture shows over the whole screen, and clicks walk through every seen picture to the last, then back to the grid. The wheel walks back and forth.
  - Scene mode: six pages of twelve scenes; under the pointer a scene shows a slide show of `scene_thumbNN.akb` (a second each, a second's fade). A click plays the scene.
  - Music mode: 41 tracks in eight slanted rows of `music_title.akb`, a scroll bar and the music volume. The CG and scene modes play `ape_27.ogg` when nothing plays; the music mode stops the music it finds. The scroll bar's arrows scroll one row as they go down and not again while held, as under Wine.
  - The modes fade in from black when they open and out to black when they close, 12 steps of the 20 ms tick (240 ms, matched under Wine); input waits for the fade. The title menu fades out with the screen.
  - Arrow keys move the pointer to the middle of another button, by each screen's own table (a key from no button goes to the first cell), and Enter clicks there. On a picture over the whole screen, left and right walk back and forth.

## Not done yet

- Under Wine the original holds the script 0.6 to 0.8 seconds inside each way-0 show of the hologram (timed with the trace probe). The executable waits for her 12 frames of coming each time, also when she is up already; the rest is most likely the slow frame rate under Wine, which is not measured yet.
- The animations of the pictures over the scene, the αMAIL panel and the DNA PC window follow the executable's code and were checked in the runtime only.
- Of the effects above, only `Game.1001` sub 1 was compared frame by frame with the original under Wine; the rest follow the executable's code and were checked in the runtime only.

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
| Deduction board (`Game.994`) | 0x48ddf0, 0x453fd0, 0x4544e0, 0x454790 |
| Notes and explanation screen (`Game.996`) | 0x48e060, 0x46d990, 0x46e000, 0x46e170 |
| Explanation screen: fade in (subs 13, 14), blink (15), pulse (19), fade out (17), into a layer (18) | 0x46e4c0, 0x46e6a0, 0x46e880, 0x46eba0, 0x46e260, 0x46e430 |
| Wavering picture (`CRasterAnime`): step, draw | 0x46fd50, 0x46ffa0 |
| Item panel (`Game.997`): handler, open, close | 0x48e4f0, 0x45a770, 0x45ab90 |
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
| Save list (`Game.1016`) | 0x491c50 |
| Screen shake (`Game.993`), its step and offset tables | 0x48d380, 0x4779b0, 0x5346c0..0x534b6f |
| Zoom flash (`Game.1005`) | 0x48fa30 |
| Effect handlers: specks (1000), attacks (998), screen effects (1001), spells (1002) | 0x48ecd0, 0x48e610, 0x48f150, 0x48ee60 |
| Attacks: dispatch by kind; kind builders | 0x435130; 0x435400..0x438cc0 |
| Attacks: bullet streaks (set up, step, draw); fireball (set up, step) | 0x439fc0, 0x43a390, 0x43a530; 0x439940, 0x439a00 |
| Attacks: random slashes; sword rain | 0x43bbc0, 0x43c1c0; 0x43a7e0, 0x43aaa0 |
| Specks (`Game.1000`): set up, step, draw, end | 0x45c800, 0x45cab0, 0x45cd50, 0x45ce70 |
| Random slashes (`Game.1001` sub 6): object, step, draw; builder | 0x43b330, 0x43b0c0, 0x43afe0; 0x4822d0 |
| Spells (`Game.1002`): builders for subs 0..6, 9, 10; glitch bars | 0x489250, 0x4898d0, 0x48a000, 0x48a1c0, 0x48a4d0, 0x48a720, 0x48a920, 0x48acc0, 0x48b0f0; 0x489550 |
