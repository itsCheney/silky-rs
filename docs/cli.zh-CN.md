# 命令行

[English](cli.md) | 简体中文

## silky-tool

```sh
silky-tool info  <游戏目录>                      # 封包与文件数量
silky-tool check <游戏目录> [--images]           # 解码全部脚本、MAP 和图片
silky-tool dis   <游戏目录> FK001.MES            # 反汇编脚本
silky-tool map   <游戏目录> FK001.MAP --section 1
silky-tool akb   <游戏目录> TITLE_BG_01.AKB title.png
silky-tool ls      <封包.arc>
silky-tool extract <封包.arc> <输出目录>
```

## silky-run

```sh
silky-run trace <游戏目录>                           # 启动过程，直到标题菜单
silky-run trace <游戏目录> --result 29.110=1         # 在标题菜单选择“开始”
silky-run trace <游戏目录> --result 29.110=1 --text-only --lang zh-hans --max-calls 50000
silky-run trace <kimaten2 目录> --result 29.110=1 --result 29.42=1,2 --result 29.43=1 --text-only
```

```sh
silky-run snap <游戏目录> --out snaps --font DejaVuSans.ttf --lines 0,10,50
silky-run snap <游戏目录> --lang 1 --font msyh.ttc --lines 170
silky-run snap <游戏目录> --lines 99999999 --path 0,1,1   # 把一条路线玩到结局
```

`snap` 用 `silky-runtime` 运行游戏，把标题画面和指定的对话行（从游戏第一行开始计数）保存为 PNG。字体不属于游戏文件，需要用 `--font` 传入一个或多个字体，按回退顺序排列；不提供字体时画面中不绘制文本。`--path` 按顺序给出前几个选项的选择（之后的选项按 `--choose`）；指定它时，游戏回到标题画面即停止运行，并且会保存每段视频结束后停留的画面。在《ふゆから、くるる。》中，`--path 1` 和 `--path 0,0` 通往两个坏结局，`--path 0,1,0` 和 `--path 0,1,1` 通往真结局，包括片尾视频和结束画面。在《きまぐれテンプテーション2》中，`--actions` 指定前几个行动菜单要选的按钮（之后的菜单轮流选择尚未完成的按钮），地图则前往第一个尚未完成的地点；前五个菜单和地图会保存为 `actionsN.png` 和 `mapN.png`。本作的调查需要在选项中选对答案，`--path` 和 `--choose` 并不知道答案，因此可能会在某个调查中绕圈。

`trace` 不执行任何引擎函数，只记录每次调用及其参数。脚本会用到返回值的调用默认返回 0，可以用 `--result 类.子命令=值` 指定；给出多个值时（`29.42=1,1,2`），后续的调用依次取用，最后一个值一直保留。选项（`Game.20`）由 `--choose` 决定，从 0 开始计数（默认第一个）。`--lang` 可取 `en`、`zh-hans`、`zh-hant` 或 `ja`（0、1、2 仍表示 `.MAP` 的分区）；每个游戏只提供它有文本的语言：《ふゆから、くるる。》为英文和简繁中文，《きまぐれテンプテーション2》为日文。在《きまぐれテンプテーション2》中，`29.42` 回答行动菜单，`29.43` 回答地图。
