# Features

English | [简体中文](features.zh-CN.md)

What the engine side of silky-rs does for every game. What each game adds on top is in [games.md](games.md).

## Files and scripts

- `.arc` archives, including the LZSS compression and the override order from `Patch.xml` or `FileInfo.xml` (`script.arc;script1.arc`: the later archive wins). `update.arc` is read as the newest data archive.
- AKB images, including `AKB+` difference images on top of a base image.
- `.MAP` text tables (one section per language).
- `.MES` scripts and `LIBLARY.LIB`: header, offset tables and the full byte code decoder. Japanese builds keep their text in the scripts (line breaks by the `text` opcode).
- `.VSD` movie containers (an MPEG-1 program stream inside).
- The script VM: stack, variable banks, functions, script calls and selections. Engine functions go to a host.

## Runtime

- Pictures: backgrounds, event pictures with their difference pictures, standing characters with expressions, the message window with its name plate, portrait and text, localized file names, the title picture.
- The engine-drawn interface: title menu buttons (layout taken from the executable, per language), the button bar and icons of the message window (`mes_o.area`), the click-wait indicator, and selections (`Game.20`).
- Sound: music (looping at the `smpl` loop points of the WAV files), sound effects and voices, mixed in software; the desktop player plays them with cpal.
- Timing and transitions: timed waits (a click skips them), fades to and from a colour, crossfades of pictures and characters, rule wipes with the game's `NN.bin` masks, screen shake, and waits for a sound to end. Times are read as frames of 1/60 s for now.
- The save and load screen as the engine draws it (from `saveload.area` and its sheets): save, auto save and quick save tabs, thumbnails, dates, the newest-save badge, overwriting, deleting one or all saves, help lines from `saveloadhelp.map`, and the yes/no dialog with "do not ask again". An auto save is made when a selection comes up; Continue opens the newest save. Saves are this runtime's own JSON files, with a PNG thumbnail each.
- The backlog (`backlog00.area`): the last 300 messages with their nameplates, replaying a line's voice, playing marked voices one after another, and jumping back to a message.
- The settings screen (`config00.area`, `config01.area`), opened from the title menu, the window's settings button or a right click: text speed and letter-by-letter text with an optional pause after punctuation, skip (read text only, speed, keep after selections), auto mode, message window colour and opacity, effect speed, which questions to ask, what a right click does, the language (switched while playing), and the volumes of all sound, music, effects, voices and each character's voice. Help lines come from `confighelp.map` and `soundconfighelp.map`.
- The extras of the title menu: the CG gallery (`cgmode.area`, nine pages of thumbnails, click through the pictures of an entry), scene replay (`scmode.area`) and the music room (`soundmode.area`, with a progress bar). What unlocks them is kept with the settings in `system.json` in the save folder: pictures and music met in the story, and lines read. The tables of gallery pictures, scenes and tracks are taken from the executable of ふゆから、くるる。.
- Movies (`.vsd`): MPEG-1 video and MP2 sound decoded with [pl_mpeg](https://github.com/phoboslab/pl_mpeg) (MIT, in `crates/silky-runtime/vendor`); the picture follows the sound clock, and a click ends the movie.
- Output size apart from the game's 1280 x 720 layout: pictures are composed at the game's size and scaled once, text is drawn at the output size (the player's window height, 1080 for `silky-run snap` by default, `--height` to change), so it stays sharp. Clicks and areas stay in game pixels.
- The window's previous choice button (back to the message before the last selection) and next choice button (skip to the next selection, through unread text too).
- Close-ups and character moves, read from the engine's handlers: event pictures moved and enlarged (`Game.12` subs 4 and 5), characters that hop or shake (`Game.13` sub 7; the step length of 50 ms is a guess), change slot (sub 10) or stand off their slot position (sub 14).
