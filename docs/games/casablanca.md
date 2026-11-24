# 景の海のアペイリア ～カサブランカの騎士～

English | [简体中文](casablanca.zh-CN.md)

The fan disc of 景の海のアペイリア. Japanese build (SILKY'S PLUS WASABI, 2017). The runtime knows the game by `casablanca.exe` in its folder; the build described here has SHA-256 `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716`. Program version 1.0.0.0, linked 2017-12-10. Engine profile: [engine-2017](../builds/engine-2017.md).

It runs on the Apeiria module. The program is built from the same code as `apeiria.exe`: 4049 of its 4643 functions match, 3959 of them identical. The engine classes, the `Game` dispatcher, title menu, message window, αMAIL, hologram and gauges are the same code, so everything on the [Apeiria page](apeiria.md) applies here and is not repeated. This page lists what differs.

Scripts: `FD0101` to `FD4102` (story), `FD_SCENE_01` to `FD_SCENE_14` (scene mode), `MAIN`, `START`, `OMAKE`, `SCMODE`, `FLAGINI`. They are compiled with debug line markers (`dbg.line`, `dbg.call`) between statements, and a message's text is pushed on the line before the call that shows it (`BackLog.3` inside the `main.inc` macro). The VM keeps message text pushed before a debug marker as part of the next engine call; this was added for this game.

The story comes back to a mail choice (沙羅へメールを送る, 七海へメールを送る) after each route; once both are read it goes on to the last chapter. Headless, `silky-run snap --path 0,1` plays both routes and the last chapter to the ending movie and back to the title, with every engine and `Game` call carried out. The first line's frame matches the original's under Wine, and the opening's engine calls match the original's but for one thing: after `MesWnd.3` the original runs a voiced line's name and voice calls (the ones between `BackLog.2` and `BackLog.3`) a second time. That is seen in the trace, not read from the code. The second run repeats the same name and voice at once, so the runtime runs them once.

## What differs from Apeiria

### Effects

Most effects look the same. Many fade-ins became wipes through rule pictures, and the fade-outs stay fades.

- Wipes instead of fade-ins: attack kinds 1 to 5, 13 and 15; spells sub 4 and 9; `Game.1001` subs 1, 3, 14, 15 and 20. Each uses its own rule picture, as the executable does.
- Attack kind 18 (`Game.998`): a pillar of fire (`attack_17_00.akb`) drops in from above, then wipes out. The script waits until it is gone.
- Attack kind 14 takes an argument: 0 shows all ten pictures, 1 the first five, 2 the last five.
- `Game.998` with a sub above 4 does nothing (two calls in the scripts).
- `Game.13` sub 12 (alpha, time, size, name, slot): a standing character at an opacity.
- `Game.10` sub 14 is a start-up call with nothing to do.
- Conclusions list (`Game.992`): cells are counted from 1, so the cell for value `v` is column (v - 1) / 100, row (v - 1) % 100.

### Extras

- CG mode: 5 pages of 3 x 3 cells, 43 entries, 243 pictures. The entry grid keeps its 12 cells a page. The entries are in table order, not sorted by picture, and the seen bit of a picture is not always its row (two pictures inserted in the 16-picture entry that starts with `cg_ev1209.akb` have bits 241 and 242). The CG tracker also keeps backgrounds named `cg...`, not only `ev...`.
- Scene mode: 14 scenes, 3 x 2 cells a page, 3 pages, seen bytes 3101 to 3114. A scene plays `FD_SCENE_NN.MES`.
- Music mode: 32 `.wav` tracks in 8 rows; the heard bytes are not in row order. The extras play `casablanca_shortinst.wav`.
- The scroll bar, the page buttons and the arrow keys follow the new counts. Up from the next arrow of the CG screen goes nowhere, as in the original.

## Not done yet

- The deduction board picture (`Game.994` sub 4) shows at once; the original wipes it in (rule 1).
- `Game.991` sub 3 (unused by the scripts) has a wipe with a rule taken from an argument.
- The CG page is not restored when the CG screen opens again.
- The zoom flash reads the scene-skip setting in the original; this is not followed.
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
| Deduction board (`Game.994`) | 0x455ce0 |
| Notes and explanation screen (`Game.996`) | 0x48f910 |
| Screen effects (`Game.1001`) subs 1, 3, 14, 15, 20 | 0x482b10, 0x4831b0, 0x4852d0, 0x4854e0, 0x485f80 |
| Ending picture preload | 0x480c50 |
| Extras (`Game.30` to `Game.32`) | 0x492280 |
| Mode open test (bytes 3002 to 3004) | 0x4706f0 |
| CG mode: page, buttons | 0x4469c0, 0x446650 |
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
