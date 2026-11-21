# engine-2022

English | [简体中文](engine-2022.zh-CN.md)

The engine of the 同級生 Remake Steam release (English, Simplified and Traditional Chinese): `doukyuusei.exe` 1.0.4.0, linked 2022-03-15 07:54:45 with Visual Studio 2017 (linker 14.16), 11,159,880 bytes, SHA-256 `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451`. The program carries Steam's DRM wrapper.

Its screen is 1920 x 1080. The engine classes are taken as those of [engine-2026](engine-2026.md) without `sprites_and_flags`; from the start to the title menu they agree with the original call by call (see the trace probe below).

What differs from the other builds so far:

- Text tables: one MAP file per script and language (`X_EN.MAP`, `X_CN.MAP`, `X_TW.MAP`): a u32 count, then that many pairs of u32 line id and u32 offset, then UTF-16 lines ending in a 0 unit. The scripts carry no text; `Game.24` shows a line by its id (see the game page).
- Message text in bitmap fonts: `font_han.akb` (half-width, 20 x 40 cells), `font_han_i.akb` (the same letters in italics) and `font_zen.akb` (full-width, 40 x 40 cells), 20 cells to a row, in the order of `font_han.map` and `font_zen.map` (a u16 count, then that many UTF-16 characters).
- `update.arc` holds newer copies of files of every archive and is read first.
- The movie calls are those of 2017 and 2021: `Movie.0` takes the file alone, `Movie.3` plays and returns whether it played (the scripts keep the result), `Movie.2` closes.
- Simplified Chinese pictures end in `_ch` (`title_bg_ch.akb`), the text tables in `_cn`.

## Trace probe

Addresses in the program as linked (image base 0x400000), for `tools/trace/probe.py` ([trace.md](../trace.md)). Checked under Wine from the start to the title menu, on a copy of this build without the DRM wrapper (every section other than the code is byte for byte the same).

| What | Where |
| --- | --- |
| Script opcode switch (opcode 0x18 goes to the `native` handler) | 0x461a1f (byte table 0x463434, jump table 0x463374) |
| `native` handler; pops the class | 0x43fa30 |
| Breakpoint: class popped, statement still on the stack | 0x43fa7e (`esi` = class, `[ebp-0xb8]` = the value stack, a vector) |
| Value pop: the same 0x4c-byte values as [engine-2026](engine-2026.md) | 0x463490 |

The spec for this build:

```json
{
  "name": "doukyuusei 1.0.4.0 (2022)",
  "stack": {"vector": "[ebp-0xb8]", "size": 76, "kind": 0, "int": 4, "str": 12, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x43fa7e", "stack": true, "call": {"read": "esi", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}}
  ]
}
```

`Game.10(1, 18)` does nothing in the program and leaves its 1 on the stack, so the original's next calls carry an extra 1 in front until the script returns. The runtime takes the whole statement.

## Games

- [同級生 Remake](../games/doukyuusei.md) (Steam)
