# Game commands

English | [简体中文](game-commands.zh-CN.md)

The commands of native class 29 (`Game`) that silky-rs carries out. A call is written `Game.command.sub(arguments)`; the sub command is left out when a command has none. "Returns" is what the call leaves on the script's stack. The last column says how sure the behaviour is (see [engine.md](engine.md#how-sure-each-part-is)).

## Common set

Commands of ふゆから、くるる。 general enough to try on a game not known yet (`games/mod.rs`). きまぐれテンプテーション2 uses 11, 12, 20, 30 to 32 and 110 from it.

| Call | Returns | Does | Source |
| --- | --- | --- | --- |
| `10.6(body)`, `10.7(face)` | | Message window portrait | Compared |
| `10.9`, `10.11` | | Show, hide the portrait | Scripts |
| `10.14` | | Erase the message window | Scripts |
| `11(row)`, `11(name)` | | Speaker name plate: a row of the sheet, or by name in Japanese builds | Compared |
| `12.0(name)` | | Background | Scripts |
| `12.1(slot, parameter, name)` | | Difference picture in a slot; an empty name removes it | Scripts |
| `12.2`, `12.11` | | Set up and close a picture change | Read |
| `12.3(time)` | | Crossfade to the new picture | Read |
| `12.4(y, x)`, `12.5(percent)`, `12.6` | | Close-up: move, enlarge, back to the whole picture | Read |
| `13.0(time, size, name, slot)` | | Standing character | Compared |
| `13.14(x, y, time, size, name, slot)` | | Standing character off its slot position | Read |
| `13.1(time, slot, face)` | | Expression | Scripts |
| `13.3(time, slot)`, `13.4(time)` | | Remove one, remove all | Scripts |
| `13.10(time, new slot, slot)` | | Move to another slot | Read |
| `13.7(pixels, kind, slot)` | | Hop or shake; the step time is a guess | Read, guess |
| `13.2` | | Apply the character changes over the longest time given | Read |
| `20(question line, style)` | option, from 1 | Selection with labels from the text table | Scripts |
| `20(style, question)` | option, from 1 | Selection in Japanese builds, labels from the option bodies | Scripts |
| `23.2(table)` | 0 | Load a `.map` text table | Scripts |
| `23.0(line)` | | Show a line and wait | Compared |
| `30`, `31`, `32` | result of the screen | CG, scene and music modes | Read |
| `110` | item picked | Title menu | Compared |
| `223(name)` | name | Localized file name (`x.akb` to `x_en.akb` when that exists) | Scripts |

## ふゆから、くるる。

`games/fuyukuru/commands.rs`.

| Call | Returns | Does | Source |
| --- | --- | --- | --- |
| `14` | | Auto save point | Scripts |
| `111.rule(colour, time)` | | Fade out to a colour through a rule mask | Read |
| `112.mode(time)` | | White wipe into (0) and out of (1, 2) the future scenes; the picture does not shrink here | Read |
| `113` | | Heartbeat zoom | Read |
| `114` | | Slash flash | Read |
| `115.0`, `115.1` | | Snow on, off | Read |
| `993.0`, `993.2(size)`, `993.3`, `993.1` | | Screen quake: set up, size, start, stop | Read |
| `1005.0` | | Zoom and white flash | Read |
| `1007.0(file)`, `1007.1` | | Pictures that change with the voice line (`.vo`), end | Read |
| `13.15(time, pixels, across, slot)` | | Character swing | Read |
| `10.4`, `10.5` | | Scene skip point, setting off: nothing to do | Read |
| `12.7`, `961.1`, `116.1`, `1003` | | Set ups the runtime does not need | Read |
| `1006.11` | 0 | Clear an animation list no script fills | Read |

`1009` returns a value and is not carried out yet.

## きまぐれテンプテーション

`games/kimaten1/commands.rs`. The commands not listed here or in the common set are reported. Those that return a value were read from the executable: 30 to 33, 41, 42, 43, 45, 46, 110, 990 and 1009 always, 44 (sub 4) and 1006 (sub 0) for one sub only. 1001, 1002, 989 and the other subs of 35 are not carried out yet. See [games/kimaten1.md](games/kimaten1.md).

