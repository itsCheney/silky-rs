# engine-2017

[English](engine-2017.md) | 简体中文

四个程序所用的引擎，都用 Visual Studio 2013 链接。除特别标注外，均从可执行文件读出。

| 程序 | 版本 | 链接时间（UTC） | SHA-256 |
| --- | --- | --- | --- |
| `apeiria.exe`（景の海のアペイリア） | 1.0.3.0 | 2017-11-09 07:06:49 | `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e` |
| `casablanca.exe`（景の海のアペイリア ～カサブランカの騎士～） | 1.0.0.0 | 2017-12-10 14:52:07 | `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716` |
| `kimaten.exe`（きまぐれテンプテーション，带更新 1.04） | 1.0.4.0 | 2019-12-16 04:48:49 | `36148a0286bd9d31ceaec4323f4be987319bd759e68b5961c59c992b07b54700` |
| `nanairo.exe`（なないろリンカネーション） | 1.0.0.2 | 2014-09-09 19:27:20 | 尚未记录 |

`casablanca.exe` 大小为 1,559,040 字节。它的引擎代码和 `apeiria.exe` 相同，所以下面的差异对两者都成立。trace 探针地址只给出 `apeiria.exe`、`casablanca.exe` 和 `nanairo.exe` 的。

`kimaten.exe` 大小为 2,039,808 字节。在检查过的几处（图层 5 上的 `Sprite.1`、`BackLog`、`Movie.3`、`HEffect`），它的引擎调用与下表一致；它的 `Game` 命令见 [kimaten1.zh-CN.md](../games/kimaten1.zh-CN.md)。

`nanairo.exe` 的文件大小尚未记录。它有返回值的引擎调用与下表一致；它的 `Game` 命令见 [nanairo.zh-CN.md](../games/nanairo.zh-CN.md)。

## 与 engine-2026 的不同

| 调用 | 本版本 | engine-2026 |
| --- | --- | --- |
| 图层 5 上的 `Sprite.1` | 隐藏创建 2048 像素见方的图层，存放按钮图集（`title_p.akb`），由游戏画面按格子绘制 | 显示图片 |
| 无参数的 `BackLog.3` | 结束该条目的语音标记 | 不这样调用 |
| `Movie.2` | 关闭影片，不返回值（脚本在每次换场后调用） | 播放影片，返回是否播放 |
| `Movie.3` | 返回值 | 不返回值 |
| `Movie.1` | (_, 文件)：开启影片时的动态背景（来自脚本）。运行时尚未解码它，暂时显示同名图片 | 不调用 |
| `BackLog.4` 到 `BackLog.7` | 返回值 | 不返回值 |
| `OneMes.5` | 返回值 | 不返回值 |
| `Save.3`、`Save.8` | 返回值 | 不返回值 |
| `HEffect`（类 17） | 在另一组音效通道上播放音效，子命令与 `Effect` 相同（来自脚本） | 未实现 |

## 调用记录探针

`apeiria.exe` 的链接地址（映像基址 0x400000），供 `tools/trace/probe.py` 使用（见 [trace.zh-CN.md](../trace.zh-CN.md)）。已在 Wine 下核对前几章。

| 内容 | 地址 |
| --- | --- |
| 脚本操作码分派（操作码 0x18 进入 `native` 处理函数，0x19 进入 `message`） | 0x4b2a95（字节表 0x4b2e34，跳转表 0x4b2d74） |
| `native` 处理函数；弹出类号，再按 类号 − 1 分派 | 0x49e0a0 |
| 断点：类号已弹出，语句的值仍在栈上 | 0x49e103（`eax` = 类号，`esi` = 脚本对象，开头就是它的值栈 vector） |
| 弹出值：复制栈顶的值，每个 0x20 字节；+0 为种类（0 数字，1 字符串），+4 为数字，+8 为 `std::string` | 0x4b2e90 |
| `message` 处理函数（在这里下断点可标出每条消息） | 0x4b2c00 |

这个版本的 spec 见英文页面。

在一个函数的开头和结尾各下一个断点就能给它计时：用 `--clock` 这样测出了全息面板显示（0x467eb0，结束于 0x4681ab）所用的时间（见 [apeiria.zh-CN.md](../games/apeiria.zh-CN.md)）。

### casablanca.exe

`casablanca.exe` 的链接地址（映像基址 0x400000）。函数和上面的相同，只是地址不同；栈的布局也相同。

| 内容 | 位置 |
| --- | --- |
| 脚本操作码分派 | 0x4b4445（字节表 0x4b47e4，跳转表 0x4b4724） |
| `native` 处理函数 | 0x49f9c0 |
| `native` 处理函数处的断点：类号已弹出，语句的值仍在栈上 | 0x49fa23（`eax` = 类号） |
| 弹出值 | 0x4b4840 |
| `message` 处理函数（在这里下断点可标出每条消息） | 0x4b45b0 |
| 类 16 `Effect`、17 `HEffect`、18 `Voice`：构造（类分支的 case）、共用的子命令分支 | 0x49fca3, 0x49fcdc, 0x49fd15；0x4adc70（跳转表 0x4add34） |
| `Effect` 和 `HEffect`：方法表、通道列表生成函数；音量读取函数（`HEffect`：`/Config/HEffect/Volume`，默认 90，乘 `/Config/Master/Volume`，换算为 -5000..0（1/100 dB）；小函数 0x4ae990、0x4aec50；音乐 0x494fb0 和语音 0x495190 用同一公式；运行时对 `casablanca.exe` 和 `apeiria.exe` 按增益 10^(att/2000) 处理） | 0x52ea14, 0x52b758；0x4ae700, 0x4ae9c0；0x495050, 0x4950f0 |

