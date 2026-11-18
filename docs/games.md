# Games

English | [简体中文](games.zh-CN.md)

How far each game runs on silky-rs. The engine shared by all games is described in [engine.md](engine.md) and [features.md](features.md). `silky-run snap` lists every engine call the runtime does not carry out yet.

| Game | Known by | Page |
| --- | --- | --- |
| ふゆから、くるる。 (international build 1.0.1.0; Japanese package 1.0.0.0 and 1.0.1.0) | `fuyukuru.exe` | [games/fuyukuru.md](games/fuyukuru.md) |
| きまぐれテンプテーション2 (2024 Japanese build, patch 1.02) | `kimaten2.exe` | [games/kimaten2.md](games/kimaten2.md) |
| 景の海のアペイリア (Japanese build with the official update), story and extras | `apeiria.exe` | [games/apeiria.md](games/apeiria.md) |
| 同級生 Remake (Steam, English and Chinese), plays through the room, the walking maps and conversations, with its own save and load screen, backlog and settings screen; the sound and schedule screens are not done | `doukyuusei.exe` | [games/doukyuusei.md](games/doukyuusei.md) |

The runtime tells the games apart by the name of the program file in the folder; it never reads the file. A folder with none of these names is run as a game not known yet: the shared engine and the common `Game` commands work, and the rest is reported. [adding-a-game.md](adding-a-game.md) describes how to add one.