| Call | Returns | Does | Source |
| --- | --- | --- | --- |
| `10.6(file, file)` | | E-mote model of the companion: a costume's two files, fading in | Read |
| `10.7(extra, face)`, `10.8`, `10.13(pose)`, `10.16(timeline)`, `10.17` | | Face, extra timeline, pose, `a` timeline, following pose | Read |
| `10.9`, `10.11`, `10.12` | | Show, fade out and remove, remove | Read |
| `10.4`, `10.5` | | Scene skip point, setting off: nothing to do | Read |
| `10.10`, `10.14` | | Wipe the message window's text; take the window away | Read |
| `10.15` | | Hold the script two seconds (the notebook's unlock); the bar's blinking is not drawn | Read |
| `10.18` | | Acts on a part of the window nothing reaches: nothing to do | Read |
| `11`, `12`, `13` | | Speaker, picture, character: as the common set (`13` subs 5, 6, 8, 9, 11, 12 do nothing) | Read |
| `14` | | Auto save point | Read |
| `30`, `31`, `32` | -1, or the scene number | CG, scene and music modes; -1 when left | Read |
| `33` | -1 | Anne viewer | Read |
| `35.2` | | Nothing to do | Read |
| `40.0(frames, size, file, file, slot)`, `40.12(…)` | | Standing E-mote model in a slot | Read |
| `40.1(slot, extra, face)`, `40.15(pose, slot)`, `40.14(timeline, slot)`, `40.13`, `40.17(slot, extra)` | | Face, pose, `a` timeline, following pose, extra timeline | Read |
| `40.8(dy, dx, slot)` | | Move a model | Read |
| `40.3(frames, slot)`, `40.4(frames)` | | Remove one, remove all | Read |
| `40.16(file)`, `40.21`, `40.31` | | Mouth track of the voice; stop the mouth; clear the cache of mouth tracks | Read |
| `40.2` | | Crossfade to the new models over the longest time the other calls gave | Read |
| `40.18` | | Pause the driver | Read |
| `40.100(slot, frames)` | | White flash: the old model fades out as the new one fades in | Read |
| `41(number, file)` | hotspot, or 100 | Investigation over the background (`.carea`) | Read |
| `42(skip)` | kind + 1, or 100 | Action menu from bytes 0 to 3; argument 1 allows the skip button | Read |
| `43(back)` | room 1 to 6, or 100 | Map of the apartment; argument 1 shows the back button | Read |
| `44.0(text)`, `44.1`, `44.2`, `44.3` | | Notices: queue a string, show the queue, take the strips down, clear the queue | Read |
| `44.4` | 0 or 1 | Whether strips are still up | Read |
| `45` | option + 1, or 100 | Option list of the registered options | Read |
| `46` | piece from 1 | Choice of evidence from bytes 224, 231, 212, 213 and 138 | Read |
| `47.0(card)`, `47.1(first, second)` | | Evidence cards: fade in, stay, fade out | Read |
| `48.0(file)`, `48.1`, `48.15`, `48.2`, `48.3(frames)` | | E-mote CG model: load, face, main timeline, remove, fade out | Read |
| `110(page)` | button | Title menu; argument 1 opens the extras page | Read |
| `961.0(channel, file)`, `961.1(channel)` | | Long voice that the next message does not cut; stop it | Read |
| `990(true end)` | 1 | Staff roll; 1 as argument after the true ending | Read |
| `993.0`, `993.2(mode)`, `993.3`, `993.1` | | Screen shake: reset, set the mode, start, stop | Read |
| `999.0` to `999.3` | | Picture over the screen: fade in, fade out, show, put away | Read |
| `1000(hold, frames, rule, height, width, y, x)` | | Strip of text wiped in through a rule picture | Read |
| `1003`, `1006.11`, `1010`, `1011`, `10008`, `10009` | | Set ups the runtime does not need | Read |
| `1006.0` | 0 | | Read |
| `1007.0(file)`, `1007.1` | | Changes of models a set time into a voice line (`.vo`); drop what is left | Read |
| `1009` | 0 | | Read |

## きまぐれテンプテーション2

`games/kimaten2/commands.rs`. The commands not listed here or in the common set are reported. Those that return a value were read from the executable: 24, 34, 44, 959, 990, 1006, 1009 and 1010 are not carried out yet.

| Call | Returns | Does | Source |
| --- | --- | --- | --- |
| `10.6(file, file)` | | E-mote model of the companion: a costume's two files | Read |
| `10.7(extra, face)`, `10.12`, `10.13(pose)`, `10.16(timeline)` | | Face, pose, `a` timeline | Read |
| `10.9`, `10.11` | | Show, hide | Read |
| `10.4` | | Scene skip point, setting off: nothing to do | Read |
| `10.17(number)` | | A number kept on the model, not used | Read |
| `40.0(frames, size, file, file, slot)`, `40.12(…)` | | Standing E-mote model in a slot | Compared |
| `40.1(slot, extra, face)`, `40.15(pose, slot)`, `40.14(timeline, slot)` | | Face, pose, `a` timeline | Read |
| `40.8(dy, dx, slot)` | | Move a model | Read |
| `40.3(frames, slot)`, `40.4(frames)` | | Remove one, remove all | Read |
| `40.16(file)` | | Mouth track of the voice | Read |
| `40.2` | | Crossfade to the new models over the longest time the other calls gave | Read |
| `51(mode)` | 0, 1, 2 | Costume screen; mode 1 adds 次へ (answers 2) | Read |
| `42` | button, or -1 | Action menu from bytes 0 to 6 | Compared |
| `43(voice set, back, which map)` | place | Map | Compared |
| `49` | | H mode screen | Read |

## なないろリンカネーション

`games/nanairo/commands.rs` and `effects.rs`. The commands not listed here or in the common set are reported. Those that return a value were read from the executable: 18, 30 to 32 and 34. 34 is not carried out yet. See [games/nanairo.md](games/nanairo.md).

| Call | Returns | Does | Source |
| --- | --- | --- | --- |
| `10` | | Start up of the game's library; nothing to do | Read |
| `12.4(face, body)` | | Face window: the body picture, with the face drawn over it when it differs | Read |
| `12.5`, `12.6`, `12.7`, `12.8(picture)` | | Show, hide, empty the face window; show a whole picture | Read |
| `13.0(time, size, name, slot)` | | Standing character | Read |
| `13.1(time, slot)`, `13.7(time)` | | Remove one, remove all | Read |
| `13.5(flag, slot, name)` | | Expression layer; what the flag changes is not known | Read |
| `13.9(size, slot)` | | Zoom to a size, here at once | Read |
| `13.2` | | Crossfade every change over the longest time given | Read |
| `13.6`, `13.8`, `13.11` to `13.15` | | Side, effects, flags, motion: accepted, not drawn | Read |
| `14.10(name)`, `14.1(slot, name)` | | CG registry: the picture, a layer of it | Read |
| `14.11`, `14.5`, `14.7(time)` | | Register the combination | Read |
| `14.4`, `14.2`, `14.3` | | Start of a scene picture, clearing name tables: nothing to keep | Read |
| `15(name)` | | Speaker name | Read |
| `18` | item picked | Title menu: 1 new game, 5 quit, 257, 258, 259 CG, scene, music | Read |
| `19(value, end, start)` | | Fill byte flags `start` up to `end` | Scripts |
| `22.0` | | Register the picture shown | Read |
| `26` | | End of chapter: empties the message window | Read |
| `27` | | Message window start up | Read |
| `22.1`, `22.2` | | Sparkle picture (`kirara.akb`): put up clear; flash out over 16 frames, growing to 1.2 times about (640, 360); the script waits | Read |
| `23.0(from, to, _)` | | Moves the picture in sprite 2 sideways a pixel a frame from window position `from` to `to`; the script waits. The direction is assumed | Read |
| `1001.0`, `1001.1` | | Event picture: grows to 1.2 times over 16 frames, an effect picture fades in and repeats; the end fades both out over 24 frames. Both wait | Read |
| `1002.0(name, frames)`, `1002.1(frames)` | | Scene movie fades in and out, the script waits. Movies are not decoded; the picture of the same name stands in | Read |
| `1002.3(name, frames)`, `1002.4(frames)` | | The same for an effect movie; nothing is drawn, the time passes | Read |
| `1003.0(name, frames)`, `1003.1(frames)` | | Effect picture fades in and out, the script waits | Read |
| `1005.0`, `1005.1` | | Effect picture fades in over 64 frames, the script waits a round of its pulse; the end removes it | Read |
| `1006.3`, `1006.13`, `1006.20` | | Event animator: swaying lines over a figure picture; a glowing figure over a beam of light. Not waiting. Other subs are reported | Read |
| `1001.3`, `1002.2`, `1003.2`, `1005.2`, `1006.100` | | Clean up: take the effect away | Read |
| `39`, `50`, `1004`, `1007` to `1009` | | Accepted, nothing drawn or started | Read |

## 先輩が私の妄想にドージンする

`games/doujin/`. Read from the executable; no command here is carried out yet. Those that return a value are 20, 30, 31, 32, 110, 990 and 1009 always, and 40 subs 0 and 1 and 1006 sub 0 only. The module gives 0 for the last three until they are carried out. Every other call leaves nothing on the stack.

| Call | Returns | Does | Source |
| --- | --- | --- | --- |
| `20(a, b)` | A number | A selection: hands the two values to the game's screen object | Read |
| `30`, `31`, `32` | A code, or -1 | CG, scene and music mode screens; -1 when the screen failed or was left | Read |
| `110(state)` | The picked item | The title menu | Read |
| `990(value)` | A number | The staff roll | Read |
| `1009` | 0 or 1 | 1 when the first message window entry has bit 14 of its flags set | Read |
| `40.0(a, b)`, `40.1(a, b)` | A number | Asks the doujinshi screen object for a result (page turn or choice, not known) | Read |
| `1006.0(file, a, b, c, d, e)` | A handle | Creates an entry in the animation layer list from a `.brt` file | Read |

The codes returned by 20, 30 to 32, 110 and 990 are not known yet.
