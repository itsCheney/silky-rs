# 先輩が私の妄想にドージンする

English | [简体中文](doujin.zh-CN.md)

The runtime knows the game by `doujin.exe` in its folder (`Title::Doujin`). Program version 1.0.1.0, linked 2024-08-30 07:40:15 UTC with Visual Studio 2019 (linker 14.29), image base 0x400000. Size and SHA-256 not recorded yet: the copy read was not the file as shipped. The program ships inside a disc-check wrapper (a separate small program the game's exe starts from); the addresses here are those of the program inside the wrapper, as linked. Engine profile: [engine-2024-08](../builds/engine-2024-08.md). Its own `Game` commands: [game-commands.md](../game-commands.md#先輩が私の妄想にドージンする).

All scripts decode. Headless, `silky-run snap --path 0` plays from the start to an ending: about 6160 message lines through ten chapter scripts. The game adds its own commands to the engine (native class 29); which of them return a value comes from the executable.

## Current state

- Selections: before each option the script tests a flag, to hide routes already taken. The runtime gathers those options into one selection and answers the option's own value + 1, as the scripts expect.
- Engine calls are all carried out, except `MesWnd.3` and `Sprite.5` (once each).
- Carried out: the title menu (`Game.110`), the speaker name (`Game.11`), the second portrait window (`Game.16` subs 6, 7, 9 and 11), the breath animation (`Game.1006`, from `.brt` files), the pictures that change with the voice line (`Game.1007`, from `.vo` files), the zoom and white flash (1005), the rule wipe (12.8) and the auto save point (14).
- Extras (`Game.30` to `Game.32`, from `OMAKE.MES`): the CG, scene and music modes are the game's own screens (`omake.rs`), tabs of one screen. The back button and a right click answer -1 (the script returns to the title); only the scene mode answers a number, the scene picked (0 to 18, played by `SCMODE.MES`). Pictures seen are bits from byte 3300, set as the picture and its layers on screen match a picture of the game's table (`Game.12` subs 2 and 3); scenes seen are bytes 3101 to 3119, tracks heard bytes 3051 to 3063, and bytes 0xbba to 0xbbc open the tabs. The CG mode has four pages of nine entries and a grid of the entry's variants (nine to a page); the scene mode has three pages and plays two frames of the thumbnail under the pointer; the music mode lists the 13 tracks with a scroll bar and a volume bar and plays a picked track once. The CG and scene modes start the opening's instrumental when no music is playing. The original shows no help lines there (`CGMODEHELP.HLG`, `SCENEHELP.HLG` and `MUSICMODEHELP.HLG` are not read). Every press that does something plays the confirm sound (`ses_001.wav`) and the right click that leaves plays the cancel sound (`ses_002.wav`); the bars, the variant grid's picture view and a press that does nothing are silent. In the music mode a right click stops a playing track first and leaves only when none plays, and the playing row clears when the track ends.
- Accepted with nothing to do: 13.18 (it stops what 13.17 attaches to a standing character), 993 apart from 993.1, 1003, 961.1, 12.7, 10.4, 10.5 and 10.16.
- Carried out: the doujinshi screen (`Game.40`, `comic.rs`): the page picture as the background, the bubbles' words set vertically in 10-pixel cells from the point the script gives (taken from the lettered `do0Na.akb` page where there is one, else drawn with the font), the two round page buttons (`comic.akb`, areas of `DOUJINSHI.AREA`; they return 2 and 3 from a wait), the hot spots of sub 6 and the wait of subs 0 and 1 (0 on a click). `silky-run snap --title 257` plays it headless.
- Not carried out yet: `Game.16` subs 8 and 12, and of the doujinshi screen the one-by-one appearance of the words, the buttons' hover look and sound, the fade of a hot spot and the code 1 (leave).
- Movies: the game's movies (`do-jin*.dat`) are ASF files (VC-1 video, WMA Pro sound). The runtime decodes MPEG-1 only, so they are not played yet.
- Interface: the game's own title menu (`title_p.akb` cells over the areas of `title.area` and `title2.area`, extras page started by `Game.110(1)`), the message window's button bar with its click-wait mark, padlock and tooltips, the speaker name on the left of the text and the two portrait windows (`Game.10` on the left, `Game.16` on the right) are drawn. The window is the shaded band alone (the second, textured block of `message_p.akb` is not drawn), the text starts at x = 180 and the title button shows the home cell. Not measured against the original yet: the name's size and baseline, the click-wait frame time and the bar's show height; the pages of the title do not fade into each other and the full-screen skip picture (`skip.akb`) is not shown.

## Open items

- Carry out the game's own commands listed above.
- Decode the movies (VC-1 and WMA Pro in ASF).
- Check the title menu, name line, button bar, wait mark and portrait places against the original under Wine; the page fade of the title menu and the skip picture.
- Carry out `MesWnd.3` and `Sprite.5`.
- The meaning of the codes returned by `Game.20` and `Game.110` is not known yet.
- Check the runtime against the original program under Wine.

## Where things are in the executable

Addresses in `doujin.exe` of the build above, as linked. The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` dispatcher (`CFuncExec` slot 0) | 0x494ac0 |
| `CFuncExec` vtable | 0x50b158 |
| Command 10 to 110: byte table, jump table | 0x495564, 0x495520 |
| Command above 910: byte table, jump table | 0x4955fc, 0x4955cc |
| `Game.10`, portrait set (subs 4 to 16) | 0x495640 |
| `Game.11`, name plate | 0x494b58 |
| `Game.12`, background and difference pictures | 0x495f60 |
| `Game.13`, standing characters | 0x4967b0 |
| `Game.14` | 0x494c72 |
| `Game.16`, second portrait set (subs 6 to 12) | 0x495b90 |
| `Game.20`, selection (result from 0x480750) | 0x494d7c |
| `Game.30` to `Game.32`, CG, scene and music modes | 0x498580 |
| CG mode screen: build, click, cell state, page fill | 0x456ed0, 0x457040, 0x457a70, 0x457e90 |
| CG mode variant grid: page, cell state, click, picture on screen, next picture | 0x45a640, 0x45a400, 0x459d50, 0x45aeb0, 0x45b170 |
| CG mode: cell table (29 entries), picture table (279 pictures, background and layers) | 0x535ce0, 0x535ec0 |
| CG seen bits: name registration, commit | 0x458e40, 0x458c30 |
| Scene mode screen: build, click, cell state, page fill, hover animation | 0x46c710, 0x46cb60, 0x46d210, 0x46d580, 0x46db80 |
| Scene mode: scene byte per cell, thumbnail frame counts | 0x53c520, 0x5030b0 |
| Music mode screen (vtable 0x509df8): code | 0x47dfc0, 0x47e730, 0x47f1c0 |
| Extras screens: confirm sound (`ses_001.wav`), cancel sound (`ses_002.wav`), shared vtable slots 0x88 and 0x8c; music mode right click, track end check | 0x439800, 0x439950, 0x47eff0, 0x47e730 |
| Completion percent of the CG, scene and music modes, and its drawing | 0x483090, 0x483270, 0x483310, 0x483380 |
| `Game.35` | 0x499650 |
| `Game.40`, doujinshi screen | 0x49a140 |
| Doujinshi object: vtable, start, step (`Game.40.5`), wait (`Game.40.0`) | 0x508eb8, 0x466370, 0x466630, 0x466790 |
| Doujinshi object: frame update, per-frame update, button draw, click | 0x4669b0, 0x466e90, 0x467130, 0x467330 |
| Doujinshi object: text typing, finish typing, hot spot animation | 0x467880, 0x467cc0, 0x468430 |
| Doujinshi bubble texts (strings from `db0001.ogg` to `comic.akb`) | 0x507a50 |
| `Game.110`, title menu (built by 0x492780) | 0x494e64 |
| `Game.961` | 0x499820 |
| `Game.990`, staff roll | 0x495047 |
| `Game.993` | 0x497780 |
| `Game.1003` | 0x495142 |
| `Game.1005`, zoom and white flash | 0x497f20 |
| `Game.1006`, animation layers (byte table 0x497f08, jump table 0x497ef4) | 0x497ad0 |
| `Game.1007`, voice-linked pictures (`.vo` loader 0x469970) | 0x49516d |
| `Game.1006` breath animation: list slot, entry, key builder, `.brt` loader | 0x456180, 0x455b30, 0x455e20, 0x456360 |
| `Game.1006` breath fade step (user step), clear all | 0x455aa0, 0x455d90 |
| Keyframe stepper, key constructor, add key, start and stop, timeline | 0x4ac3e0, 0x4acc00, 0x4abfc0, 0x4ad490, 0x4ad060 |
| `Game.1003` object (sub switch) | 0x475de0 |
| `Game.12.8`, rule wipe of the shown picture | 0x483e50 |
| `Game.12.7`, reset of globals | 0x484270 |
| `Game.1009` | 0x495228 |
| `Game.1010` | 0x4983b0 |
| Title menu object: build, page switch, click, hover, cell state | 0x492780, 0x4938e0, 0x492b80, 0x492b30, 0x4931d0 |
| Title cell tables: places, sheet cells, sizes (11 cells) | 0x53c6c0, 0x53c668, 0x53c718 |
| Message window layers and button cells | 0x470aa0 |
| Message window button tooltip lines | 0x471310 |
| Message window button states (hide 0xd, padlock 0xe); source column of each button (home and power cells swapped, table at 0x50f320) | 0x4725b0 |
| Message window click-wait mark | 0x476f00 |
| Portrait windows: second (`Game.16`), first (`Game.10`) | 0x474c90, 0x475340 |
| Message window settings (`/Config/MesWnd/*`): position, move flag | 0x424130, 0x424200 |
| Padlock setting (`/Config/MenuLock/Enable`, 1 when not set) | 0x425830 |
| Config flag test used by 10, 12, 16, 993, 1006 and 1007 | 0x423ee0 |
| Message window list (singleton) | 0x49d600 |
| Game object holder (singleton) | 0x49a5f0 |
