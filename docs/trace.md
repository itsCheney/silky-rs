# Traces

English | [简体中文](trace.zh-CN.md)

A trace is the list of engine calls a run made, in order: each `native` with its class, sub-command and arguments, each story and selection message, and each list of options and the option taken. Two traces of the same story point can be compared call by call, which shows where this runtime and the original program (or two versions of this runtime) part, without looking at screenshots.

## Making traces

```sh
silky-run trace <game folder> --result 29.110=1 --out stub.json    # engine calls answered by stubs
silky-run snap  <game folder> --path 0,1 --lines 300 --trace run.json
silky-player    <game folder> --trace play.json                    # written when the window closes
```

`snap` and `silky-player` run the real runtime, so their traces hold what the runtime itself answered. The player's calls carry the session clock in milliseconds as their `tick`; `trace` and `snap` have no clock and leave it out. A run that stops on an error still writes its trace, up to the call that failed.

## Comparing

```sh
silky-run diff expected.json actual.json
silky-run diff original.json run.json --ignore tick,pc,result --skip MesWnd,Wait --from message:120 --max 5
```

The first file is taken as right. Both go through the same filter: `--only` and `--skip` take classes (`Sprite`), calls (`Game.24`) or kinds (`message`); `--from` starts each trace at its first call that matches, with an optional first argument (`message:120`, `Script.0:FK002.MES`); `--ignore` leaves fields out of the comparison. Calls are matched by order. At a difference, the next calls of both sides (`--window`, 200 by default) are aligned to find where they agree again, so a missing call and a changed call show as separate divergences:

```text
compared 5 expected and 4 actual calls; the first 1 agree
divergence 1 at compared call 1 / 1:
    #0 Sprite.1(3, "BG01")
  - #1 MesWnd.3(12, 0)
  then they agree again (1 expected, 0 actual calls differ)
```

Lines with `-` are in the expected trace only, `+` in the actual one; `#n` is the call's place in its trace. `diff` exits with 1 when the traces differ, so a script can check a route against a trace kept from a good run. Ticks of two runs seldom line up (under Wine the original runs about 2.5 times slower), so against the original leave `tick` out, `pc` too unless the probe reads it, and `result` (see below).

## The file

One JSON document, with one call per line:

```json
{
  "format": "silky-trace",
  "version": 1,
  "producer": {"kind": "runtime", "name": "silky-run snap", "version": "0.1.0"},
  "game": "Doukyuusei",
  "notes": ["<game> --path 0,1 --lines 300 --trace run.json"],
  "calls": [
    {"script":"START.MES","pc":4096,"call":"Sprite.1","args":[3,"BG01"]},
    {"tick":1520,"script":"FK001.MES","pc":4120,"call":"Game.20","args":[30],"result":1}
  ]
}
```

| Field | Meaning |
| --- | --- |
| `producer.kind` | `runtime` (this project) or `original` (the game's program) |
| `game` | The game as the runtime names it |
| `notes` | How the run was made; not compared. The game folder is written as `<game>` |
| `call` | `Class.sub` for an engine function (`Game.n` for class 29, the number for a class without a name); `message` and `smessage` with the message number; `options` with the options' values; `choose` with the index taken |
| `args` | The statement's values, oldest first, without the sub-command; numbers and strings |
| `result` | What the call left on the stack for the script, if anything |
| `tick`, `script`, `pc` | The host's clock, the running script and the offset of the instruction; left out where unknown |

A file of another `format` or `version` is refused.

## The original program

`tools/trace/probe.py` runs the game's program under the Win32 debug API (on Windows, or under Wine with a 32-bit Windows Python), stops at the addresses a spec names, reads the call from registers and memory, and writes the same file with `producer.kind` `original`. The only write into the program is the breakpoint byte, put back after each hit.

```sh
wine python.exe tools/trace/probe.py --exe doukyuusei.exe --cwd <game folder> \
    --spec douk.json --out original.json --seconds 300 --arm-after 30 --game Doukyuusei
```

The spec is kept outside the repository. Each breakpoint names its address in the program as linked (`image_base`, 0x400000 by default), the call, and expressions for the sub-command, the arguments and optionally the script and pc. An expression is registers, numbers, `+` and `-`, and `[...]` for the 32-bit value at an address; an argument's `type` is `int`, `str` (a NUL-ended string in `encoding`, CP932 by default) or `wstr` (UTF-16):

```json
{
  "name": "doukyuusei 1.0.4.0",
  "breakpoints": [
    {
      "at": "0x...",
      "call": "Game",
      "sub": "[esp+4]",
      "args": [],
      "by_sub": {"24": [{"read": "[[esp+8]+0x10]", "type": "int"}]}
    }
  ]
}
```

The addresses come from the build's address table in the game's page (for example the `Game` command dispatch in [games/doukyuusei.md](games/doukyuusei.md)), and the argument expressions from that handler's code. `--arm-after` lets the start-up run at full speed before the breakpoints go in; `--tick` names a frame counter to read at each call. The probe reads at the call's entry, so it writes no `result`.

The probe has not yet been run against a game: the first spec for each build still has to be written from its program.
