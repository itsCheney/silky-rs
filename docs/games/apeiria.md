# 景の海のアペイリア

English | [简体中文](apeiria.zh-CN.md)

Japanese build (SILKY'S PLUS WASABI, 2017) with the official update. The runtime knows the game by `apeiria.exe` in its folder; the build checked is the one whose `apeiria.exe` has SHA-256 `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e`.

Bring-up has started. All 171 scripts decode; the story text is in the scripts (no MAP tables). Headless, `silky-run snap` goes from the title menu through the first chapters with backgrounds, standing characters and the message window. The screens were compared with the game running under Wine.

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

## Not done yet

- The game's own commands: `Game.1013` to `Game.1016`, `Game.99x`, `Game.1000` to `Game.1007` and others are reported. `Game.1012` subs 7, 10 and 12 to 14 (list scrolling and animation) are reported too.
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
