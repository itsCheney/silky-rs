# きまぐれテンプテーション2

English | [简体中文](kimaten2.zh-CN.md)

Japanese build 2024 with patch 1.02. The runtime knows the game by `kimaten2.exe` in its folder; the build checked is the one whose `kimaten2.exe` has SHA-256 `5cbd99107b94befe4935bbb7b8fdd7015d715a795945b434788657dee0a50d29`. Its own `Game` commands are listed in [game-commands.md](../game-commands.md#きまぐれテンプテーション2). Program version 1.0.0.0, linked 2024-10-15; its engine profile is [engine-2024](../builds/engine-2024.md).

All 64 scripts and 712 images decode. Headless, `silky-run trace` boots to the title menu and plays the story text with speaker names, in Japanese. Each game adds its own commands to the engine (native class 29); for this one the commands that return a value were read from its executable.

## Interface

The player and `silky-run snap` open it. The title menu, the message window with the speaker's name plate, the window buttons, the side menu, the selections, the action menu and the map (with the second map's panels) are drawn from the game's sheets. The window, action menu and map layouts are read from the executable, and the text is placed as on screenshots of the game, two-line messages included. The backlog, the save and load screen, the settings pages and the yes/no dialog have the game's own look, with its help lines (`.HLG` tables) and its nine voice groups on the sound page. Saving at an action menu or the map saves the message before it. The bar's voice loop button plays the message's voice again each time it ends, and its skip-all button skips unread text too. Its jump button skips to the next unread text or selection; as in the original, it is grey while the message on screen has not been read before. The buttons and side menu entries show the game's tooltips (`MesButtonHelp.hlg`) in its balloon, placed as the executable does. Auto mode waits for the voice and keeps a message for the time the executable gives, and wheel down on a message goes on like a click. The bar's pin (a setting saved with the others, on by default) keeps the bar shown; with the pin off the bar slides down out of the screen while the pointer is above it and comes back when the pointer goes near the bottom. The costume button is grey until byte 700 is set (the scripts set it once the costume screen opens).

Compared with the original running under Wine: the message text has a one-pixel black edge around bold glyphs, the click-wait icon is the three frames at (1262, 0) of the window sheet, shown at (1062, 609), and the window is 40 % transparent by default, as the program starts it. Event pictures that the scripts put up with `Game.12` sub 10 show (they were left out before). Screenshots of a story line and of an event picture differ in under 0.7 % of their pixels (glyph shapes, the fonts differ).

## Costume, notebook and collection

These screens are drawn from their sheets and read the script's byte variables the way the executable does:

- Notebook entry `i` is open when byte `900 + i` is set, collection item `i` when byte `801 + i` is set.
- The costume is bytes 701 (form) and 708 (outfit), written back when the screen closes. Forms C and D open with bytes 711 and 712. The outfit button shows only once byte 3007 is set.
- The extras page's アンネの部屋 opens the same screen through `Game.51` with a 次へ button added. 次へ answers 2 and goes on to H mode. もどる answers 1 if the costume changed and 0 if not.
- From the title menu the notebook shows the copies the scripts keep at `1000 +`. Bytes from 1000 on are kept outside a game in `system.json`, so they survive loads and new games.
- Some notebook entries grow during the story: byte `940 + i` says how many parts are open, the rest of the text is cut off, and a "New" mark stands before the newest part (cut and mark positions from the executable). From the title menu the mark goes away once the entry was opened there (byte 2020 + i).
- The last collection item, a voice recording, is a play/pause button over its picture that plays five voice files in turn. The executable names them `kt_*.ogg`; they are not in the copy of the game tested, so they are reported as missing.

## Title menu and extras

The title buttons drop in one after another and bounce into place, with the timing read from the executable. Back to title from the game menu returns to the menu itself, without the logos. On the extras page, the music mode needs byte 3004, the notebook byte 3006, and the Anne Viewer and Anne's Room byte 3007 (set by the endings); until then they are grey. A missing sound file is reported once and skipped, as the game does.

The CG, scene and music modes (`Game.30` to `Game.32`) are screens of the engine; their layouts and tables were read from the executable. All three are drawn from `mode_p.akb` and keep what was seen in byte variables from 1000 on, which `system.json` keeps across games:

