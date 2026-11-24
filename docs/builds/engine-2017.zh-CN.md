# engine-2017

[English](engine-2017.md) | 简体中文

三个程序所用的引擎，都用 Visual Studio 2013 链接。除特别标注外，均从可执行文件读出。

| 程序 | 版本 | 链接时间（UTC） | SHA-256 |
| --- | --- | --- | --- |
| `apeiria.exe`（景の海のアペイリア） | 1.0.3.0 | 2017-11-09 07:06:49 | `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e` |
| `casablanca.exe`（景の海のアペイリア ～カサブランカの騎士～） | 1.0.0.0 | 2017-12-10 14:52:07 | `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716` |
| `nanairo.exe`（なないろリンカネーション） | 1.0.0.2 | 2014-09-09 19:27:20 | 尚未记录 |

`casablanca.exe` 大小为 1,559,040 字节。它的引擎代码和 `apeiria.exe` 相同，所以下面的差异对两者都成立。trace 探针地址只给出 `apeiria.exe` 和 `casablanca.exe` 的。

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
| `HEffect`（类 17） | 像 `Effect` 一样播放音效（来自脚本） | 未实现 |

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

这个版本的 spec 同样见英文页面。

## 安装文件和存档

`apeiria.exe` 从 `AppData\Roaming\SilkysPlus\apeiria` 读取 `apeiria_inst.xml`，其中 `InstallPath` 没有设置时立即停止；存档和设置在它旁边的 `Save` 文件夹中。`casablanca.exe` 同样从 `AppData\Roaming\SilkysPlus\casablanca` 读取 `casablanca_inst.xml`，它的设置和存档也在旁边的 `Save` 文件夹中（设置文件 `Save\casablanca.xml`）。把那里 `onemes.dat` 的每一位都设为 1 后，按住 Ctrl 就能把剧情快进到要记录的地方。

## 游戏

- [景の海のアペイリア](../games/apeiria.zh-CN.md)
- [景の海のアペイリア ～カサブランカの騎士～](../games/casablanca.zh-CN.md)
- [なないろリンカネーション](../games/nanairo.zh-CN.md)
