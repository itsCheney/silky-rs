# 调用记录（对拍）

[English](trace.md) | 简体中文

调用记录（trace）是一次运行按顺序发出的引擎调用：每次 `native` 的类、子命令和参数，每条对话和选项消息，以及每组选项和所选的选项。对同一剧情位置的两份记录可以逐条比较，从而看出本运行时与原版程序（或本运行时的两个版本）从哪里开始不同，而不必比对截图。

## 生成记录

```sh
silky-run trace <游戏目录> --result 29.110=1 --out stub.json    # 引擎调用由桩函数应答
silky-run snap  <游戏目录> --path 0,1 --lines 300 --trace run.json
silky-player    <游戏目录> --trace play.json                    # 关闭窗口时写出
```

`snap` 和 `silky-player` 运行真正的运行时，因此记录中是运行时自己给出的应答。播放器的调用以会话时钟（毫秒）作为 `tick`；`trace` 和 `snap` 没有时钟，不写这一项。运行因错误中止时也会写出记录，直到出错的那次调用为止。

## 比较

```sh
silky-run diff expected.json actual.json
silky-run diff original.json run.json --ignore tick,pc,result --skip MesWnd,Wait --from message:120 --max 5
```

第一个文件视为正确的一方。两份记录经过同样的筛选：`--only` 和 `--skip` 接受类（`Sprite`）、调用（`Game.24`）或种类（`message`）；`--from` 让每份记录从第一个匹配的调用开始，可带第一个参数（`message:120`、`Script.0:FK002.MES`）；`--ignore` 把字段排除在比较之外。调用按顺序对应。遇到不同之处时，会把双方之后的调用（`--window`，默认 200）对齐，找出重新一致的位置，因此缺少的调用和改变的调用会作为不同的分歧分别列出：

```text
compared 5 expected and 4 actual calls; the first 1 agree
divergence 1 at compared call 1 / 1:
    #0 Sprite.1(3, "BG01")
  - #1 MesWnd.3(12, 0)
  then they agree again (1 expected, 0 actual calls differ)
```

`-` 行只在期望的记录中，`+` 行只在实际的记录中；`#n` 是该调用在各自记录中的位置。两份记录不同时 `diff` 以 1 退出，因此脚本可以用一份正常运行时留下的记录来检查一条路线。两次运行的 tick 很少能对上（在 Wine 下原版大约慢 2.5 倍），所以和原版比较时排除 `tick`，探针不读 `pc` 时也排除 `pc`，并排除 `result`（见下文）。

## 文件

一个 JSON 文档，每行一个调用：

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

| 字段 | 含义 |
| --- | --- |
| `producer.kind` | `runtime`（本项目）或 `original`（游戏自己的程序） |
| `game` | 运行时对该游戏的称呼 |
| `notes` | 这次运行是怎样生成的；不参与比较。游戏目录写作 `<game>` |
| `call` | 引擎函数为 `类.子命令`（类 29 为 `Game.n`，没有名字的类写编号）；`message` 和 `smessage` 带消息编号；`options` 带各选项的值；`choose` 带所选的序号 |
| `args` | 语句中的值，先压入的在前，不含子命令；数字或字符串 |
| `result` | 调用留在栈上给脚本的值（如果有） |
| `tick`、`script`、`pc` | 宿主的时钟、正在运行的脚本和指令偏移；不知道时省略 |

`format` 或 `version` 不同的文件会被拒绝。

## 原版程序

`tools/trace/probe.py` 通过 Win32 调试 API 运行游戏程序（在 Windows 上，或在 Wine 中使用 32 位 Windows 版 Python），在 spec 指定的地址停下，从寄存器和内存读出调用，写出同样格式、`producer.kind` 为 `original` 的文件。对程序唯一的写入是断点字节，每次命中后立即恢复。

```sh
wine python.exe tools/trace/probe.py --exe doukyuusei.exe --cwd <游戏目录> \
    --spec douk.json --out original.json --seconds 300 --arm-after 30 --game Doukyuusei
```

spec 放在仓库之外。每个断点给出它在程序中的链接地址（`image_base`，默认 0x400000）、调用名，以及子命令、参数和（可选的）脚本与 pc 的表达式。表达式由寄存器、数字、`+`、`-` 和表示该地址处 32 位值的 `[...]` 组成；参数的 `type` 为 `int`、`str`（按 `encoding` 解码、以 NUL 结尾的字符串，默认 CP932）或 `wstr`（UTF-16）：

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

地址来自游戏页面中该版本的地址表（例如 [games/doukyuusei.zh-CN.md](games/doukyuusei.zh-CN.md) 中的 `Game` 命令分派），参数表达式来自对应处理函数的代码。`--arm-after` 让启动过程全速运行，之后再下断点；`--tick` 指定每次调用时读取的帧计数器地址。探针在调用入口读取，因此不写 `result`。

探针还没有在游戏上实际运行过：每个版本的第一份 spec 仍需根据其程序编写。
