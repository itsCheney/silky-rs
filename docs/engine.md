# Engine

English | [简体中文](engine.zh-CN.md)

How silky-rs is divided between the engine all Silky Engine games share and what each game adds. The features of the shared engine are listed in [features.md](features.md).

## Engine and games

A script calls the engine with `native`: it pushes the arguments, a sub command and a class number. Classes 1 to 28 belong to the engine, which every game carries. Class 29 (`Game`) is written for each game: its commands differ from game to game, even where the number is the same.

So the code is split in two:

- The engine classes are in `crates/silky-runtime/src/host.rs` and the modules it uses (sprites, screens, sound, saves, the engine screens). They follow the engine as the executables show it, so that a game not tested yet can use them.
- Each game's own part is in `crates/silky-runtime/src/games/<game>/`: its `Game` commands, its tables and its own screens. `games/mod.rs` holds the `Game` commands that are general enough to try on any game (the common set).

A `Game` call goes to the running game's module first, then to the common set. What nobody handles is reported with its arguments and counted, never treated as success. [game-commands.md](game-commands.md) lists every `Game` command handled, with what it takes and gives back and how sure that is.

## Engine builds

Each game links its own copy of the engine, and the engine classes changed between copies. The runtime knows the copy by the program's link time stamp and size and follows its profile where the copies differ; [builds/](builds/README.md) lists the builds checked and a page for each profile.

## Engine classes

| Class | Name | What the runtime does |
| --- | --- | --- |
| 1 | Select | Selection settings; the selection itself is a `Game` command |
| 2 | MesWnd | Hides and shows the message window; in Japanese builds shows the message |
| 3 | Shake | Screen shake |
| 10 | Font | Not used: the runtime lays out text itself |
| 11 | Value | Not done yet |
| 12 | Mouse | Whether the player clicked, polled once a frame |
| 13 | Anime | Frame programs from `.ax` files that draw cells of a sheet into the background (blinking faces); read from 同級生 Remake's program and used for it only so far |
| 14 | Flag | Flags from byte 1000 on, kept across games |
| 15, 16, 18 | Music, Effect, Voice | Channels, play, stop, fade out, whether a sound plays |
| 17 | HEffect | Sounds in the 2017 build; otherwise not done yet |
| 19 | Sprite | Sprite layers and their animations (rule wipes, dimming) |
| 20 | Screen | Fades and rule wipes |
| 21 | Script | Jump to, call and load scripts |
| 22 | BackLog | In Japanese builds brings the message text; the runtime keeps its own backlog |
| 23 | Movie | `.vsd` movies |
| 24 | OneMes | Not done yet |
| 25 | Xml | Settings the original reads from its XML file |
| 26 | Ui | Waits and per-frame polls |
| 27 | Save | The store of variables the scripts rebuild the scene from after a load: `2 (table, key, default)` reads, `4 (table, key, value, 0)` writes, `5 (table, key)` and `6 (key)` delete (from one table, from all). Done in ふゆから、くるる。 and 景の海のアペイリア, kept in the scene so saves hold it; the runtime also saves the scene itself |
| 28 | Wait | Not done yet |
| 29 | Game | Per game, see [game-commands.md](game-commands.md) |

Some engine calls need nothing from this runtime in ふゆから、くるる。 (set up of the original's own message window, font and backlog, for example); they are listed in `games/fuyukuru/commands.rs` and count as carried out only there.

## How sure each part is

The docs and comments say where each behaviour comes from:

- **Compared**: checked against screenshots or a run of the original.
- **Read**: read from the executable. The code does not name addresses; the game pages list them for the build checked.
- **Scripts**: taken from how the game's scripts use the call.
- **Guess**: a value nothing confirms yet, marked in the code.

## Choices of this runtime

These are not the original's and are kept apart from what was read from it:

- Saves and settings are JSON files (`system.json` and one file per save, with a PNG thumbnail), not the original's formats.
- Fonts are given by the player; the runtime does not use the game's font setup.
- Pictures are composed at the game's size (1280 x 720) and scaled once to the output; text is drawn at the output size.
- Times given in frames are read as 1/60 s.
- Settings the original offers but this runtime does not (such as skipping H scenes) behave as the original does with them off.
