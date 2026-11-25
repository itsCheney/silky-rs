# engine-2017

English | [简体中文](engine-2017.zh-CN.md)

The engine of four programs, all linked with Visual Studio 2013. Read from the executable unless marked.

| Program | Version | Linked (UTC) | SHA-256 |
| --- | --- | --- | --- |
| `apeiria.exe` (景の海のアペイリア) | 1.0.3.0 | 2017-11-09 07:06:49 | `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e` |
| `casablanca.exe` (景の海のアペイリア ～カサブランカの騎士～) | 1.0.0.0 | 2017-12-10 14:52:07 | `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716` |
| `kimaten.exe` (きまぐれテンプテーション, with update 1.04) | 1.0.4.0 | 2019-12-16 04:48:49 | `36148a0286bd9d31ceaec4323f4be987319bd759e68b5961c59c992b07b54700` |
| `nanairo.exe` (なないろリンカネーション) | 1.0.0.2 | 2014-09-09 19:27:20 | not recorded yet |

`casablanca.exe` is 1,559,040 bytes. Its engine code is the same as that of `apeiria.exe`, so the differences below hold for both. The trace probe addresses are given for `apeiria.exe`, `casablanca.exe` and `nanairo.exe`.

`kimaten.exe` is 2,039,808 bytes. Its engine calls match the table below at the points checked (`Sprite.1` on layer 5, `BackLog`, `Movie.3`, `HEffect`); its `Game` commands are described in [kimaten1.md](../games/kimaten1.md).

The size of `nanairo.exe` is not recorded yet. Its engine calls that return a value match the table below; its `Game` commands are described in [nanairo.md](../games/nanairo.md).

## Differences from engine-2026

| Call | This build | engine-2026 |
| --- | --- | --- |
| `Sprite.1` on layer 5 | Creates the layer hidden, 2048 pixels square: a store for button sheets (`title_p.akb`) that the game's screens draw in cells | Shows the picture |
| `BackLog.3` without arguments | Ends the voice mark of the entry | Not called so |
| `Movie.2` | Closes the movie, returns nothing (the scripts call it after each scene change) | Plays the movie, returns whether it played |
| `Movie.3` | Returns a value | Returns nothing |
| `Movie.1` | (_, file): a moving background, when movies are on (scripts). The runtime does not decode it yet and shows the picture of the same name | Not called |
| `BackLog.4` to `BackLog.7` | Return a value | Return nothing |
| `OneMes.5` | Returns a value | Returns nothing |
| `Save.3`, `Save.8` | Return a value | Return nothing |
| `HEffect` (class 17) | Plays sounds on a second set of effect channels, with the same sub commands as `Effect` (scripts) | Not done |

## Trace probe

Addresses in `apeiria.exe` as linked (image base 0x400000), for `tools/trace/probe.py` ([trace.md](../trace.md)). Checked under Wine in the first chapters.

| What | Where |
| --- | --- |
| Script opcode switch (opcode 0x18 goes to the `native` handler, 0x19 to `message`) | 0x4b2a95 (byte table 0x4b2e34, jump table 0x4b2d74) |
| `native` handler; pops the class, then a switch on class − 1 | 0x49e0a0 |
| Breakpoint: class popped, statement still on the stack | 0x49e103 (`eax` = class, `esi` = the script object, its value stack a vector at its start) |
| Value pop: copies the top value, 0x20 bytes; kind at +0 (0 number, 1 string), number at +4, `std::string` at +8 | 0x4b2e90 |
| `message` handler (a breakpoint here marks each message) | 0x4b2c00 |

The spec for this build:

```json
{
  "name": "apeiria 1.0.3.0 (2017)",
  "stack": {"vector": "esi", "size": 32, "kind": 0, "int": 4, "str": 8, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x49e103", "stack": true, "call": {"read": "eax", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}},
    {"at": "0x4b2c00", "call": "message"}
  ]
}
```

