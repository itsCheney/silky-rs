# engine-2017

English | [简体中文](engine-2017.zh-CN.md)

The engine of three programs, all linked with Visual Studio 2013. Read from the executable unless marked.

| Program | Version | Linked (UTC) | SHA-256 |
| --- | --- | --- | --- |
| `apeiria.exe` (景の海のアペイリア) | 1.0.3.0 | 2017-11-09 07:06:49 | `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e` |
| `casablanca.exe` (景の海のアペイリア ～カサブランカの騎士～) | 1.0.0.0 | 2017-12-10 14:52:07 | `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716` |
| `nanairo.exe` (なないろリンカネーション) | 1.0.0.2 | 2014-09-09 19:27:20 | not recorded yet |

`casablanca.exe` is 1,559,040 bytes. Its engine code is the same as that of `apeiria.exe`, so the differences below hold for both. The trace probe addresses are given for `apeiria.exe` and `casablanca.exe`.

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
| `HEffect` (class 17) | Plays sounds like `Effect` (scripts) | Not done |

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

## Install file and saves

`apeiria.exe` reads `apeiria_inst.xml` from `AppData\Roaming\SilkysPlus\apeiria` and stops at once if `InstallPath` there is not set; its saves and settings go to the `Save` folder next to it. `casablanca.exe` reads `casablanca_inst.xml` from `AppData\Roaming\SilkysPlus\casablanca` in the same way, and its settings and saves are in the `Save` folder next to it too (settings file `Save\casablanca.xml`). With every bit of `onemes.dat` there set, holding Ctrl skips the story to the place to trace.

## Games

- [景の海のアペイリア](../games/apeiria.md)
- [景の海のアペイリア ～カサブランカの騎士～](../games/casablanca.md)
- [なないろリンカネーション](../games/nanairo.md)
