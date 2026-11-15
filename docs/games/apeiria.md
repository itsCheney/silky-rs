# 景の海のアペイリア

English | [简体中文](apeiria.zh-CN.md)

Japanese build (SILKY'S PLUS WASABI, 2017) with the official update. The runtime knows the game by `apeiria.exe` in its folder; the build checked is the one whose `apeiria.exe` has SHA-256 `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e`.

All 171 scripts decode; the story text is in the scripts (no MAP tables). Headless, `silky-run snap` runs the whole story to its end with every engine and `Game` call carried out. The screens were compared with the game running under Wine.

## What works

- Ruby: `text 1` opens a ruby, its string goes above the text that follows. The runtime draws it at half size above that text.
- Selections are the engine's `Select.2`, which returns the option picked from 1.
- Title menu, read from the executable: six entries from `title.area`, the extras page (CG, scene, music) from `title2.area`, cells of `title_p.akb` in four 42-pixel state rows. Continue and load are grey without saves; CG, scene and music need bytes 3002, 3003 and 3004.
- Message window: a band at y = 508 (rows 212..424 of `message_p.akb`, the alpha mask the executable fills with the window colour) under faint lines (rows 0..212). The game's own defaults: colour red 0, green 25, blue 46 of 100, 30 % transparent.
- Window buttons, read from the executable: a ring around the `MENU` ball. The ball and the ring backdrop are 302-pixel cells of `menu_p00.akb`; the eleven buttons and the hide button are 40-pixel cells of `message_p.akb`. `mes_o.area` holds the ring buttons as ids 1..11 (save, load, quick save, quick load, voice, previous choice, backlog, auto, skip, next choice, settings); the hide button and the window are relative to the window top. The ball folds the ring away.
- Click wait: the hourglass of `message_p.akb` (ten 80-pixel frames from (1280, 0)) at (1204, 642), measured under Wine.
- Name plate, read from the executable: a left cap, 14-pixel middle pieces for the width of the name and a right end with its flourish, from row 584 of `message_p.akb`, each over the same piece of row 643 filled with the window colour. The name goes 69 pixels in and 17 down; the plate's place (47, 498) was measured under Wine.
- Text: glyphs about 24 pixels tall every 28.3 pixels from x = 152, lines 41 apart, measured under Wine.
- αMAIL, the in-game mail screen (`Game.1012`), read from the executable: the phone frame (`SLD.akb`) and the mail list of `mail01_p.akb` with the tray label (sub 0, list A or B), the newest mail row (byte 1200 or 1400, set by sub 11), the "new mail" notice with its count (subs 5 and 6), the selected row (subs 8 and 9) and the open mail on `mail02_p.akb` with its body text (subs 3 and 4). Places were checked against Wine; the body wrap width is a guess.
- Standing characters, read from the executable: the slot gives the centre (slot 1 at -400, 2 and 3 at +400 from the middle), and the top edge is (1 - size) · a - size · k with a and k by character (`chNN`). Checked against Wine.
- The window portrait (`Game.10` subs 6, 8, 11 to 13): the face of the body picture, cut by a window per character and drawn at 45 % at (-144, 440), with cheeks over it; the name plate moves to x = 210 while it is up. Characters take cheeks (`Game.13.6`), move at once (13.8) and change body in place (13.13).
- Apeiria on the hologram panel (`Game.1013`): the top or bottom half of `sld_mini.akb` under the characters and, over it, a window of her picture at half size with the expressions drawn over it; on the top half she is at 230 / 255 opacity. Checked against Wine.
- Pictures over the scene: copies of one picture at fixed spots (`Game.995`, the virus swarms; tables for 1, 2, 4, 8, 16 and 32 copies, scaled about their centres), the deduction board at (132, 15) (`Game.994`), the notes panel `k01_0N.akb` at (145, 0) and the explanation screen over `kbg.akb` (`Game.996`), the item panel at the left, its bottom at y = 495 (`Game.997`).
- The DNA PC (`Game.1014`): its screen goes into the background, the monitor over it, and the system window at (305, 37) is pieced from `apeirasystem.akb`.
- The HP gauges of the battles (`Game.1015`): five gauges cut from `hp.akb`, green from half, orange from a tenth, red below, a skull at 0; steps queued with sub 4 run with sub 9.
- A voice the next message does not cut (`Game.961`) and the auto save (`Game.14`).
- CG, scene and music modes (`Game.30` to `Game.32`), read from the executable and compared with the original under Wine. What was seen is kept in the byte variables from 3000, across games: a CG bit set from byte 3300, one bit per picture, set when the background and layers `Game.12` put up (subs 0, 1, 10, 12) match a row of the executable's table once the screen is shown (subs 2, 3, 4, 6, 11), or directly (sub 13); scenes from byte 3101 and music from byte 3050, set by the scripts; bytes 3002 to 3004 open the three modes.
  - CG mode: eight pages of twelve entries (`cgmode_thmNN.akb`), the count of seen pictures on each, and the completion of all three modes at the bottom. An entry opens a grid of its pictures over the dimmed page (from its `cg_evNNNN.akb` strip), with pages of twelve; a picture shows over the whole screen, and clicks walk through every seen picture to the last, then back to the grid. The wheel walks back and forth.
  - Scene mode: six pages of twelve scenes; under the pointer a scene shows a slide show of `scene_thumbNN.akb` (a second each, a second's fade). A click plays the scene.
  - Music mode: 41 tracks in eight slanted rows of `music_title.akb`, a scroll bar and the music volume. The CG and scene modes play `ape_27.ogg` when nothing plays; the music mode stops the music it finds.

## Not done yet

- In the extras: the fades between screens, keyboard moves, and the slide of the scroll bar's arrows while held.
- Effects that leave nothing behind are passed over: screen shakes (`Game.993`), attack and spell effects (998, 1001, 1002), the flash of a zoomed screen (1005), a sprite animation (1000). Fades, slides and particles of the pictures above are not done; they appear and go at once.
- The mouths of event pictures moving with the voice (`Game.1007`, `.vo` files), the draining of HP over time (`Game.1015` subs 6 and 7), the list of conclusions drawn into the background (`Game.992`), and `Game.1012` subs 7, 10 and 14.
- Text outline, the window position setting, the hourglass frame time.

## Where things are in the executable

Addresses in `apeiria.exe` of the build above. The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` command dispatch | 0x48bf50 |
| Title menu (`Game.110`) | 0x490250 |
| Message window: sheets, ring button places (0x529b08) | 0x45eac0 |
| Message window: band and window colour | 0x4639c0 |
| Message window: one button drawn | 0x460350 |
| Message window: button states | 0x45ffd0 |
| Name plate | 0x465e00 |
| αMAIL (`Game.1012`) handler | 0x4912d0 |
| αMAIL object: sheets loaded | 0x477ba0 |
| αMAIL subs 0, 1, 2, 3, 4, 5, 6, 8/9 | 0x4783b0, 0x477db0, 0x478060, 0x4788a0, 0x478bf0, 0x478e60, 0x479200, 0x479480 |
| Hologram (`Game.1013`) handler; her place, the panel's place | 0x491550; 0x468410, 0x468480 |
| Window portrait: face windows | 0x4625b0 |
| `Game.12`, `Game.13`, `Game.14` handlers | 0x48c710, 0x48ca40, 0x48d0e0 |
| Copies of a picture (`Game.995`): handler, set up, show, hide; spot tables | 0x48dad0, 0x487700, 0x487950, 0x487ce0; 0x574c60 (by count through 0x4891b0) |
| Deduction board (`Game.994`) | 0x48ddf0, 0x453fd0, 0x4544e0, 0x454790 |
| Notes and explanation screen (`Game.996`) | 0x48e060, 0x46d990, 0x46e000, 0x46e170 |
| Item panel (`Game.997`) | 0x48e4f0, 0x45a770 |
| DNA PC (`Game.1014`) | 0x4917e0, 0x452130, 0x451ff0 |
| HP gauges (`Game.1015`): handler, set up, colours and bar, queue | 0x4918b0, 0x457600, 0x458920, 0x457c80 |
| Long voice (`Game.961`) | 0x491d70 |
| Event picture lip sync (`Game.1007`) | 0x48fde0, 0x454bc0 |
| Extras (`Game.30` to `Game.32`) | 0x490990 |
| CG mode: open, page, buttons, click | 0x444640, 0x445370, 0x445000, 0x444780 |
| CG entries, pictures (and the copy the seen check reads) | 0x55cc70, 0x55d1e0 (0x562d00) |
| CG seen check, set by number (`Game.12.13`) | 0x446150, 0x4465a0 |
| CG entry grid: page, picture, next, previous | 0x448420, 0x448c00, 0x448ea0, 0x4491a0 |
| Completions; mode buttons open | 0x46eea0, 0x46f050, 0x46f110; 0x46ee30 |
| Scene mode: open, page, buttons, click, slide show | 0x4593a0, 0x459dc0, 0x459ae0, 0x459470, 0x45a410 |
| Scene bytes, slide show lengths | 0x56e8b0, 0x5299f0 |
| Music mode: open, list, scroll bar, play, volume | 0x469740, 0x46ad60, 0x46b1d0, 0x46a490, 0x46aa70 |
