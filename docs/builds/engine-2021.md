# engine-2021

English | [简体中文](engine-2021.zh-CN.md)

The engine of the ふゆから、くるる。 package, Japanese only, Visual Studio 2019:

- `fuyukuru.exe` 1.0.0.0 as installed, linked 2021-09-06, SHA-256 `9797a7cb71955c0ddb4b4865e0e592166f82386a5e642bd25072922cf9d6d9fa`.
- `fuyukuru.exe` 1.0.1.0 of the official update 1.01, linked 2021-09-29, SHA-256 `b5b163ceb979d9825c6586401da005d51a57f4005a93cf227ca304bf9e831988`.

Checked with update 1.01: headless, the story runs with every engine call carried out, and the title menu, settings, extras page and save screen show. From the scripts unless marked.

## Differences from engine-2026

| Call | This build | engine-2026 |
| --- | --- | --- |
| `Movie.0` | (file) sets up a movie | (channel, file) |
| `Movie.3` | Plays the movie, returns whether it played | Not called |
| `Movie.2` | Closes the movie, returns nothing | Plays the movie, returns whether it played |

The movie calls are those of [engine-2017](engine-2017.md).

## Game data

The package has the Japanese art under plain names (`title_p.akb`), where the download has one file per language (`title_p_jp.akb`, `title_p_en.akb`, ...); the runtime takes the plain name when the language's file is missing. The save screen sheet is `saveload_p.akb` (`save_load_p_<lang>.akb` in the download). The title menu cells are those the download's executable lists for Japanese.

## Games

- [ふゆから、くるる。](../games/fuyukuru.md) (package)
