# Adding a game

English | [简体中文](adding-a-game.zh-CN.md)

How to bring another Silky Engine game to silky-rs. The engine is shared, so most of a new game runs as it is; the work is in its own `Game` commands and screens.

## 1. Run it as it is

Point the runner at the folder:

```sh
silky-run snap <game folder> --out snaps --font <font> --lines 0,10,100
```

A folder without a known program file runs as a game not known yet: the engine classes and the common `Game` commands work, and every call nobody handles is listed at the end with how often it came. The list is the to-do list. If the scripts or images do not decode, the formats come first (`silky-tool` inspects archives, scripts and images, see [cli.md](cli.md)).

## 2. Give it a name

Add a variant to `Title` in `crates/silky-format/src/game.rs` and detect it by its program file in `Title::detect`. Then add a module `crates/silky-runtime/src/games/<game>/` with a `commands.rs`, and send the game's calls to it in `Runtime::game` (`games/mod.rs`), as the two games there do.

In `silky_vm::native::returns_value`, list the game's `Game` commands that leave a value on the stack. Read them from the executable: a handler that pushes a result. A wrong list breaks the script's stack, so this comes before anything else.

## 3. Its commands

For each command in the list from step 1, find out what it does from the scripts that call it and, where needed, from the executable, then:

- Use the common set when the command behaves the same way, and add the number to the game's list of shared commands (as `kimaten2::SHARED_COMMANDS` does). Same number does not mean same command, so check the scripts first.
- Otherwise handle it in the game's `commands.rs` and return `None` for anything not understood, so it is reported.
- Add it to [game-commands.md](game-commands.md) with what it takes, what it returns, what it does and how sure that is.

Never handle a call just to make it go away. A call that really needs nothing (a set up of something this runtime does differently) can be counted as done, with a comment saying why.

## 4. Its page

Add `docs/games/<game>.md` and its Chinese copy: the build tested, the SHA-256 of its program file, what works and what is left, and the executable addresses behind what was read from it. Add the game to [games.md](games.md). Game data never goes into the repository; tests build their small files in code.

## 5. Check the others

A change to the engine or the common set touches every game. Before committing, run `cargo test --workspace`, and play the known games again with `silky-run snap`: on ふゆから、くるる。 `--path 0,1,1 --lines 99999999` must reach the ending with no call left undone.
