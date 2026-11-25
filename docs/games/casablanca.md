# 景の海のアペイリア ～カサブランカの騎士～

English | [简体中文](casablanca.zh-CN.md)

The fan disc of 景の海のアペイリア. Japanese build (SILKY'S PLUS WASABI, 2017). The runtime knows the game by `casablanca.exe` in its folder; the build described here has SHA-256 `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716`. Program version 1.0.0.0, linked 2017-12-10. Engine profile: [engine-2017](../builds/engine-2017.md).

It runs on the Apeiria module. The program is built from the same code as `apeiria.exe`: 4049 of its 4643 functions match, 3959 of them identical. The engine classes, the `Game` dispatcher, title menu, message window, αMAIL, hologram and gauges are the same code, so everything on the [Apeiria page](apeiria.md) applies here and is not repeated. This page lists what differs.

Scripts: `FD0101` to `FD4102` (story), `FD_SCENE_01` to `FD_SCENE_14` (scene mode), `MAIN`, `START`, `OMAKE`, `SCMODE`, `FLAGINI`. They are compiled with debug line markers (`dbg.line`, `dbg.call`) between statements, and a message's text is pushed on the line before the call that shows it (`BackLog.3` inside the `main.inc` macro). The VM keeps message text pushed before a debug marker as part of the next engine call; this was added for this game.

The story comes back to a mail choice (沙羅へメールを送る, 七海へメールを送る) after each route; once both are read it goes on to the last chapter. Headless, `silky-run snap --path 0,1` plays both routes and the last chapter to the ending movie and back to the title, with every engine and `Game` call carried out. The first line's frame matches the original's under Wine, and the engine calls of the whole story (`silky-run snap --path 0,1 --xml /Config/Movie/Play=0` against the original under Wine taking the same two mail choices, 15,103 lines to the title) match the original's, but for the repeated name and voice calls explained below and the random pick of the title's first sound (`seg_000?.wav`). The repeated calls are these: after `MesWnd.3` the original runs a voiced line's name and voice calls (the ones between `BackLog.2` and `BackLog.3`) a second time. The engine keeps those calls as the line's backlog record: `BackLog.2` starts it, every script instruction run until `BackLog.3` is appended to it, and the backlog runs it again to replay the voice. The code shows where: when `MesWnd.3` sets up the line, the message window runs the line's backlog record once more with a flag set that makes the text instructions count characters per line instead of drawing (it measures the line). The name and voice calls in the record run again only because the native calls do not check that flag. They set the same name and voice again; the voice was already started at `MesWnd.3`, so nothing on screen or in sound changes, and the runtime runs them once. The second run is not done when SceneSkip is on and the line was already read: the line set-up (0x466120) reads `/Config/SceneSkip` (0x5308f8, at 0x466164) and the line's read flag before it runs the record again, and the runtime does not run it a second time in any case. Movies are off in that comparison because the original's configuration under Wine had `/Config/Movie/Play` 0; with it at 1 the scripts play `matrix.vsd` and the other movies where the original showed their akb animations. The runtime's default is 1, which is the executable's own: every reader of `/Config/Movie/Play` asks the XML reader (0x4ca8e0, which returns the caller's default when the node is missing) with the default 1 (the movie object at 0x42c1e3, 0x42c306 and 0x42c509; the configuration object's load at 0x496b02, which keeps it in a field that the save at 0x496eb4 writes back). Nothing in the program sets it to 0: those five places are the only references to the string, and no code stores 0 in that field; the scripts never touch it (a whole-story trace has `Xml.1` for `/Config/OneMes` and `/Config/AutoRead` only), and a movie that cannot start does not write it. The 0 in the Wine file did not come from the engine or an installer: our own Wine set-up wrote that file, from the sample configuration in the game's archive with `/Config/Movie/Play` set to 0. Without a configuration file the original faults before the title because the first `Xml.1` of `start.mes` (`/Config/OneMes`) has no tree to write into: the loader (0x4ca3d0) leaves the root empty when the file cannot be opened, and the set-by-path function (0x4caea0) then adds the first node to that empty root, reading offset 0x50 of a null pointer (0x4cb1be: page fault at 0x00000050). Reads on an empty tree are fine (they return the default). So a missing file is expected to crash; the engine does not create it and has no built-in copy. The file under `Save/` is a copy of `FileInfo.xml` (the one in the game folder) with the engine's `<Config>` added at the end, and the program expects whatever installs the game to put it there (`<name>_inst.xml` next to the `Save` folder, `/InstallSetting/InstallPath` and `/InstallSetting/RunExe`, is loaded as well and nothing reads it through the configuration pointer; that the installer copies `FileInfo.xml` is inferred from the files, not from code). The first-run default of the other settings is the one in the code too, for example the message window colour (0, 25, 46) and alpha 30 at 0x496a0b onwards.

## What differs from Apeiria

### Effects

Most effects look the same. Many fade-ins became wipes through rule pictures, and the fade-outs stay fades.

The effects were compared frame by frame with the original under Wine at eight places in the story: `Game.1001` subs 1 and 2, the dragon and attack 18, the `Game.12` cut-ins, `Game.13` sub 12 at two opacities, and the static screen before the last chapter. They match, after two fixes: the soft edge of the sprite wipes and the opacity of a character put up again (both below).

- Zoom flash (`Game.1005`): with the scene-skip setting on, the original does nothing at all (no zoom, flash or fade); with it off, the zoom and white flash of Apeiria. The setting is not offered here and reads as off unless a script writes `/Config/SceneSkip`, which the scripts do not.
- Wipes instead of fade-ins: attack kinds 1 to 5, 13 and 15; spells sub 4 and 9; `Game.1001` subs 1, 3, 14, 15 and 20. Each uses its own rule picture, as the executable does.
- The sprite wipes (attacks, spells, `Game.1001`) have a soft edge 255 rule levels wide: a pixel starts to show when the wipe reaches its rule value and is fully shown 255 levels later. Measured on the original's frames; the runtime's other wipes keep an edge of 32.
- αMAIL (`Game.1012`): sub 1 is the same code as in Apeiria. It starts one wipe of the panel's sprite through rule 46 over 64 frames (flags 0x2000102, frames 0x40, rule 0x2e) and waits for it, or shows the panel at once when the engine is skipping; the 50 light specks are the same (0x47ba10, 0x47bc10, 0x47bd20, 0x47bf10: 50 specks, 8 frames each, none started in the last 8 frames, restart band y 20 to 479). The wipe step (0x4c3880, set up at 0x4c3190) is the one every rule wipe of a sprite goes through: a pixel of rule value r has opacity `min(its own opacity, level - r)`, so the soft edge is 255 levels wide. The set-up stores one step, `512 / (frames - 1)` (a float; the integer period 0x200 / (frames - 1) is kept at +0xac), as the first level, the draw uses the level (truncated) and then adds the step, so frame n draws with `(n + 1)` steps and the panel is whole on the 62nd frame (63 steps), before the animation's 64 end. The glass (translucent) therefore reaches its own opacity before the opaque list does. The runtime does this (`Canvas::blend_capped`, level `wipe_level` in `mail.rs`), and the same step now serves the other sprite wipes; Apeiria's is the same code (checked in `apeiria.exe`, see the [Apeiria page](apeiria.md)). The rule picture is laid over the panel from its own top left corner (checked by fitting the level to the frames of the original: the level read off pixels of rule value r agrees to within 1 across the panel). Before this was found the runtime started the level at 0 (one step late).
- Attack kind 18 (`Game.998`): a pillar of fire (`attack_17_00.akb`) drops in from above, then wipes out. The script waits until it is gone.
- Attack kind 14 takes an argument: 0 shows all ten pictures, 1 the first five, 2 the last five.
- `Game.998` with a sub above 4 does nothing (two calls in the scripts).
- `Game.13` sub 12 (alpha, time, size, name, slot): a standing character at an opacity, but only a new character takes it. A slot that already holds the same character gets the new picture and size and keeps the opacity it has; the alpha is not used then (0x44b4e0). Two names are the same character when the executable gives them the same number (0x44e0a0): the name must start with `ch`, then `01` to `07` give 0 to 6, `09` gives 7 and `10` gives 8; anything else (`ch00`, `ch08`, `ch11` and up, names that do not start with `ch`, names shorter than four characters) gives -1. Only the first four characters are read, so `ch01_a` and `ch01_b` are one character, and two names that both give -1 are the same character too. The runtime uses the same number. The scripts that reach this call in a normal play-through use only `ch09` and `ch10` as the body name.
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

### Sound

`HEffect` (class 17) takes the same sub commands as `Effect` (class 16): both go through the same sound-object code (0x4adc70, one jump table at 0x4add34 for all three of `Effect`, `HEffect` and `Voice`). They differ in their own table of methods and in the configuration they read. `Effect`'s object is built at 0x49fca3 with the methods at 0x52ea14 and its channel list made by 0x4ae700; `HEffect`'s is built at 0x49fcdc with the methods at 0x52b758 and its channel list made by 0x4ae9c0. `HEffect` is a second set of effect channels, not the same channels: a sound started on `HEffect` channel 0 does not replace one on `Effect` channel 0. It reads `/Config/HEffect/Error`, `/Config/HEffect/Enable` and `/Config/HEffect/Volume` (0x53074c, 0x530764, 0x53077c). The volume is a fixed configuration value, not the effect volume scaled: 0x4950f0 reads `/Config/HEffect/Volume` (default 90) and `/Config/Master/Volume` (default 100) and returns the DirectSound attenuation `-5000 + 5000 * (HEffect/100) * (Master/100)` in 1/100 dB (the constants are 100.0 at 0x534d58 and -5000.0 at 0x534e1c), or -10000 (silent) when `/Config/HEffect/Enable` or `/Config/Sound/Enable` is 0. The same formula sets all four buses: Music at 0x494fb0 (`/Config/Music/Volume`, default 70, `/Config/Music/Enable`), Effect at 0x495050 (`/Config/Effect/Volume`, default 90, `/Config/Effect/Enable`), HEffect at 0x4950f0 and Voice at 0x495190 (`/Config/Voice/Volume`, default 100, `/Config/Voice/Enable`, and a further check by 0x495270 that gives -10000 when it fails). Each has its own Enable and also `/Config/Sound/Enable` (0x530690, default 1). The runtime follows this for `casablanca.exe` and `apeiria.exe`: each bus's gain is `10^(att/2000)` (att in 1/100 dB), so volume 0 is -50 dB and the HEffect default 90 is about 0.562; a disabled bus is silent. Other games keep the linear mapping. I did not find a movie volume with this formula. `HEffect`'s method table gets it through the stub at 0x4aec50 (`Effect`'s through 0x4ae990). The scripts call only `HEffect.0`, `.1` and `.3` (channels, play, fade out), so none sets a volume, and no screen of the game sets the configuration values: the volume stays at 90 and the sound is on. The runtime has a bus of its own for `HEffect` (volume 0.9, not changed by the effect volume setting), and stopping all sound stops it too.

### Staff roll and ending pictures

The staff roll is `Game.990` (case at 0x48d8f4, handler at 0x492160); the scripts call it with sub 0 and no arguments, after the last chapter. It is a panel of its own, not scripted pictures: the handler builds a panel object (vtable 0x530100, constructor 0x480a80), loads its pictures at once (0x480c50: `ending.akb`, `staff.akb`, `staff00.akb`, `staff05.akb`, `staff03.akb`, `bcg99.akb`), runs a loop (0x4a8780) while a thread (0x481040) calls the panel's update (0x4810f0) every tick, and takes the pictures away when it ends (0x480e20). The update measures the time from the start and stops the panel at 105,000 ms (0x19a28). Over the picture that was on screen:

| Time (ms) | What |
| --- | --- |
| 700 to 5,033 | `ending.akb` fades in and stays on until the end |
| 5,033 to 6,033, out 8,033 to 9,000 | `staff00.akb` fades in, stays, fades out |
| 10,366 to 11,333, out 13,366 to 14,333 | `staff05.akb` fades in, stays, fades out |
| 16,033 to 97,866 | `staff.akb` (1280 x 2640) rises from y = 720 to y = -2640 in a straight line |
| 97,700 to 99,666, out 102,700 to 105,000 | `staff03.akb` fades in, stays, fades out |
| 102,700 to 105,000 | `bcg99.akb` fades in over the picture |

The timings and the shape are those of the code, read from the update (0x4810f0) and its two helpers. The roll is stepped by the clock, not by frames: the thread (0x481040) calls the update again and again, with a short sleep between calls, and every value is computed from the milliseconds since the start (0x480f10 stores the start). All of it is linear in time:

- Pages (0x481190, table at 0x561518, four times each: in start, in end, out start, out end): the fade-in alpha is `(t - in start) * 255 / (in end - in start)` (integer); the fade-out alpha is `255 + (out start - t) * 255 / (out end - out start)` (float, cut to an integer, never below 0). Nothing is set between the end of the fade-in and the start of the fade-out, so the page stays at the last alpha it was given. When the fade-out reaches 0 the next page is loaded. The three pages are `staff00.akb`, `staff05.akb` and `staff03.akb`, with the times of the table above (the last row's fade-out is 102,700 to 105,000).
- Ending picture and `bcg99.akb` (0x4812f0, table at 0x561468, two times each: 700 to 5,033 and 102,700 to 105,000): alpha is `(t - start) * 255 / (end - start)`, drawn over a copy of what was on that layer when the step began; the next step begins when the alpha reaches 255 or the time is past its end.
- Strip (0x4810f0): while `16,033 <= t <= 97,866` (0x561510, 0x561514) its y is `(16,033 - t) * 3360 / (97,866 - 16,033) + 720`, with 3360 = 720 + 2640, so it goes from 720 at the start to -2640 at the end in a straight line and nothing is drawn after that. It has no alpha.
- The panel stops itself when the time reaches 105,000 (0x19a28, at 0x481114).

The runtime steps in 16 ms frames (`TICK_MS`) and rounds each of these times to the nearest frame on its own, so the lengths do not drift; a frame of the runtime can therefore differ from the original's value at the same moment by up to one frame's worth of the slope (about 4 alpha levels in the 1,000 ms fades, 0.66 pixel of the strip). The start and end frames are `round(t / 16)`: 44 and 315 (ending picture), 315 to 377 and 502 to 563 (`staff00.akb`), 648 to 708 and 835 to 896 (`staff05.akb`), 1,002 to 6,117 (strip), 6,106 to 6,229 and 6,419 to 6,563 (`staff03.akb`), 6,419 to 6,563 (`bcg99.akb`), end 6,563.

The roll can be cut short by Escape, Delete, keypad 0 and the secondary mouse button (the panel loop 0x4a8780 sends the input ids 1, 0xd3, 0x52 and 0x201 to the vtable slot at 0x530180, 0x480ff0), but only while bit 11 of global 55 is set (0x480ff0 reads it from the script variable record at offset 0x6e4, record 55 of 32 bytes; the scripts set bit 11 at the start of the game and clear it at the title). With the bit clear, the roll cannot be cut short. The primary click (id 0x200), Enter, Space and the skip mode do nothing: the click and Enter only go to the panel's buttons, and it has none, and the loop has no test of the skip mode. When it is cut, the handler sets the panel's result to 1 (fields at 0x8 and 0xc: type 0 and value 1) and stops the loop; when the time runs out the result stays at its start value, 0. The panel is torn down the same way (0x480e80, 0x480e20) and nothing else is changed: no flag, no sound, no variable. `Game.990` pushes that result (0x492160 copies it with 0x4722f0 and pushes it with 0x4589c0): 0 when the roll ran to its end (the trace of a played-through roll shows 0 and then `Screen.0(16,0)` for the fade-in), 1 when it was cut short. That the script then draws the last screen itself (with 1 it records `ending.akb` and `staff03.akb` with `Game.12`) is inferred from the script, not from the code. The runtime does all of this in `Game.990` (`games/apeiria/staff.rs`, `Session::roll_cancel`): it puts the pictures on the effect stage, waits 105,000 ms with the screen live, lets Escape, Delete, keypad 0 or the secondary click end the wait only when bit 11 is set (a primary click and the skip mode do not), and pushes the answer when the wait is over. The music is not touched; the scripts start it before the call.

## Not done yet


- The αMAIL panel opening (`fd0301.mes`, its first opening) is compared frame by frame against the original under Wine. The two glow differences seen earlier (a softer frame at 40 %, a stronger corner glow when open) came from aligning the frames wrongly: the original's wipe is still running at the frame taken for 100 % (its level at the top-left disc is 446 of 512 there; it is whole about 10 captured frames later), and its captured frames are 1.35 per wipe step. Aligned by wipe level (read off the original's pixels: 8 levels a step, as the code says), the 640x360 gray MAEs against the runtime are 0.055 (0 %), 0.045, 0.045, 0.045, 0.045 at 25, 38, 51 and 63 steps, which is the 0.05 that the frames differ by before the animation starts. The original's wipe does not reach any visible pixel before level about 115 (the smallest rule value over the panel), so the first 14 steps show nothing.

## Where things are in the executable

Addresses in `casablanca.exe` of the build above (image base 0x400000). The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` command dispatch | 0x48d7d0 |
| `Game.13` sub 12: handler, put up a character, assign into the slot (skips the alpha for the same character), character number from the name | 0x48e2e0 (case at 0x48e7fe), 0x44c7e0, 0x44b4e0, 0x44e0a0 |
| Attacks: dispatch by kind; kind 18 builder | 0x435d50; 0x43a0c0 |
| Spells (`Game.1002`) builders for subs 0, 1, 3, 9 | 0x48aa90, 0x48b100, 0x48b9e0, 0x48c4f0 |
| Zoom flash (`Game.1005`) | 0x4912e0 |
| Conclusions list (`Game.992`) | 0x455170 |
| Deduction board (`Game.994`): handler, open (sub 2), close (sub 3), new picture (sub 4), marker (sub 5) | 0x48f6a0, 0x4557d0, 0x455a50, 0x455ce0, 0x455fa0 |
| `Game.991`: handler, wipe in (sub 3) | 0x48ee70, 0x486740 |
| Message window line set-up; replay of the backlog record; character count while measuring | 0x466120; 0x4b9410; 0x4b9560 |
| Notes and explanation screen (`Game.996`) | 0x48f910 |
| Screen effects (`Game.1001`) subs 1, 3, 14, 15, 20 | 0x482b10, 0x4831b0, 0x4852d0, 0x4854e0, 0x485f80 |
| Ending picture preload (`ending.akb`, `staff*.akb`, `bcg99.akb`) | 0x480c50 |
| Staff roll (`Game.990`): case in the dispatch, handler | 0x48d8f4, 0x492160 |
| Staff roll panel: constructor, vtable, start, stop and tear-down | 0x480a80, 0x530100, 0x480f10, 0x480e80, 0x480e20 |
| Staff roll: timer thread, update (105,000 ms limit at 0x481114), pages for the time, ending picture for the time, cancel test (bit 11 of global 55) | 0x481040, 0x4810f0, 0x481190, 0x4812f0, 0x480ff0 |
| Staff roll: page times, ending picture times, strip times | 0x561518, 0x561468, 0x561510 |
| Panel loop: input ids to cancel (1, 0x52, 0xd3, 0x201) | 0x4a8780 (slots 0x24, 0x28, 0x64 of the vtable at 0x530100) |
| Panel result record: copy, push | 0x4722f0, 0x4589c0 |
| Configuration: get by path (missing node gives the default), set by path (faults on an empty tree at 0x4cb1be), load of the file, load and save of the configuration object | 0x4ca8e0, 0x4caea0, 0x4ca3d0, 0x496800, 0x496c30 |
| Panel loop (shared with the extras) | 0x4a8780 |
| αMAIL (`Game.1012`): handler (jump table 0x492df8), subs 0, 1, 2, 3, 4 | 0x492bc0, 0x479d20, 0x479720, 0x4799d0, 0x47a210, 0x47a560 |
| αMAIL light specks: set up, step, first specks, start again | 0x47ba10, 0x47bc10, 0x47bd20, 0x47bf10 |
| Sprite wipe: start (flag 0x100), set up of the steps, step of the rule mask (vtable 0x5332d8) | 0x4c4020, 0x4c3190, 0x4c3880 |
| `HEffect` (class 17): build at 0x49fcdc, methods, channel list; `Effect`: build, methods, channel list | 0x49fcdc, 0x52b758, 0x4ae9c0; 0x49fca3, 0x52ea14, 0x4ae700 |
| Sound object: sub command switch (jump table 0x4add34) | 0x4adc70 |
| Effect volume; `HEffect` volume | 0x495050; 0x4950f0 |
| Character number from a name | 0x44e0a0 |
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