Breakpoints at the start and end of a routine time it: with `--clock`, the hologram's show (0x467eb0, ending at 0x4681ab) was timed this way ([apeiria.md](../games/apeiria.md)).

### casablanca.exe

Addresses in `casablanca.exe` as linked (image base 0x400000). The functions are those above, at other addresses; the stack layout is the same.

| What | Where |
| --- | --- |
| Script opcode switch | 0x4b4445 (byte table 0x4b47e4, jump table 0x4b4724) |
| `native` handler | 0x49f9c0 |
| Breakpoint at the `native` handler: class popped, statement still on the stack | 0x49fa23 (`eax` = class) |
| Value pop | 0x4b4840 |
| `message` handler (a breakpoint here marks each message) | 0x4b45b0 |
| Class 16 `Effect`, 17 `HEffect`, 18 `Voice`: build (cases of the class switch), shared sub command switch | 0x49fca3, 0x49fcdc, 0x49fd15; 0x4adc70 (jump table 0x4add34) |
| `Effect` and `HEffect`: method tables, channel list builders; volume readers | 0x52ea14, 0x52b758; 0x4ae700, 0x4ae9c0; 0x495050, 0x4950f0 |

The spec for this build:

```json
{
  "name": "casablanca 1.0.0.0 (2017)",
  "stack": {"vector": "esi", "size": 32, "kind": 0, "int": 4, "str": 8, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x49fa23", "stack": true, "call": {"read": "eax", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}},
    {"at": "0x4b45b0", "call": "message"}
  ]
}
```

### nanairo.exe

Addresses in `nanairo.exe` as linked (image base 0x400000, version 1.0.0.2). The functions are those above, at other addresses; the stack layout is the same (a value is 0x20 bytes, kind at +0, number at +4, `std::string` at +8, the value stack a vector at the start of the script object). Not yet run under Wine.

| What | Where |
| --- | --- |
| Script opcode switch (opcode 0x18 goes to the `native` handler, 0x19 to `message`) | 0x492815 (byte table 0x492ba4, jump table 0x492ae4; the range check and byte lookup start at 0x492805) |
| `native` handler; pops the class, then a switch on class − 1 (jump table 0x47eab4, no byte table) | 0x47e3d0 |
| Breakpoint: class popped, statement still on the stack | 0x47e43c (`eax` = class, `esi` = the script object, its value stack a vector at its start) |
| Value pop: copies the top value into the caller's buffer and returns it; 0x20 bytes, kind at +0, number at +4, `std::string` at +8 | 0x492c00 |
| `message` handler (a breakpoint here marks each message) | 0x480410 |

Here the class is read just before the breakpoint (`dec eax` at 0x47e43c is the first instruction of the switch), and the pop routine takes the script object in `ecx` and an output buffer on the stack. The spec for this build:

```json
{
  "name": "nanairo 1.0.0.2",
  "stack": {"vector": "esi", "size": 32, "kind": 0, "int": 4, "str": 8, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x47e43c", "stack": true, "call": {"read": "eax", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}},
    {"at": "0x480410", "call": "message"}
  ]
}
```

## Install file and saves

`apeiria.exe` reads `apeiria_inst.xml` from `AppData\Roaming\SilkysPlus\apeiria` and stops at once if `InstallPath` there is not set; its saves and settings go to the `Save` folder next to it. `casablanca.exe` reads `casablanca_inst.xml` from `AppData\Roaming\SilkysPlus\casablanca` in the same way, and its settings and saves are in the `Save` folder next to it too (settings file `Save\casablanca.xml`). With every bit of `onemes.dat` there set, holding Ctrl skips the story to the place to trace.

## Games

- [景の海のアペイリア](../games/apeiria.md)
- [景の海のアペイリア ～カサブランカの騎士～](../games/casablanca.md)
- [きまぐれテンプテーション](../games/kimaten1.md)
- [なないろリンカネーション](../games/nanairo.md)
