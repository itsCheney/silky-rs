# 先輩が私の妄想にドージンする

English | [简体中文](doujin.zh-CN.md)

The runtime knows the game by `doujin.exe` in its folder (`Title::Doujin`). Program version 1.0.1.0, linked 2024-08-30 07:40:15 UTC with Visual Studio 2019 (linker 14.29), image base 0x400000. Size and SHA-256 not recorded yet: the copy read was not the file as shipped. The program ships inside a disc-check wrapper (a separate small program the game's exe starts from); the addresses here are those of the program inside the wrapper, as linked. Engine profile: [engine-2024-08](../builds/engine-2024-08.md). Its own `Game` commands: [game-commands.md](../game-commands.md#先輩が私の妄想にドージンする).

All scripts decode. Headless, `silky-run snap --path 0` plays from the start to an ending: about 6160 message lines through ten chapter scripts. The game adds its own commands to the engine (native class 29); which of them return a value comes from the executable.

## Current state

- Selections: before each option the script tests a flag, to hide routes already taken. The runtime gathers those options into one selection and answers the option's own value + 1, as the scripts expect.
- Engine calls are all carried out, except `MesWnd.3` and `Sprite.5` (once each).
- The game's own commands are not carried out yet: `Game.13.18`, the 1006 effects, the 40 doujinshi pages, 1007, 16, 10, 1005, 12, 1003, 961, 993 and 14.
- `Game.40` subs 0 and 1 and `Game.1006` sub 0 leave a value for the script. The module gives 0 until they are carried out.
- Movies: the game's movies (`do-jin*.dat`) are ASF files (VC-1 video, WMA Pro sound). The runtime decodes MPEG-1 only, so they are not played yet.
- Interface: the common title and message window are drawn. The game's own title menu, name plate and button bar are not drawn yet.

## Open items

- Carry out the game's own commands listed above, starting with the doujinshi viewer (`Game.40`) and the animation layers (`Game.1006`).
- Decode the movies (VC-1 and WMA Pro in ASF).
- Draw the game's own title menu, name plate and button bar.
- Carry out `MesWnd.3` and `Sprite.5`.
- The meaning of the codes returned by `Game.20`, `Game.110` and the screens `Game.30` to `Game.32` is not known yet.
- Check the runtime against the original program under Wine.

## Where things are in the executable

Addresses in `doujin.exe` of the build above, as linked. The code itself does not name them.

| What | Address |
| --- | --- |
| `Game` dispatcher (`CFuncExec` slot 0) | 0x494ac0 |
| `CFuncExec` vtable | 0x50b158 |
| Command 10 to 110: byte table, jump table | 0x495564, 0x495520 |
| Command above 910: byte table, jump table | 0x4955fc, 0x4955cc |
| `Game.10`, portrait set (subs 4 to 16) | 0x495640 |
| `Game.11`, name plate | 0x494b58 |
| `Game.12`, background and difference pictures | 0x495f60 |
| `Game.13`, standing characters | 0x4967b0 |
| `Game.14` | 0x494c72 |
| `Game.16`, second portrait set (subs 6 to 12) | 0x495b90 |
| `Game.20`, selection (result from 0x480750) | 0x494d7c |
| `Game.30` to `Game.32`, CG, scene and music modes | 0x498580 |
| `Game.35` | 0x499650 |
| `Game.40`, doujinshi screen | 0x49a140 |
| `Game.110`, title menu (built by 0x492780) | 0x494e64 |
| `Game.961` | 0x499820 |
| `Game.990`, staff roll | 0x495047 |
| `Game.993` | 0x497780 |
| `Game.1003` | 0x495142 |
| `Game.1005`, timed wait | 0x497f20 |
| `Game.1006`, animation layers (byte table 0x497f08, jump table 0x497ef4) | 0x497ad0 |
| `Game.1007`, voice-linked pictures | 0x49516d |
| `Game.1009` | 0x495228 |
| `Game.1010` | 0x4983b0 |
| Config flag test used by 10, 12, 16, 993, 1006 and 1007 | 0x423ee0 |
| Message window list (singleton) | 0x49d600 |
| Game object holder (singleton) | 0x49a5f0 |