- CG mode: 29 entries of 268 pictures, 12 entries a page, each with a seen/total count, and the completion in per cent. Picture `i` is seen when bit `i % 8` of byte `3300 + i / 8` is set. The engine sets it when the picture on screen, with its layers, matches a row of its table; the runtime does the same. An entry opens a grid of its pictures; a click shows one, the next click the next one seen.
- Scene mode: six scenes, seen when bytes 3101 to 3106 are set. A click plays the scene through `scmode.mes`.
- Music mode: 29 tracks (`kima2_01.wav` to `kima2_29.wav`), heard when bytes 3051 to 3079 are set (the scripts set them), seven rows with a scroll bar, and the music volume.
- Each mode's unlock button asks first, then sets every byte of its mode, as the game does.

## E-mote

Anneliese is an E-mote 4 model (`.psb`, header encrypted): two files per costume, `ta`/`tb` + form + outfit (the ones the costume screen chooses between), with `_h0007` holding faces 0 to 7 and `_h0813` faces 8 to 13. [Eluna](https://github.com/xmoezzz/eluna) (vendored in `vendor/eluna_rs`, MPL-2.0, the copy siglus_rs uses) decrypts them and runs their timelines and physics. Its sprites (textured triangles, the six blend modes, stencil masks; DXT5 textures) are drawn by wgpu in the player, straight into the frame at window size, and by a small software rasterizer headless; both give the same picture. The key of this game is built in; for other games the runtime searches the 2^32 keys once (a few seconds to a minute).

Two commands show her, both read from the executable:

- `Game.10` is Anneliese walking along with the protagonist, at the bottom left: sub 6 loads a costume, 7 sets the face and the timeline after the faces, 13 (and 12) the pose, 16 the `a` timelines, 9 and 11 show and hide. Size 0.35, position given as (500, -930).
- `Game.40` holds standing models by slot: sub 0 (and 12) loads a costume into a slot at a size in percent from the script (50 and 100 in the middle, 60 on the right), 1 sets the face, 15 the pose, 13 the pose that follows once the playing one ends (H mode starts this way on the waiting loop), 14 the `a` timelines, 8 moves a model, 3 removes one slot and 4 all, and 2 crossfades to the new picture over the frames those calls gave, as `Game.13` does for standing characters. Sub 16 hands over the mouth track of the voice that just started (`k2*.csv`: UTF-16 numbers, 60 a second, the model's `face_talk` value from 0 to 5); it moves the mouth of every Anneliese on screen while the voice plays. The slot gives x (0 middle, 1 left, 2 right) and the size gives y, so the feet stay on the same line.

How these positions map to the screen was measured on two screenshots of the game (one of each layout); both agree. The animation runs at 60 frames a second on the GPU (15 in software). Turning off E-mote animation in the settings keeps the picture still. Built without the default `emote` feature, the runtime leaves these pictures out.

`Game.10` sub 4, called at the start of every H scene, is the original's scene skip point: with its "skip H scenes" setting on, it ends the scene there. That setting is not offered, so the call does nothing, as in the game with the setting off.

## H mode

`Game.49` is an interactive screen of the engine, read from the executable. The script first loads an `hg030100_*` model pair into slot 0. The player picks an action from the column on the right and clicks a region of the touch map `anne_h.carea`. Each action and region plays the timelines, expression and voice the executable gives for it. Two gauges rise as the player acts; a full gauge plays a closing animation and waits for its voice. The buttons switch costume, an alternate model and background, auto repeat, and gauge locks. The finish button picks a closing line and returns to the script. Over a body region the pointer turns into a picture of the chosen tool (`cursor.akb`, two frames of 300 ms) by the executable's tool and region table. In dream mode the mist (`yume.akb`) scrolls and waves along a sine, and a climax flashes white, both stepping every 16 ms as the engine's sprite animations do.

The one-shot timelines of these models give a length of 3 ticks while their frames run to 50 or 55; they are played to their last frame, since the game waits for them to end.

The H scene models clip the whole picture to a frame layer, `トリミング枠`, whose opacity is 0. A stencil source counts by its shape, whatever its opacity, so the frame still cuts the picture to the screen. Their effect layers and shadows use the E-mote driver's blends that read the picture below (`bm` 0x100 to 0x10b: linear dodge, linear burn, overlay, hard light, soft light, color dodge, color burn, lighten, darken, difference, exclusion, luminance), read from the driver's shaders. Both renderers draw them from a copy of the picture below.

The censor mosaic is the driver's `bm` 0x10c: each cell takes the picture below at its middle. The frame's blend parameter gives the number of cells across the 2000 x 2000 picture the game draws a model into (80, so cells of 25 pixels), and the cells line up with that picture.

When the game loads an H scene model it plays the model's difference timelines 28, 0 and 13 and then main timeline 0, which brings the model to its plain waiting picture. It places these models 26 pixels higher than a standing model at the same numbers, since it hands their position to the model rather than to the picture it draws.

A layer whose key is empty (type 0) is not drawn itself, but its children are. The H scene models keep the lower body under a group whose keys are all empty.

## Not done yet

- H mode was compared with the game. Its buttons, gauges and the costume buttons at the top left sit where the game puts them, and the model's waiting picture matches the game's within 2 pixels. The mosaic cells sit 4 pixels higher than the game's.

## Where things are in the executable

Addresses in `kimaten2.exe` of the build above, for checking the code against the original. The code itself does not name them.

| What | Address |
| --- | --- |
| `Game.10` sub 4, scene skip point | 0x497410 |
| `Game.10`, Anneliese beside the protagonist (500, -930, size 0.35) | 0x45c170 |
| `Game.40`, standing models | 0x4c6000 |
| `Game.40` sub 8, move a model | 0x45aa00 |
| Standing model place and draw order | 0x4577a0 |
| E-mote: cut to a main timeline | 0x452ac0 |
| E-mote: timelines played on both files | 0x457d70, 0x45a760 |
| E-mote: stop the main timeline, a difference timeline | 0x457fe0, 0x457ec0 |
| E-mote: faces 0 to 13 | 0x458630 |
| E-mote: whether a main timeline plays (one-shots wait for it) | 0x48bda0 |
| Title menu: extras unlock bytes | 0x4be440 |
| Title menu: button jump | 0x4bed40 |
| CG mode: entry table, picture table | 0x575a78, 0x573488 |
| Button bar: button states (costume by byte 700) | 0x498640 |
| Button bar: slide out and in, pointer check | 0x49a6b0, 0x49a800, 0x49df50 |
| Button bar: pin setting (`MenuLock`) read and written | 0x426730, 0x426860 |
| H mode: object, init, frame | 0x542c60, 0x4844b0, 0x484a50 |
| H mode: buttons, actions, held actions, climax | 0x4852f0, 0x486b10, 0x48a090, 0x48ab90 |
| H mode: pick without repeat, voice queue, voice now | 0x48ba00, 0x48b6b0, 0x48b8e0 |
| H mode: breathing, back to the waiting loop | 0x48bbc0, 0x48bb10 |
| H mode: button animation, buttons drawn, gauges | 0x48a940, 0x486600, 0x489740, 0x489870 |
| H mode: voice tables | 0x58bc50..0x58ea44 |
| H mode: pointer picture by tool and region, table | 0x486600, 0x53c0dc |
| Game pointer (`cursor.akb`): set, frame step, place | 0x4a27a0, 0x4a2ab0, 0x4a2b60 |
| H mode: dream mist scroll and wave, white flash | 0x47ce30, 0x47cef0, 0x489b10 |
| E-mote model: queue the next pose (`Game.40` sub 13), play a pose, step the models | 0x45a9a0, 0x452ac0, 0x456fc0 |
| E-mote model: load (timelines played on an H scene model), place an H scene model | 0x45a1b0, 0x4564b0 |
| Costume screen: open, set up, click, draw, close | 0x4795b0, 0x479740, 0x479920, 0x479e20, 0x479700 |
| Sprite animation task (runs once more than its 15 ms have passed) | 0x4186f0 |
| E-mote: `Pass` on both files (driver vtable +0x110), used by the fourth tool | 0x456d20 |

Addresses in the `emotedriver.dll` that comes with it:

| What | Address |
| --- | --- |
| Blend states, from a table of 13 | 0x10061860, 0x11fc3570 |
| Draw a sprite: blend state by `bm` (0x100 to 0x10f read the picture below) | 0x10061b20 |
| Pick the shader of a blend that reads the picture below | 0x10060390 |
| Shader blobs and their sizes | 0x10d32310, 0x1014fcf0 |
| Mosaic cell size from the blend parameter | 0x10060390 |
| Copy of the picture below, snapped to the mosaic cells | 0x10065ef0 |
