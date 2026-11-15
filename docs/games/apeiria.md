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
- Text: glyphs about 24 pixels tall every 28.3 pixels from x = 152, lines 41 apart, measured under Wine.

## Not done yet

- The game's own commands: `Game.1012` to `Game.1016`, `Game.99x`, `Game.1000` to `Game.1007` and others are reported. Among them is the in-game mail screen.
- Speaker names (`Game.11` with the name) are not drawn.
- Click-wait icon, text outline, the window position setting.

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
