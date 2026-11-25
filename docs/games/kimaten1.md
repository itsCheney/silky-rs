# きまぐれテンプテーション

English | [简体中文](kimaten1.zh-CN.md)

Japanese build with the official update 1.04. The runtime knows the game by `kimaten.exe` in its folder; the build described here is 2,039,808 bytes with SHA-256 `36148a0286bd9d31ceaec4323f4be987319bd759e68b5961c59c992b07b54700`. Program version 1.0.4.0, linked 2019-12-16. Engine profile: [engine-2017](../builds/engine-2017.md). Its own `Game` commands: [game-commands.md](../game-commands.md#きまぐれテンプテーション).

The game adds its own commands to the engine (native class 29); which of them return a value comes from the executable. The runtime reads the update's `update.arc` over the other archives, newest first. Its E-mote models are the kind of the sequel ([kimaten2.md](kimaten2.md)), and the calls that take the same arguments go through the same code. The screens are the game's own: the sheets, layouts and tables differ from the sequel's.

## Interface

The player and `silky-run snap` open it. Window, button bar, action menu, investigation, map and notebook are drawn from the game's sheets, with layouts and tables from the executable. The backlog, save and load screens and the settings pages are the engine's, as in [景の海のアペイリア](apeiria.md) (the same engine build).

### Title

- Six buttons along the bottom edge, and a second page of six for the extras. They drop in one after another, 100 ms apart, and bounce into place; a page that is left falls out of sight the same way.
- The heroine's face on the title changes every five seconds, picking among four of the five faces on the sheet, and fades over 16 frames.
- Sparkles drift over the title: 20 sprites of `Prism.akb`, moved by the effect's own random numbers. The seed is fixed, so a run repeats. Their layer over the buttons is a guess. The exe has no separate logo slide.
- Extras page: CG, scene and music need bytes 0xbba, 0xbbb and 0xbbc, Anne Viewer 0xbbd and the notebook 0xbbe (the story sets them); until then they are grey. 戻る always works.
- The notebook opened from the title shows the copy the scripts keep 1000 bytes higher, with a button that opens every entry (`archive00_p.akb`). Bytes from 1000 on are kept outside a game in `system.json`.

### Message window and button bar

- A band along the bottom, a colour fill with a diamond texture over it; its top is the saved window position, 505 by default. The text sits in a box from (280, 66) to (1120, 202) under the top, in 28 pixel cells and 36 pixel lines; the speaker name is one line above it.
- The bar is thirteen buttons: save, load, quick save, quick load, voice loop, jump to the next selection, backlog, auto, skip read text, skip all text, settings, notebook, and one that folds the others away. A cross hides the window.
- The fold button slides the bar down to y = 720 and back up to 682 over 12 passes at 60 a second. Input is not blocked meanwhile (the exe may block it). The rate is taken from the same animation class in another build.
- The notebook button blinks when the notebook is unlocked (`Game.10` sub 15): cell 11 of `message_p.akb`, five rounds of 400 ms.
- Greyed buttons: history while the log is empty, load while there are no saves (both approximations of the exe's own flags), notebook by byte 899.
- The click-wait bat is four frames of 200 ms at (1126, window top + 90).
- Buttons show the game's tooltips in the information line.
- `Game.14` marks an auto save point.

### Action menu

`Game.42`: up to four large buttons (look around, talk, move, present evidence), packed into the first positions of `actselect.area` in that order; the script turns a kind on with bytes 0 to 3. A skip button at the lower right shows once the skip tutorial is unlocked (byte 0xbbf) and answers 100. The system buttons along the bottom (the same eight on every investigation screen) open the engine's own screens. Pointing at one of the four large buttons only shows the heart marker; the information line stays empty (the exe's hover code leaves the help text to the skip and system buttons).

The skip button shows the tutorial the first time (byte 0xbc0), then asks to confirm (`dialog.area`, `dialog_p.akb`, row 5); Yes answers 100. The tutorial (`TUTORIAL.AREA`, `MOVESKIP_TUTORIAL.AKB`, `OKbotton.akb`) plays `ses_009` and fades in and out over 8 frames. Its OK button makes no sound. In the confirm dialog Yes plays the OK sound and No plays `ses_002`. `tutorial01.akb` is a different tutorial (dragging keywords) that the skip button does not use.

### Investigation

`Game.41` lays a hotspot picture (`.carea`, one byte a pixel) over the background. The hotspot under the pointer is tinted pale yellow and the help line reads 「ここを調べる」. A press answers the hotspot's number; the back button, right click and Escape answer 100.

### Evidence choice

`Game.46` shows the pieces of evidence held (bytes 224, 231, 212, 213 and 138 switch the five on), packed into the first positions of `itemselect.area`. A press answers the piece's number from 1. Only the system buttons leave the screen. `Game.47` shows one or two evidence cards over the picture: they fade in over 12 frames, stay 128 frames and fade out over 12, with the script waiting.

### Option list

`Game.45` turns the options the script registered into a list of bars, four to a page with up and down bars to turn pages, for the talk and look menus. A press answers the number the script gave the option, plus one; the back button and right click answer 100. The labels are the strings the option bodies leave; a conditional option list comes as several runs before the selection and keeps the script's numbers. Compared with the original's first list in 02_B-2 (one bar, one option): the bar's position, size, emblem, texture and both colours (near black at rest, magenta under the pointer) match within one level of 255; labels are centred by 24 pixel cells but drawn with the message window's 28 pixel cells, and sit one pixel lower than the message window's selection text. The remaining text difference is the glyph shapes of the font used. Escape and right click close the list with 100 as in the original; `Game.45` takes no argument. In the original capture the history and load buttons were grey in this list; the headless frames leave them lit because no context is given.

Option n is on while byte 7500 + n of the raw area is 1; the script's option wrapper sets bytes 7500 to 7563 to 1 (with st.r8) before it opens the list, and options that are not available clear theirs. Any other value draws it in grey text (128, 128, 128) with no hover, no sound and no result. A pressed bar shows the same text colour and baseline (28) as at rest, as in the exe.

### Map

`Game.43`: the six rooms (101 to 103 on the first floor, 201 to 203 on the second) as cards, an information line naming each, and the menu buttons. A room whose byte (0 to 5) is 0 is locked. Pressing a room answers 1 to 6; the back button, shown when the call's argument is 1, answers 100. A room not yet looked at (byte 810 plus its number) shows a plain card on hover. The menu's notebook button opens the notebook over the map. The pictures and the information line fade in and out over 12 frames; leaving stops the voice channel. Right click and Escape answer 100 at once when the argument is 1; with 0 they follow the right-click setting.

### Notebook

The 捜査手帳 lists 36 entries on three pages of 14 rows. An entry opens its page (`archive_w01.akb` and on) under a title picture. A tall page scrolls with the wheel (10 pixels a notch, also in the map's embedded notebook) or by dragging the scroll bar. Each entry has a byte (900 and on): 0 locked, 1 read, 2 or 3 new; opening a new entry reads it. One entry holds a recording, played from a button over its picture: five lines in turn (`kt_b0155`, `kt_c0201`, `kt_b0156`, `kt_c0202`, `kt_c0203`). The background and page fade in and out over 12 frames. No `Game` command opens the notebook: the script announces it with a notice, and the player opens it from the bar, the map's menu or the title.

The bottom row has six menu buttons (settings, sound, history, save, load, notebook). The notebook's own button is the current one: lit (row 2 of its picture) once byte 899 is set, grey before, and pressing it does nothing. History and load are grey as on the bar. Opened from the window's bar, the first five buttons close the notebook and the window opens what they stand for (the sound button opens the settings, as on the action menu's bar). The wheel scrolls only a page that is tall enough, wherever the pointer is; with nothing open it does nothing (and does not reach the message behind).

The keys put the pointer on another element (the help line follows the hover; nothing else happens, no sound). The focus is the element under the pointer; from none, every key first goes to the first row. Ids are the area file's: rows 0 to 13, up arrow 0x100, down arrow 0x101, thumb track 0x102, back 0x200, page back 0x300, page forward 0x301, recording 0x400, unlock-all 0x500 (title copy only), menu buttons 0xf00 to 0xf05.

| Key | From | To |
| --- | --- | --- |
| Up | row n | row n-1; row 0: page back |
| Up | down arrow, back, menu button | up arrow, down arrow, page forward |
| Up | up arrow | unlock-all (title copy), else nothing |
| Down | row n | row n+1; row 13: page forward |
| Down | up arrow, down arrow | down arrow, back |
| Down | page back, page forward, unlock-all | row 0, first menu button, up arrow |
| Left | up arrow, down arrow, track | page back, page forward, row 0 |
| Left | back, unlock-all | notebook menu button, page back |
| Left | menu button n | menu button n-1 (none from the first) |
| Right | any row | up arrow |
| Right | page back, page forward | up arrow, down arrow |
| Right | menu button n | menu button n+1; the last: back |

The keys always move the pointer, whether or not the target can be pressed. A key with no entry in the table does nothing (from none it still goes to row 0). Enter presses what the pointer is on, as a click does.

The help line is written in 16 pixel cells at (668, 695), white glyphs with a black edge, smaller than the cell.

### Notices

`Game.44` queues a string (sub 0), shows the queue (sub 1), takes the strips down (sub 2), clears the queue (sub 3) or answers whether strips are still up (sub 4). A notice is a small pink strip, such as 「捜査手帳」が解放されました, that slides in from the right edge, stays four seconds and slides out. The strips of one batch are as wide as the longest; the older ones step down to make room.

## E-mote

The models are the sequel's: `_h0007` with faces 0 to 7 and `_h0813` with faces 8 to 13, two files per costume, opened by the same key. The game hands the driver its positions the other way round to the sequel: a positive x moves right, a positive y down.

- `Game.10`: the companion beside the message window, at size 0.35 and (-542, 400), fading in over 16 frames. Subs 6, 7, 8, 9, 11, 12, 13, 16 and 17 work as in the sequel. Sub 10 wipes the window's text, 14 takes the whole window away and 15 holds the script for two seconds and blinks the bar's notebook button (the notebook's unlock).
- `Game.40`: standing models by slot, with x by slot (0, -400, 400) and y from the size, so the feet stay on one line. The sequel's subs apply, plus 17 (the extra timeline alone), 18 (pause the driver), 21 (stop the mouth) and 31 (clear the cache of mouth tracks); sub 16 hands over the mouth track of the voice that just started. Sub 100 flashes a slot white while the old model fades out and the new one fades in.
- `Game.48`: the one-file CG models of the update, shown over the background (`hg010200_0` is `hg010200_0.psb`). 0 loads one (fading in over 32 frames), 1 sets the face, 15 the main timeline, 2 waits for the fade in and 3 fades it out and removes it. Their place was found by laying the model's first picture over the still picture of the same CG.
- Voice-timed models: `Game.1007(file)` reads a `.vo` file, a list of changes made a set time after a voice line starts: a standing model put up or taken away in a slot, or the companion changed, fading over 12 frames (16 for the companion). Records of kind 2 (pictures laid in a layer) are used by no file of the game.

The y origin of a model (350 on screen) was fitted on one screenshot of the game, not read from the executable.

## Effects

- `Game.1000`: a line of text, a picture strip, wiped in through a rule picture (hold, frames, rule, height, width, y, x); the script waits for the wipe and then its hold. Enter or a click does not cut it short.
- `Game.999`: a picture laid over the screen. 0 fades in the picture on screen over eight frames, 1 fades it out, 2 shows it at once, 3 puts it away.
- `Game.993`: screen shake (0 resets, 2 sets the mode, 3 starts, 1 stops).
- `Game.961`: a long voice on a channel of its own that the next message does not cut; sub 1 stops it.
- `Game.1001 (type, action)`: a looping screen effect. Action 0 starts it, 1 stops it at once, 2 stops it gracefully (each sprite finishes its lap). Types 0, 1 and 4 are soot specks drifting up (1 adds a dark haze wiped in through rule 61, 4 uses the other tile). Type 2 is a darkening pulse; 3 adds two waving copies of the screen. Type 5 is a red room with three bobbing lights. Type 6 makes the lights swell, plays a sound and waits. Type 7 is a turned and doubled screen with streaks. Type 8 is an eight-picture film. The effect loops until stopped and the script does not wait, except for type 6.
- `Game.1002 (arg)`, the sub on top: 1 a text strip over black, revealed in 255 pieces; 2 flashes (arg 0 to 3); 3 a zoom from 1.2 to 1.0; 4 a flash, then a CG fades in and becomes the background; 5 the same with eff09; 6 two pictures in turn. All of them make the script wait.
- Depth and blend of the effects come from the exe: an effect at depth 3 or below draws between the models. Lights, film and the flashes and pictures of 1002 blend additively; the rest use plain alpha.
- A flash on a model (`Game.40` sub 100) sets the E-mote model colour from neutral 0x80 to 0xff. Ours multiplies RGB by byte / 128.
- Models that leave: ghosts keep animating, as in the exe. Standing models that leave (`Game.40` subs 3 and 4) fade and slide 48 pixels (slot 1 left, slot 2 right).
- Scene skip: effect calls are skipped when `/Config/SceneSkip` is 1.
- `Game.989`: acts only when flag 89 is set. No script sets it, so it does nothing.
- `Game.35`: only sub 2 is used by the scripts. It resets an object we do not have, so it does nothing here.
- Nothing to do: `Game.1003`, `1006.11`, `1010`, `1011`, `10008`, `10009`, and the engine calls of the window set up that the game's library makes.

## Extras

CG, scene and music modes (`Game.30` to `Game.32`) are tabs of one screen: a press on a tab changes the mode without going back to the script. Clicks play `ses_001` (OK) or `ses_002` (cancel) by mode. A right click on the CG and scene modes leaves.

- CG mode: 32 entries of 261 pictures on three pages of 12, each with a seen count, and the completion in per cent. Picture `i` is seen when bit `i % 8` of byte `3300 + i / 8` is set; the runtime sets it when the picture on screen matches a row of the table, as the engine does. Registration stops while the flag at +0x8a4 is set (by the OMAKE and SCMODE scripts). An entry opens a grid of its pictures, and a click shows one full screen.
- Scene mode: eight scenes, seen when bytes 3101 to 3108 are set. A press answers the scene's number from 0 and the script plays it.
- Music mode: 24 tracks, heard when bytes 3051 to 3074 are set; eight rows with a scroll bar, and the music volume.
- Each mode's unlock button asks first, then sets every byte of its mode.
- Anne viewer (`Game.33`): the model over `bg004` with twelve outfits (bytes 3010 and on open them), fourteen expression cells, the poses of an outfit, a size slider, a list of 180 motions with a scroll bar, a blush toggle and a button that hides the others. It answers -1 when left. The buttons fade over 16 frames. A model swap fades the old one out over 12 frames and the new one in over 12; the viewer takes no input during the fades. Picking the playing motion plays it again, and some motions stop the main timeline first. The old model's fade curve is the effect stage's.

## Staff roll

`Game.990`, sub 0 after the normal endings and sub 1 after the true one, differ in the strip, the cover colour and the track. A card shows behind a cover for eight seconds; from 13 seconds the strip (1280 x 3080) rises from the bottom edge at an even speed until it has gone by at 107 seconds. The call answers 1 when the roll is over. A press ends it early, but only while bit 11 of global 55 is set, as in the original.

## Compared with the original

Frames of the first action menu and the first map (second chapter) were compared with screenshots of the original at the same pointer positions (menu: look, move; map: rooms 101 and 102; plus the frame right after pressing Move). The backgrounds differ (the reference save loads a different scene), so only the interface pixels were measured: over the pixels the interface draws opaquely the map matches to an RMSE of about 3 to 5 (floors, cards, crown and wings, locked card, title, back button, information line), and the menu buttons, heart marker, pressed state and title match as well. One difference was found and fixed: the menu wrote a help line when a large button was hovered, the original does not. The history button is grey in the reference only because that save has no backlog. A right click on the map did not cancel in the reference run; the exe's handler answers 100 when the back button is on, so that is taken to be a test artefact (the click may have been missed) and was not copied.

The notebook (opened from the action menu's bar, six entries new, the rest locked) was compared with frames of the original in the chrome areas: the list column, the new and locked markers, both page pills and arrows, the title area and the lower bar match to an RMSE of about 1 (top) and 4 (the whole bottom strip, which keeps an edge-blur difference on the bar's frames in the reference that the pixels of our sheet do not reproduce). Fixed from that: the notebook's own button is lit instead of grey, history and load grey as on the bar, the help line's place, size and black edge, and the key focus (the same four keys, Right then Left then Down give the original's help lines and outlined first row). Opening takes the same 12 frame fade.

## Not done yet

- Skip tutorial: the OK button's pressed row and wait for release, and the confirm dialog's 12-frame fades.
- Fold slide: the exe may block input during it. The stepping rate was not checked in this build.
- `Game.10` sub 18 acts on a part of the window that nothing in the story reaches.
- Title sparkles: the layer over the buttons is a guess.
- Map and notebook: input is not blocked during the fades. A flag bit on the message window object at map exit is not identified.
- Anne viewer: the text edge and baselines are estimated. A same-outfit reselect may keep the running pose in the original. The wheel is ignored while the buttons are hidden; the original scrolls.
- Extras: the CG cell sound is not skipped when g[16] is 1. Message bar buttons 0 and 2 do not grey while h[69] is 1.
- Option list and bar: the flags at 0x8a4 and 0x8c4 in the option list, the quick save and load greying in the action menu and the meaning of 0x6a4 are unknown. The greying of history and load is approximate.
- Flash white: the draw side of the model colour is a guess.
- Scene skip does not cut a running wait short.
- Effects (`Game.1001`, `1002`): the exe copies the screen once for type 7 and for `1002` subs 3 and 6; ours is live. Subs 4 and 5 bake the top picture into the background, where the exe copies the composed screen. The black fade and piece depth of sub 1, the companion model's depth, the place of the black layer of types 2 and 3 are guesses. The red room is a sprite, not the background. Random positions cannot match a run of the original. None of these effects were compared with the original under Wine.
- `Mouse.5` is the only call reported as unhandled in a headless run.

## Headless routes

`K1_ROUTE` lets a headless `silky-run snap` reach every ending (`05_bad1`, `05_bad2`, `05_03_5` with `ed00`, `06_2`, `06_1` with `06_TRUE` and `ed01`, `06_BAD3`). Use it with `--path 0`. Entries are separated by `;`.

- `sel=LABEL,...`: at a talk list, takes the first listed label that is found. Other lists go as before.
- `KIND@SCRIPT=N,...`: forces the result of a screen (`actmenu`, `area`, `exselect`, `item`, `map`) in that script, one per screen in turn.

Routes for `sel=`:

| Ending | Labels |
| --- | --- |
| bad2 | 耐える,断じてイチャイチャしない,振り払う,可愛いおじさん,あまり読まない,これ以上は近づかない,その場に腰を下ろす,声をかけつづける |
| ed00, 06_2 | 耐える,ここではイチャイチャしない,振り払う,実はまだ……,参考になります,漫画の好みを聞いてみる,知っていると答える,可愛いおじさん,あまり読まない,これ以上は近づかない,その場に腰を下ろす,声をかけつづける |
| true | 抱きしめる,ここではイチャイチャしない,振り払う,これ以上は近づかない,その場に腰を下ろす,声をかけつづける |
| bad3 | 抱きしめる,イチャイチャしていく,振り払う,実はまだ……,参考になります,漫画の好みを聞いてみる,知っていると答える,可愛いおじさん,あまり読まない,これ以上は近づかない,その場に腰を下ろす,声をかけつづける |

The earlier `03_03_c-3` loop was the picker, not the port.

## Where things are in the executable

Addresses in `kimaten.exe` of the build above. The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` dispatcher (vtable 0x53d328) | 0x496800 |
| Window: shared init, window init, colour fill | 0x470f00, 0x4755b0, 0x475830 |
| Window: text box reset, begin message and position, update | 0x475a00, 0x476680, 0x476be0 |
| Window: hide and close, typewriter, name draw, name parameters | 0x475ff0, 0x478f60, 0x477960, 0x541ec0 |
| Window: `Game.11`, button state (reads h69), draw a cell, hover | 0x496fa0, 0x472350, 0x472740, 0x471a30 |
| Window: area updates, skip toggles, scene skip | 0x477500, 0x478130, 0x471260 |
| Window: bar slide down and up, fold button handler, `Game.10` sub 15 blink | 0x4731a0, 0x472e30, 0x473500, 0x473bc0 |
| Window: open and close, tooltip balloon | 0x471bd0, 0x471cf0, 0x472950 |
| Window: open save, load, backlog, settings, notebook | 0x4c00d0, 0x4c0100, 0x4bffa0, 0x4c0130, 0x4c0350 |
| Message window vtables | 0x53af2c, 0x53af44 |
| Wait mark: vtable, draw frame, step | 0x53bea0, 0x4839e0, 0x483aa0 |
| Sound defaults: OK `ses_001`, cancel `ses_002` | 0x49d2e0, 0x49d310 |
| Action menu: `Game.42`, create, init, run | 0x49c1e0, 0x441e50, 0x442040, 0x442400 |
| Action menu: draw, click, hover, right click | 0x443390, 0x443520, 0x443fd0, 0x4429b0, 0x442940, 0x442e40 |
| Action menu: system button enabled | 0x443d30 |
| Investigation: `Game.41`, create, init, run | 0x49c0c0, 0x4484a0, 0x4486d0, 0x448a40 |
| Investigation: hover, tint, draw, pixel, help line, click, cancel | 0x448fe0, 0x44a150, 0x4495b0, 0x449060, 0x449f30, 0x4490f0, 0x4493d0 |
| Evidence choice: `Game.46`, create, init, run | 0x49c400, 0x46bef0, 0x46c110, 0x46c560 |
| Evidence choice: layout, draw, help line, click, tables | 0x46d520, 0x46d3d0, 0x46de60, 0x46cb90, 0x570110, 0x541f40 |
| Evidence cards: `Game.47`, setup, animation | 0x49c770, 0x46e0f0, 0x46e300 |
| Option list: `Game.45`, vtable, create, init, run | 0x49c4f0, 0x53a474, 0x4618f0, 0x461ae0, 0x461f10 |
| Option list: open (`areaselect.area`), hover, click, cancel | 0x462430, 0x462630, 0x4626f0, 0x462970 |
| Option list: draw by id, sprite layout, page layout | 0x462df0, 0x462ff0, 0x463190 |
| Option list: option cell draw by id (tests the on flag), flag base set in init (this+0x130) | 0x462df0, 0x462430 |
| Option list: bar state (id 5 grey by byte 899), help line, cell draw, up and down arrows | 0x463ff0, 0x464240, 0x463960, 0x463a30, 0x463b00 |
| Option list: up and down strings, back help string | 0x53a454, 0x53a458, 0x5380d8 |
| Help lines: investigation (「ここを調べる」), action menu strings, evidence strings | 0x5380c8, 0x53793c..0x537988, 0x53a77c..0x53a7c4 |
| Standard selection | 0x47c530, 0x47c970 |
| Map: help line by id, highlight by id, floor frames | 0x470b20, 0x470070, 0x470220 |
| Map: `Game.43`, vtable, create, load assets | 0x49c2f0, 0x53aba4, 0x46f210, 0x46f340 |
| Map: run, init, exit fade, show, hide | 0x46f690, 0x46f7e0, 0x46f7c0, 0x46f610, 0x46f650 |
| Map: hover, click, right click, keyboard | 0x46f8c0, 0x46f930, 0x46fc40, 0x46fe70..0x46ffd0 |
| Map: draw static, element, info line, info text, fade in and out | 0x470220, 0x470070, 0x470b20, 0x4709a0, 0x470580, 0x470790 |
| Map: place positions, room info strings, menu info strings | 0x570150, 0x53ab10, 0x5379a4 |
| Notices: `Game.44`, sub table | 0x49c600, 0x49c754 |
| Notices: create, init, add, frame, show, clear | 0x46b760, 0x46b8b0, 0x46b940, 0x46b9b0, 0x46bb20, 0x46be30 |
| Notices: build strip, draw text, strip step, created in the game object | 0x46b190, 0x46b680, 0x46b570, 0x4bed10 |
| Notebook: vtable, create, exit, init, update, click | 0x53bca8, 0x47e140, 0x47e4d0, 0x47e5c0, 0x47e6c0, 0x47e7e0 |
| Notebook: draw element, static, list, page | 0x47f1c0, 0x47f740, 0x47f880, 0x480ed0 |
| Notebook: scroll bar draw, thumb draw, drag loop | 0x480490, 0x4805a0, 0x480640 |
| Notebook: select entry, info line, voice lines and next line, unlock all | 0x480a70, 0x47ffe0, 0x482100, 0x4821b0 |
| Notebook: fade in and out, wheel, thumb press | 0x47fb60, 0x47fda0, 0x47edb0, 0x47ee70 |
| Notebook: keyboard (up, down, left, right), entry variable table | 0x47eeb0, 0x47ef70, 0x47f040, 0x47f110, 0x5368f0..0x536980 |
| Notebook: menu button state (grey or current), help text draw, pointer to element, element lookup | 0x4904e0, 0x4908b0, 0x49cd50, 0x49ccc0 |
| Skip tutorial: vtable | 0x53cf34 |
| Skip tutorial: create, open, hover, press and close | 0x4928a0, 0x4929e0, 0x492ae0, 0x492bb0 |
| Skip tutorial: fade in, fade out | 0x492d80, 0x492f30 |
| Skip tutorial: modal run, run loop, opened from the skip press | 0x444660, 0x4b1510, 0x4429b0 |
| Confirm dialog: build, press, right click | 0x45c6d0, 0x45c9b0, 0x45ca50 |
| Title: start, create | 0x4912d0, 0x490e10 |
| Title: `Game.110`, tab lock check, button state | 0x497c90, 0x491470, 0x491a50 |
| Title: source cells (main, extras, widths), destination cells | 0x570180, 0x570198, 0x570240, 0x5701e0, 0x570210 |
| Title: extras buttons start, main start, fall-out, bounce | 0x491ff0, 0x492190, 0x4920a0, 0x492450 |
| Title: tick, face change and fade | 0x492380, 0x492530, 0x4926d0 |
| Title sparkles (prism): create sprites, update, spawn, step, random | 0x45d2b0, 0x45d480, 0x45d3d0, 0x45d400, 0x45d550, 0x45d690, 0x45d790 |
| Title sparkles: MT seed, MT draw | 0x45cff0, 0x45d040 |
| Title sparkles: constants (alpha table 0x541e40) | 0x541e40, 0x541ee0, 0x541da4, 0x541d10, 0x541e08, 0x541df8, 0x541da0 |
| Extras: mode loop, `Game.30` to `Game.33` | 0x49a1d0 |
| CG mode: picture table, view and registration copies, grid entries | 0x56ad70, 0x56d250, 0x56ab60 |
| CG mode: page draw, element states, info strings, unlock and dialog, dialog draw | 0x450f80, 0x450b60, 0x451b20, 0x451d20, 0x45c6d0 |
| CG registration: by background name (reads h69), by id, `Game.12` | 0x452380, 0x4527c0, 0x4970b0 |
| CG view: create, draw, handlers | 0x452b70, 0x453a70, 0x454340, 0x454660 |
| Extras: CG mode press, right click; CG grid press, right click | 0x450220, 0x450690, 0x453180, 0x453370 |
| Extras: scene mode press | 0x469a40 |
| Extras: music mode press, right click | 0x479fd0, 0x47a340 |
| Scene mode: flags 3101 to 3108, percent table, frame counts | 0x56f730, 0x56f760, 0x5368c8 |
| Scene mode: draw, hover animation, percent | 0x46a4a0, 0x46aa40, 0x482c20 |
| Music mode: list draw, row state, scroll, volume, play | 0x47b3b0, 0x47a840, 0x47b530, 0x47b0c0, 0x47ae30, 0x47ab90 |
| Tabs: availability (0xbba and on), CG percent, digits | 0x482960, 0x4829d0, 0x482ce0 |
| Anne viewer: mode loop case, create, init, result, run start | 0x49a1d0, 0x444270, 0x4443f0, 0x444660, 0x4447e0 |
| Anne viewer: hover, press, right click, wheel, release, keyboard | 0x444880, 0x4448e0, 0x444c40, 0x444cc0, 0x444d90, 0x444dd0..0x445170 |
| Anne viewer: slider knob, draw element, draw all (unlock bytes 0xbc2 to 0xbcd), button fade in and out | 0x445310, 0x4453b0, 0x445850, 0x445ce0, 0x445ef0 |
| Anne viewer: slider drag, motion rows ("アクション%03d" at 0x537cd4), scroll | 0x446100, 0x446580, 0x446640, 0x446980, 0x446e90 |
| Anne viewer: model load (names 0x537ce4..0x537d14), face, outfit, face picture | 0x446f10, 0x447520, 0x447590, 0x447740 |
| Anne viewer: motion press with stop-first list, model swap, glyph box, pose cycle | 0x4475f0, 0x4476f0, 0x447810, 0x4479c0 |
| Anne viewer: glyph draw with edge flags, font size, font create, glyph blit | 0x4adfc0, 0x4adef0, 0x4ae250, 0x4ae2e0 |
| Sounds: OK, cancel | 0x4b2010, 0x4b2110 |
| Anne viewer: help text (table 0x537d38..0x538014), unlock | 0x447b00, 0x447f30 |
| E-mote: `Game.10`, scene skip check | 0x496c10, 0x471430, 0x4713e0 |
| E-mote: `Game.10` sub 6 load (x -542, y 400, 0.35, fade 16) | 0x43fc20, 0x43fcc0 |
| E-mote: `Game.10` subs 7, 8, 9, 11, 12, 13, 16, 17 | 0x43dd60, 0x43fef0, 0x43ff50, 0x4401f0, 0x43fbe0, 0x43db10, 0x43da70, 0x43de30 |
| E-mote: `Game.10` subs 10, 14 (message window) | 0x475b60, 0x4763a0 |
| E-mote: companion, fade wait | 0x4402b0, 0x440100, 0x440250 |
| E-mote: `Game.40`, subs 0 and 12 load, slot x table, costume name to id | 0x49b3b0, 0x43b5c0, 0x43c6f0, 0x43c770 |
| E-mote: `Game.40` y formula constants | 0x541da4, 0x541de0, 0x541d28, 0x541e10 |
| E-mote: `Game.40` subs 1, 2, 3, 4, 8, 9 | 0x43c1a0, 0x43bd70, 0x43be90, 0x43bda0, 0x43bf60, 0x43c290 |
| E-mote: `Game.40` subs 13 to 18 | 0x43b9f0, 0x43b930, 0x43b990, 0x49bd40, 0x43c1f0, 0x43c9c0 |
| E-mote: `Game.40` subs 21 and 31 | 0x43c170, 0x4403f0, 0x439cd0, 0x43ef20, 0x43f3a0 |
| E-mote: `Game.40` sub 100 white flash, white-out vtable | 0x43b210, 0x5378b4 |
| E-mote: standing model manager | 0x43ba40 |
| E-mote: main timeline, face, extra timeline | 0x4361a0, 0x4362f0, 0x436510 |
| E-mote: `Game.48`, load, face, remove, fade out, main timeline | 0x49bea0, 0x439a00, 0x43a4b0, 0x439d10, 0x439b60, 0x439be0, 0x43a6e0 |
| E-mote: CG model vtables, fade in and out | 0x537148, 0x537160, 0x537250, 0x440a20, 0x440b40 |
| Voice-timed models: `Game.1007`, `.vo` parser, record start, reset | 0x499990, 0x464a10, 0x464cc0, 0x464c70 |
| Voice-timed models: kinds 0 to 4 | 0x465200, 0x465900, 0x4663e0, 0x466c90, 0x467fb0 |
| Strip wipe: `Game.1000`, setup, start | 0x498310, 0x46e6d0, 0x46e800 |
| E-mote colour (`Game.48`, `Game.40`) | 0x43b210, 0x441d50, 0x441d70, 0x441d90, 0x43ce40 |
| E-mote colour: `emotedriver.dll` SetColor | 0x10002790 -> 0x10014690 |
| `Game.35`: handler and subs | 0x49aeb0, 0x46eb60, 0x46edd0, 0x46f010 |
| `Game.993`, `999`, `14`, `961`, `13` | 0x497db0, 0x4981f0, 0x497b40, 0x49b1d0, 0x4974a0 |
| `Game.1001`, `1002`, `1006`, `1009`, `1010`, `1011` | 0x4985b0, 0x498000, 0x4993f0, 0x499a60, 0x499b50, 0x499d20 |
| `Game.10008`, `10009`, `989` | 0x49ca40, 0x49c990, 0x499df0 |
| `Game.1003`, its callee | 0x496a34, 0x4bec70 |
| Effect classes: soot vtable, create, setup, update, spawner | 0x53a3ec, 0x45ff50, 0x460060, 0x460370, 0x4603e0 |
| Effect classes: soot speck setup, update | 0x45fc40, 0x45fe70 |
| Effect classes: random numbers (rejection sampling) | 0x45d040 |
| Effect classes: S0402 vtable, setup, peak pick, update, wave | 0x53a2bc, 0x45d950, 0x45da50, 0x45dad0, 0x45dc00 |
| Effect classes: S04022 constructor, setup, init, update | 0x45de90, 0x45dfc0, 0x45e190, 0x45e590 |
| Effect classes: S04022 piece placement, key, random | 0x45e5f0, 0x45e8a0, 0x45e9b0 |
| Effect classes: S0405 constructor, setup, mover update, mover setup, mover init | 0x45ea80, 0x45ebb0, 0x45eec0, 0x45f0f0, 0x45e9f0 |
| Effect classes: S0501 vtable, setup | 0x53a37c, 0x45f400 |
| Effect classes: S0501202 constructor, init, update, next picture | 0x45f650, 0x45f770, 0x45f8a0, 0x45f8d0 |
| Effect classes: prism, spiral, string animation vtables | 0x53a28c, 0x53a410, 0x53a440 |
| String animation (unused), key constructor, copy, add | 0x493160, 0x4ce390, 0x4ce410, 0x4cd8b0 |
| Timeline: push, blocking run, graceful stop; key stepper, finish all | 0x4cbec0, 0x4cb9e0, 0x4cbd40, 0x4cdce0, 0x4cd950 |
| Sprites: SetZ, z-list, manager draw; texture draw (additive), transform | 0x4cd820, 0x4ce700, 0x4ce940, 0x4d04c0, 0x4d06a0 |
| Sprite factory, character z table, background loader | 0x4b3470, 0x43c6f0, 0x490b60 |
| `Game.1002` subs 1 to 6 | 0x493340, 0x4948c0, 0x4945a0, 0x493510, 0x494160, 0x493d50 |
| Staff roll: `Game.990`, vtable, create, start, skip, update | 0x49a0b0, 0x53cb5c, 0x48eec0, 0x48f290, 0x48f430, 0x48f530, 0x48f5e0, 0x48f580 |
| Staff roll: timing tables (roll 106999 ms, strip 1280 x 3080) | 0x56f728, 0x56f784 |
