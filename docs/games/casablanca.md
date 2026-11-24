# 景の海のアペイリア ～カサブランカの騎士～

English | [简体中文](casablanca.zh-CN.md)

The fan disc of 景の海のアペイリア. Japanese build (SILKY'S PLUS WASABI, 2017). The runtime knows the game by `casablanca.exe` in its folder; the build described here has SHA-256 `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716`. Program version 1.0.0.0, linked 2017-12-10. Engine profile: [engine-2017](../builds/engine-2017.md).

It runs on the Apeiria module. The program is built from the same code as `apeiria.exe`: 4049 of its 4643 functions match, 3959 of them identical. The engine classes, the `Game` dispatcher, title menu, message window, αMAIL, hologram and gauges are the same code, so everything on the [Apeiria page](apeiria.md) applies here and is not repeated. This page lists what differs.

Scripts: `FD0101` to `FD4102` (story), `FD_SCENE_01` to `FD_SCENE_14` (scene mode), `MAIN`, `START`, `OMAKE`, `SCMODE`, `FLAGINI`. They are compiled with debug line markers (`dbg.line`, `dbg.call`) between statements, and a message's text is pushed on the line before the call that shows it (`BackLog.3` inside the `main.inc` macro). The VM keeps message text pushed before a debug marker as part of the next engine call; this was added for this game.

The story comes back to a mail choice (沙羅へメールを送る, 七海へメールを送る) after each route; once both are read it goes on to the last chapter. Headless, `silky-run snap --path 0,1` plays both routes and the last chapter to the ending movie and back to the title, with every engine and `Game` call carried out. The first line's frame matches the original's under Wine, and the engine calls of the first 1,829 lines (from `fd0101.mes` into `fd0201.mes`, 13,526 calls) match the original's but for one thing: after `MesWnd.3` the original runs a voiced line's name and voice calls (the ones between `BackLog.2` and `BackLog.3`) a second time. The engine keeps those calls as the line's backlog record: `BackLog.2` starts it, every script instruction run until `BackLog.3` is appended to it, and the backlog runs it again to replay the voice. The code shows where: when `MesWnd.3` sets up the line, the message window runs the line's backlog record once more with a flag set that makes the text instructions count characters per line instead of drawing (it measures the line). The name and voice calls in the record run again only because the native calls do not check that flag. They set the same name and voice again; the voice was already started at `MesWnd.3`, so nothing on screen or in sound changes, and the runtime runs them once. The second run is not done when SceneSkip is on and the line was already read (inferred).

## What differs from Apeiria

### Effects

Most effects look the same. Many fade-ins became wipes through rule pictures, and the fade-outs stay fades.

- Zoom flash (`Game.1005`): with the scene-skip setting on, the original does nothing at all (no zoom, flash or fade); with it off, the zoom and white flash of Apeiria. The setting is not offered here and reads as off unless a script writes `/Config/SceneSkip`, which the scripts do not.
- Wipes instead of fade-ins: attack kinds 1 to 5, 13 and 15; spells sub 4 and 9; `Game.1001` subs 1, 3, 14, 15 and 20. Each uses its own rule picture, as the executable does.
- Attack kind 18 (`Game.998`): a pillar of fire (`attack_17_00.akb`) drops in from above, then wipes out. The script waits until it is gone.
- Attack kind 14 takes an argument: 0 shows all ten pictures, 1 the first five, 2 the last five.
- `Game.998` with a sub above 4 does nothing (two calls in the scripts).
- `Game.13` sub 12 (alpha, time, size, name, slot): a standing character at an opacity.
- `Game.10` sub 14 is a start-up call with nothing to do.
- Deduction board (`Game.994`): the same as in Apeiria (sub 2 opens it from a line in the middle, 3 closes it, 5 zooms a marker), except that sub 4 wipes the new picture in through rule 1 over 24 frames where Apeiria fades it in. The script waits for each animation (open, close and change 24 frames, marker 12), and a click does not cut it short. When `/Config/SceneSkip` is 1 the engine skips subs 0, 2, 3, 4 and 5 entirely, and the runtime does the same. The story scripts only close and drop the board at start-up.
- `Game.991`: Casablanca never shows this panel. It is Apeiria's explanation-slide panel, left in the engine: Casablanca's data has none of its pictures (`tbg.akb`, `wbg.akb`, the `t*` and `w*` slides), its scripts only call sub 1 (destroy, with nothing to destroy) at start-up, and the load code that would rebuild it reads a save value Casablanca never writes. Its sub 3 (a wipe in through a rule picture) has no caller.
- Conclusions list (`Game.992`): cells are counted from 1, so the cell for value `v` is column (v - 1) / 100, row (v - 1) % 100.

### Extras

- CG mode: 5 pages of 3 x 3 cells, 43 entries, 243 pictures. The entry grid keeps its 12 cells a page. The entries are in table order, not sorted by picture, and the seen bit of a picture is not always its row (two pictures inserted in the 16-picture entry that starts with `cg_ev1209.akb` have bits 241 and 242). The CG tracker also keeps backgrounds named `cg...`, not only `ev...`.
- Scene mode: 14 scenes, 3 x 2 cells a page, 3 pages, seen bytes 3101 to 3114. A scene plays `FD_SCENE_NN.MES`.
- Music mode: 32 `.wav` tracks in 8 rows; the heard bytes are not in row order. The extras play `casablanca_shortinst.wav`.
- The scroll bar, the page buttons and the arrow keys follow the new counts. Up from the next arrow of the CG screen goes nowhere, as in the original.
- Nothing is remembered between openings, in either game. Each time the CG, scene or music screen opens, including when you switch between the three, it starts on the first page (or the top of the music list). The page, scene page and scroll position live only in the screen itself. The one thing the CG screen reads when it opens is a flag in the game's state record with a picture number to start from (the page is that number divided by 16); the scripts never set it, so it never applies.

## Not done yet

- The staff roll and the ending pictures are scripted and play as other pictures do.
- The effects are ported from the executable's code but not compared frame by frame with the original.

## Where things are in the executable

Addresses in `casablanca.exe` of the build above (image base 0x400000). The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` command dispatch | 0x48d7d0 |
| Attacks: dispatch by kind; kind 18 builder | 0x435d50; 0x43a0c0 |
| Spells (`Game.1002`) builders for subs 0, 1, 3, 9 | 0x48aa90, 0x48b100, 0x48b9e0, 0x48c4f0 |
| Zoom flash (`Game.1005`) | 0x4912e0 |
| Conclusions list (`Game.992`) | 0x455170 |
| Deduction board (`Game.994`): handler, open (sub 2), close (sub 3), new picture (sub 4), marker (sub 5) | 0x48f6a0, 0x4557d0, 0x455a50, 0x455ce0, 0x455fa0 |
| `Game.991`: handler, wipe in (sub 3) | 0x48ee70, 0x486740 |
| Message window line set-up; replay of the backlog record; character count while measuring | 0x466120; 0x4b9410; 0x4b9560 |
| Notes and explanation screen (`Game.996`) | 0x48f910 |
| Screen effects (`Game.1001`) subs 1, 3, 14, 15, 20 | 0x482b10, 0x4831b0, 0x4852d0, 0x4854e0, 0x485f80 |
| Ending picture preload | 0x480c50 |
| Extras (`Game.30` to `Game.32`) | 0x492280 |
| Mode open test (bytes 3002 to 3004) | 0x4706f0 |
| CG mode: page, buttons | 0x4469c0, 0x446650 |
| CG mode: open (first page) | 0x445cb0 |
| CG tracker (`Game.12` background), seen check, set by number | 0x447a40, 0x447820, 0x447cd0 |
| CG entries, pictures (and the copy the seen check reads) | 0x55cc70, 0x55cf30 (0x55f180) |
| CG entry grid: page, picture, next, previous | 0x449bb0, 0x44a3b0, 0x44a690, 0x44a9a0 |
| CG, scene and music completion | 0x470760, 0x470910, 0x4709e0 |
| Scene mode: open, buttons, page, slide show | 0x45a920, 0x45abe0, 0x45b310, 0x45b5e0, 0x45bc60 |
| Scene page table, slide show lengths | 0x5613d0, 0x52b9f0 |
| Music mode: play, list, scroll bar, volume, open | 0x46bcf0, 0x46c560, 0x46ca70, 0x46c270, 0x46b024 |
| Music track names, extras music name | 0x52ec74.., 0x52edf0 |
| Extras arrow keys: CG mode, CG entry grid, scene mode, music mode | 0x4462f0.., 0x449490.., 0x45afb0.., 0x46b770.. |
| Extras fade in, fade out; key loop | 0x4a44e0, 0x4a46b0; 0x4a8780 |
