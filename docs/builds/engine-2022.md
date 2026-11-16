# engine-2022

English | [简体中文](engine-2022.zh-CN.md)

The engine of the 同級生 Remake Steam release (English, Simplified and Traditional Chinese): `doukyuusei.exe` 1.0.4.0, linked 2022-03-15 07:54:45 with Visual Studio 2017 (linker 14.16), 11,159,880 bytes, SHA-256 `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451`. The program carries Steam's DRM wrapper.

Its screen is 1920 x 1080. The engine classes are taken as those of [engine-2026](engine-2026.md) without `sprites_and_flags`; they have not been compared call by call yet.

What differs from the other builds so far:

- Text tables: one MAP file per script and language (`X_EN.MAP`, `X_CN.MAP`, `X_TW.MAP`): a u32 count, then that many pairs of u32 line id and u32 offset, then UTF-16 lines ending in a 0 unit. The scripts carry no text; `Game.24` shows a line by its id (see the game page).
- Message text in bitmap fonts: `font_han.akb` (half-width, 20 x 40 cells), `font_han_i.akb` (the same letters in italics) and `font_zen.akb` (full-width, 40 x 40 cells), 20 cells to a row, in the order of `font_han.map` and `font_zen.map` (a u16 count, then that many UTF-16 characters).
- `update.arc` holds newer copies of files of every archive and is read first.
- The movie calls are those of 2017 and 2021: `Movie.0` takes the file alone, `Movie.3` plays and returns whether it played (the scripts keep the result), `Movie.2` closes.
- Simplified Chinese pictures end in `_ch` (`title_bg_ch.akb`), the text tables in `_cn`.

## Games

- [同級生 Remake](../games/doukyuusei.md) (Steam)
