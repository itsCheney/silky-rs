# engine-2026

English | [简体中文](engine-2026.zh-CN.md)

The engine of the ふゆから、くるる。 download, `fuyukuru.exe` 1.0.1.0 with Japanese, English and Chinese, linked 2026-03-18 with Visual Studio 2022, SHA-256 `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707`.

This is the base profile: the engine classes in `crates/silky-runtime/src/host.rs` were first written against this build, and games not known yet get it. [engine.md](../engine.md) lists what the runtime does for each class. Its movie calls are those of [engine-2024](engine-2024.md), not those of the 2021 package of the same game ([engine-2021](engine-2021.md)).

## Games

- [ふゆから、くるる。](../games/fuyukuru.md) (download)

## Trace probe

Addresses in the program as linked (image base 0x400000), for `tools/trace/probe.py` ([trace.md](../trace.md)). Checked under Wine from the start to the title menu.

| What | Where |
| --- | --- |
| Script opcode switch (opcode 0x18 goes to the `native` handler) | 0x44ae7c (byte table 0x44cac4, jump table 0x44ca04) |
| `native` handler; pops the class, then a switch on class − 1 (table 0x430458) | 0x42f8e0 |
| Breakpoint: class popped, statement still on the stack | 0x42f941 (`esi` = class, `[ebp-0x58]` = the script object, its value stack a vector) |
| Value pop: copies the top value, 0x4c bytes; kind at +0 (0 number, 1 string), number at +4, `std::string` at +0xc | 0x44cb20 |
| Class 25 (`Xml`) and some others are handled inside the switch, not by their `CFunc` object | 0x42ffd9 |

The spec for this build:

```json
{
  "name": "fuyukuru 1.0.1.0 (2026)",
  "stack": {"vector": "[ebp-0x58]", "size": 76, "kind": 0, "int": 4, "str": 12, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x42f941", "stack": true, "call": {"read": "esi", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}}
  ]
}
```
