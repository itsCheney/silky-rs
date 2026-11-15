# engine-2017

English | [简体中文](engine-2017.zh-CN.md)

The engine of `apeiria.exe` 1.0.3.0 (景の海のアペイリア), linked 2017-11-09 with Visual Studio 2013, SHA-256 `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e`. Read from the executable unless marked.

## Differences from engine-2021

| Call | This build | engine-2021 |
| --- | --- | --- |
| `Sprite.1` on layer 5 | Creates the layer hidden, 2048 pixels square: a store for button sheets (`title_p.akb`) that the game's screens draw in cells | Shows the picture |
| `BackLog.3` without arguments | Ends the voice mark of the entry | Not called so |
| `Movie.2` | Closes the movie, returns nothing (the scripts call it after each scene change) | Plays the movie, returns whether it played |
| `Movie.3` | Returns a value | Returns nothing |
| `BackLog.4` to `BackLog.7` | Return a value | Return nothing |
| `OneMes.5` | Returns a value | Returns nothing |
| `Save.3`, `Save.8` | Return a value | Return nothing |
| `HEffect` (class 17) | Plays sounds like `Effect` (scripts) | Not done |

## Games

- [景の海のアペイリア](../games/apeiria.md)