这个版本的 spec 同样见英文页面。

### nanairo.exe

`nanairo.exe` 的链接地址（映像基址 0x400000，版本 1.0.0.2）。函数和上面的相同，只是地址不同；栈的布局也相同（每个值 0x20 字节，+0 为种类，+4 为数字，+8 为 `std::string`，值栈是脚本对象开头的 vector）。尚未在 Wine 下运行过。

| 内容 | 位置 |
| --- | --- |
| 脚本操作码分派（操作码 0x18 进入 `native` 处理函数，0x19 进入 `message`） | 0x492815（字节表 0x492ba4，跳转表 0x492ae4；范围检查和查字节表从 0x492805 开始） |
| `native` 处理函数；弹出类号，再按 类号 − 1 分派（跳转表 0x47eab4，没有字节表） | 0x47e3d0 |
| 断点：类号已弹出，语句的值仍在栈上 | 0x47e43c（`eax` = 类号，`esi` = 脚本对象，开头就是它的值栈 vector） |
| 弹出值：把栈顶的值复制到调用者给的缓冲区并返回它；每个 0x20 字节，+0 为种类，+4 为数字，+8 为 `std::string` | 0x492c00 |
| `message` 处理函数（在这里下断点可标出每条消息） | 0x480410 |

这里类号在断点前一刻读出（0x47e43c 处的 `dec eax` 是分派的第一条指令），弹出函数用 `ecx` 传脚本对象，输出缓冲区放在栈上。这个版本的 spec：

```json
{
  "name": "nanairo 1.0.0.2",
  "stack": {"vector": "esi", "size": 32, "kind": 0, "int": 4, "str": 8, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x47e43c", "stack": true, "call": {"read": "eax", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}},
    {"at": "0x480410", "call": "message"}
  ]
}
```

### kimaten.exe

`kimaten.exe` 1.0.4.0 的链接地址（映像基址 0x400000）。函数和上面的相同，只是地址不同；栈的布局也相同。已对照反汇编核对。

| 内容 | 位置 |
| --- | --- |
| 脚本操作码分派（操作码 0x18 进入 `native` 处理函数，0x19 进入 `message`，0x1a 进入 `smessage`） | 0x4bd145（字节表 0x4bd4d4，跳转表 0x4bd414），在 0x4bcfe0 这个函数里 |
| `native` 处理函数；弹出类号，再按 类号 − 1 分派（跳转表 0x4a8c8c） | 0x4a8610 |
| `native` 处理函数处的断点：类号已弹出，语句的值仍在栈上 | 0x4a8673（`eax` = 类号，`esi` = 脚本对象，开头就是它的值栈 vector） |
| 弹出值（vector 在 `ecx`，+0 和 +4 为起止指针）：复制栈顶的值，每个 0x20 字节；+0 为种类（0 数字，1 字符串），+4 为数字，+8 为 `std::string`（容量 +0x1c 不超过 15 时文本内联，长度在 +0x18） | 0x4bd530 |
| `message` 处理函数（在这里下断点可标出每条消息） | 0x4aa7b0 |

这个版本的 spec：

```json
{
  "name": "kimaten 1.0.4.0 (2017)",
  "stack": {"vector": "esi", "size": 32, "kind": 0, "int": 4, "str": 8, "kinds": {"0": "int", "1": "str"}},
  "breakpoints": [
    {"at": "0x4a8673", "stack": true, "call": {"read": "eax", "names": {
      "1": "Select", "2": "MesWnd", "3": "Shake", "10": "Font", "11": "Value", "12": "Mouse",
      "13": "Anime", "14": "Flag", "15": "Music", "16": "Effect", "17": "HEffect", "18": "Voice",
      "19": "Sprite", "20": "Screen", "21": "Script", "22": "BackLog", "23": "Movie", "24": "OneMes",
      "25": "Xml", "26": "Ui", "27": "Save", "28": "Wait", "29": "Game"}}},
    {"at": "0x4aa7b0", "call": "message"}
  ]
}
```

## 安装文件和存档

`apeiria.exe` 从 `AppData\Roaming\SilkysPlus\apeiria` 读取 `apeiria_inst.xml`，其中 `InstallPath` 没有设置时立即停止；存档和设置在它旁边的 `Save` 文件夹中。`casablanca.exe` 同样从 `AppData\Roaming\SilkysPlus\casablanca` 读取 `casablanca_inst.xml`，它的设置和存档也在旁边的 `Save` 文件夹中（设置文件 `Save\casablanca.xml`）。把那里 `onemes.dat` 的每一位都设为 1 后，按住 Ctrl 就能把剧情快进到要记录的地方。

## 游戏

- [景の海のアペイリア](../games/apeiria.zh-CN.md)
- [景の海のアペイリア ～カサブランカの騎士～](../games/casablanca.zh-CN.md)
- [きまぐれテンプテーション](../games/kimaten1.zh-CN.md)
- [なないろリンカネーション](../games/nanairo.zh-CN.md)
