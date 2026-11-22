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

The first file is taken as right. Both go through the same filter: `--only` and `--skip` take classes (`Sprite`), calls (`Game.24`) or kinds (`message`); `--from` starts each trace at its first call that matches, with an optional first argument (`message:120`, `Script.0:FK002.MES`); `--ignore` leaves fields out of the comparison; `--show` sets how many calls of each side a divergence lists. Calls are matched by order. At a difference, the next calls of both sides (`--window`, 200 by default) are aligned to find where they agree again, so a missing call and a changed call show as separate divergences:

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
wine python.exe tools/trace/probe.py --exe fuyukuru.exe --cwd <game folder> \
    --spec fuyukuru-2026.json --out original.json --seconds 120 --game Fuyukuru
silky-run diff original.json run.json --ignore tick,script,pc,result \
    --skip message,smessage,options,choose
```

The probe sees only `native` calls, so messages and options are left out of the comparison.

One breakpoint is enough for every engine call: the place in the engine's `native` handler right after it pops the class number, where the statement's values are still on the script's value stack. The spec gives that address (as linked, `image_base` 0x400000 by default), where the class number is, the names of the class numbers, and the layout of the value stack:

| Field | Meaning |
| --- | --- |
| `breakpoints[].at` | Address to stop at |
| `breakpoints[].call` | The call's name, or `{"read": expr, "names": {"1": "Select", ...}}` to name it by a value read there |
| `breakpoints[].stack` | `true`: the arguments are the values on the stack, the last one being the sub-command |
| `stack.vector` | The stack object, a vector whose first two words are its begin and end |
| `stack.size`, `kind`, `int`, `str` | Size of one value, and offsets of its kind, its number and its string (an MSVC `std::string`) |
| `stack.kinds` | Which kind values are numbers and which strings: `{"0": "int", "1": "str"}` |
| `breakpoints[].only` | `{"read": expr, "is": [...]}`: keep only the hits where the value read is one of these, such as the return address of the script's own loop, since the engine runs some calls itself |
| `breakpoints[].sub`, `args`, `by_sub`, `script`, `pc` | Instead of `stack`: expressions for each value, with an argument `type` of `int`, `str` or `wstr` |

An expression is registers, numbers, `+` and `-`, and `[...]` for the 32-bit value at an address. A breakpoint with a plain `call` name and no arguments marks a place, such as the start and end of a routine to time. `--arm-after` lets the start-up run at full speed before the breakpoints go in; `--tick` names a frame counter to read at each call, and `--clock` writes the milliseconds since the start instead. `--live FILE` also writes each call to a file as it happens, one per line, for a script that drives the game (clicking, or skipping with Ctrl until some call comes) to watch. The probe stops before the call runs, so it writes no `result`. The addresses and layout for each build are in its page under [builds/](builds/README.md).

`diff` counts a block of up to 8 calls repeated right after itself once (`--no-squash` keeps them): the scripts wait for sounds and fades in loops that run as often as the wait takes, which differs between runs and under Wine. The scripts also pick some sounds at random, so a call's string can differ between two runs of the original too.

A handler that does not take all of a statement's values leaves the rest on the stack, under the next statements' values, until the script returns. The probe reads the whole stack, so the original's calls then carry those values in front.

On ふゆから、くるる。 (2026 build), the probe and `silky-run snap` agree on every call from the start to the first lines of the story, except the logo's wait loop, which the runtime leaves at once because headless runs play no sound, the random sound, and the title menu's buttons reacting to the mouse. On 同級生 (2022 build) they agree from the start to the title menu, apart from a value one call leaves on the stack (see the build page).
