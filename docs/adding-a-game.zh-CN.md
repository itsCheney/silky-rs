# 加入新游戏

[English](adding-a-game.md) | 简体中文

如何让另一款 Silky Engine 游戏在 silky-rs 上运行。引擎是共用的，所以新游戏的大部分内容可以直接运行；工作量在它自己的 `Game` 命令和画面上。

## 1. 直接运行

用运行器打开游戏目录：

```sh
silky-run snap <游戏目录> --out snaps --font <字体> --lines 0,10,100
```

没有已知程序文件的目录会作为尚未支持的游戏运行：引擎类和通用的 `Game` 命令可以工作，谁都不处理的调用会在最后连同出现次数一起列出。这个列表就是待办清单。如果脚本或图片无法解码，就先处理文件格式（`silky-tool` 可以查看封包、脚本和图片，见 [cli.zh-CN.md](cli.zh-CN.md)）。

## 2. 给它一个名字

在 `crates/silky-format/src/game.rs` 的 `Title` 中加入一项，并在 `Title::detect` 中根据它的程序文件识别。然后加入模块 `crates/silky-runtime/src/games/<游戏>/` 和其中的 `commands.rs`，并在 `Runtime::game`（`games/mod.rs`）中把这款游戏的调用交给它，做法与已有的游戏相同。

在 `crates/silky-format/src/engine.rs` 的 `BUILDS` 中加入它的程序，并指定它的引擎符合的配置（见 [builds/](builds/README.zh-CN.md)）。如果它的引擎类和所有配置都不同，就新建一个配置和对应的页面。

在 `silky_vm::native::returns_value` 中列出这款游戏会在栈上留下值的 `Game` 命令。这要从可执行文件中读出：看处理函数是否压入结果。列表错了会破坏脚本的栈，所以这一步要最先做。

## 3. 它的命令

对第 1 步列表中的每个命令，根据调用它的脚本，必要时再根据可执行文件，弄清它做什么，然后：

- 行为相同时使用通用命令，并把编号加入这款游戏的共用命令列表（如 `kimaten2::SHARED_COMMANDS`）。编号相同不代表命令相同，所以要先看脚本。
- 否则在这款游戏的 `commands.rs` 中处理，对不理解的调用返回 `None`，让它被报告出来。
- 在 [game-commands.zh-CN.md](game-commands.zh-CN.md) 中写下它接收什么、返回什么、做什么，以及可靠程度。

绝不能为了让调用消失而处理它。确实无需处理的调用（本运行时用别的方式实现的初始化）可以算作已执行，并用注释说明原因。

## 4. 它的页面

加入 `docs/games/<游戏>.md` 及其中文版：测试的版本、程序文件的 SHA-256、哪些能用、还剩什么，以及从可执行文件读出的内容所在的地址。在 [games.zh-CN.md](games.zh-CN.md) 中加入这款游戏。游戏数据不进入仓库；测试在代码中构造小文件。

## 5. 检查其他游戏

对引擎或通用命令的改动会影响所有游戏。提交前运行 `cargo test --workspace`，并用 `silky-run snap` 再玩一遍已有的游戏：在《ふゆから、くるる。》上，`--path 0,1,1 --lines 99999999` 必须走到结局，且没有未执行的调用。
