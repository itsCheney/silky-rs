# engine-2024

English | [简体中文](engine-2024.zh-CN.md)

The engine of `kimaten2.exe` 1.0.0.0 (きまぐれテンプテーション2), linked 2024-10-15 with Visual Studio 2019, SHA-256 `5cbd99107b94befe4935bbb7b8fdd7015d715a795945b434788657dee0a50d29`. It grew out of the ふゆから、くるる。 code: the program still holds that game's version line.

## Differences from engine-2021

| Call | This build | engine-2021 |
| --- | --- | --- |
| `Sprite.1` | Loads the title's button sheet, which the title screen draws in cells | Shows the picture |
| Other `Sprite` calls, `Flag`, `Mouse.4` | Not checked on this build: the 2021 handling is not tried, and the calls are reported as not carried out | Handled as read on that build |

## Games

- [きまぐれテンプテーション2](../games/kimaten2.md)
