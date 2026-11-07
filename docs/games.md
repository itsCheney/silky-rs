# Games

English | [简体中文](games.zh-CN.md)

How far each game runs on silky-rs. The engine features shared by all games are listed in the [README](../README.md#status). `silky-run snap` lists every engine call the runtime does not carry out yet.

## ふゆから、くるる。

International build 1.0.1.0, Japanese, English and Simplified Chinese.

All 56 scripts (7.9 million instructions) decode with every jump landing on an instruction, and all 3,255 images decode. Headless, the VM boots `start.mes` through the logo, caution and title scripts, and plays the story from `fk001.mes` on with its text in all three languages. Every route plays to its ending.

Compared with screenshots of the original (Windows, Simplified Chinese): the message window, name plate, window portrait, the position and pitch of Chinese text, and the size and position of standing characters. Still provisional: English text metrics (no English screenshots to compare with yet), the line pitch of two-line messages, the wrap width, and where selections sit on screen.

Effects read from the executable: rule-mask wipes and screen dimming (sprite animations), snow, the future flash, pulse zoom, slash, white zoom flash, screen quake, character swing, and the pictures that change while a voice line plays (`.vo` files). Approximate: the picture does not shrink during the future flash, queued music starts without its fade-in, and the step time of character hops is a guess.

The `Flag` calls keep the flags from byte 1000 on across games in the system data, as the executable does with its system flag file. A run through every route leaves no engine call undone. The "fixed window" setting is kept but changes nothing, since the window never moves here.

## きまぐれテンプテーション2

Japanese build 2024 with patch 1.02; the fan Simplified Chinese patch (folder `kimaten2_chs`) is read as a language.

All 64 scripts and 712 images decode. Headless, `silky-run trace` boots to the title menu and plays the story text with speaker names, in Japanese and, with the fan patch folder, in Simplified Chinese. Each game adds its own commands to the engine (native class 29); for this one the commands that return a value were read from its executable.

### Interface

The player and `silky-run snap` open it. The title menu, the message window with the speaker's name plate (the fan patch's redrawn plates for its Chinese names too), the window buttons, the side menu, the selections, the action menu and the map (with the second map's panels) are drawn from the game's sheets. The window, action menu and map layouts are read from the executable, and the text is placed as on screenshots of the game, two-line messages included. The backlog, the save and load screen, the settings pages and the yes/no dialog have the game's own look, with its help lines (`.HLG` tables) and its nine voice groups on the sound page. Saving at an action menu or the map saves the message before it. The bar's voice loop button plays the message's voice again each time it ends, and its skip-all button skips unread text too.

### Costume, notebook and collection

These screens are drawn from their sheets and read the script's byte variables the way the executable does:

- Notebook entry `i` is open when byte `900 + i` is set, collection item `i` when byte `801 + i` is set.
- The costume is bytes 701 (form) and 708 (outfit), written back when the screen closes. Forms C and D open with bytes 711 and 712.
- From the title menu the notebook shows the copies the scripts keep at `1000 +`. Bytes from 1000 on are kept outside a game in `system.json`, so they survive loads and new games.
- Some notebook entries grow during the story: byte `940 + i` says how many parts are open, the rest of the text is cut off, and a "New" mark stands before the newest part (cut and mark positions from the executable). From the title menu the mark goes away once the entry was opened there (byte 2020 + i).
- The last collection item, a voice recording, is a play/pause button over its picture that plays five voice files in turn. The executable names them `kt_*.ogg`; they are not in the copy of the game tested, so they are reported as missing.

### Title menu and extras

The title buttons drop in one after another and bounce into place, with the timing read from the executable. Back to title from the game menu returns to the menu itself, without the logos. On the extras page, the music mode needs byte 3004, the notebook byte 3006, and the Anne Viewer and Anne's Room byte 3007 (set by the endings); until then they are grey. A missing sound file is reported once and skipped, as the game does.

The CG, scene and music modes (`Game.30` to `Game.32`) are screens of the engine; their layouts and tables were read from the executable. All three are drawn from `mode_p.akb` and keep what was seen in byte variables from 1000 on, which `system.json` keeps across games:

- CG mode: 29 entries of 268 pictures, 12 entries a page, each with a seen/total count, and the completion in per cent. Picture `i` is seen when bit `i % 8` of byte `3300 + i / 8` is set. The engine sets it when the picture on screen, with its layers, matches a row of its table; the runtime does the same. An entry opens a grid of its pictures; a click shows one, the next click the next one seen.
- Scene mode: six scenes, seen when bytes 3101 to 3106 are set. A click plays the scene through `scmode.mes`.
- Music mode: 29 tracks (`kima2_01.wav` to `kima2_29.wav`), heard when bytes 3051 to 3079 are set (the scripts set them), seven rows with a scroll bar, and the music volume.
- Each mode's unlock button asks first, then sets every byte of its mode, as the game does.

### E-mote

Anneliese is an E-mote 4 model (`.psb`, header encrypted): two files per costume, `ta`/`tb` + form + outfit (the ones the costume screen chooses between), with `_h0007` holding faces 0 to 7 and `_h0813` faces 8 to 13. [Eluna](https://github.com/xmoezzz/eluna) (vendored in `vendor/eluna_rs`, MPL-2.0, the copy siglus_rs uses) decrypts them and runs their timelines and physics. Its sprites (textured triangles, the six blend modes, stencil masks; DXT5 textures) are drawn by wgpu in the player and by a small software rasterizer headless; both give the same picture. The key of this game is built in; for other games the runtime searches the 2^32 keys once (a few seconds to a minute).

Two commands show her, both read from the executable:

- `Game.10` is Anneliese walking along with the protagonist, at the bottom left: sub 6 loads a costume, 7 sets the face and the timeline after the faces, 13 (and 12) the pose, 16 the `a` timelines, 9 and 11 show and hide. Size 0.35, position given as (500, -930).
- `Game.40` holds standing models by slot: sub 0 (and 12) loads a costume into a slot at a size in percent from the script (50 and 100 in the middle, 60 on the right), 1 sets the face, 15 the pose, 14 the `a` timelines, 8 moves a model, 3 removes one slot and 4 all, and 2 crossfades to the new picture over the frames those calls gave, as `Game.13` does for standing characters. Sub 16 hands over the mouth track of the voice that just started (`k2*.csv`: UTF-16 numbers, 60 a second, the model's `face_talk` value from 0 to 5); it moves the mouth of every Anneliese on screen while the voice plays. The slot gives x (0 middle, 1 left, 2 right) and the size gives y, so the feet stay on the same line.

How these positions map to the screen was measured on two screenshots of the game (one of each layout); both agree. The animation runs at 30 frames a second on the GPU (15 in software). Turning off E-mote animation in the settings keeps the picture still. Built without the default `emote` feature, the runtime leaves these pictures out.

`Game.10` sub 4, called at the start of every H scene, is the original's scene skip point: with its "skip H scenes" setting on, it ends the scene there. That setting is not offered, so the call does nothing, as in the game with the setting off.

### H mode

`Game.49` is an interactive screen of the engine, read from the executable. The script first loads an `hg030100_*` model pair into slot 0. The player picks an action from the column on the right and clicks a region of the touch map `anne_h.carea`. Each action and region plays the timelines, expression and voice the executable gives for it. Two gauges rise as the player acts; a full gauge plays a closing animation and waits for its voice. The buttons switch costume, an alternate model and background, auto repeat, and gauge locks. The finish button picks a closing line and returns to the script.

The one-shot timelines of these models give a length of 3 ticks while their frames run to 50 or 55; they are played to their last frame, since the game waits for them to end.

### Not done yet

- The pin of the button bar.
- In H mode: the mouse cursor per action and region, and one model call whose meaning is not read yet. Two effect timings are guesses. The screen has not been compared with the game yet.
