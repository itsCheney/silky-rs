# engine-2022

[English](engine-2022.md) | 简体中文

同級生 Remake Steam 版（英文、简体中文、繁体中文）的引擎：`doukyuusei.exe` 1.0.4.0，2022-03-15 07:54:45 用 Visual Studio 2017（链接器 14.16）链接，11,159,880 字节，SHA-256 `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451`。程序带有 Steam 的 DRM 外壳。

画面为 1920 x 1080。引擎类暂按 [engine-2026](engine-2026.zh-CN.md) 处理（不含 `sprites_and_flags`），从启动到标题菜单，与原版逐个调用一致（见下面的调用记录探针）。

目前看到的与其他版本的不同：

- 文本表：每个脚本每种语言一个 MAP 文件（`X_EN.MAP`、`X_CN.MAP`、`X_TW.MAP`）：u32 条数，然后是这么多对 u32 行号和 u32 偏移，然后是以 0 单元结尾的 UTF-16 行。脚本本身不带文本，`Game.24` 按行号显示一行（见游戏页面）。
- 消息文字用位图字体：`font_han.akb`（半角，20 x 40 格）、`font_han_i.akb`（同样的字母，斜体）和 `font_zen.akb`（全角，40 x 40 格），每行 20 格，顺序与 `font_han.map`、`font_zen.map` 相同（u16 个数，然后是这么多个 UTF-16 字符）。
- `update.arc` 存放各个档案中文件的较新版本，优先读取。
- 动画调用与 2017、2021 版相同：`Movie.0` 只取文件名，`Movie.3` 播放并返回是否播放（脚本会保存这个结果），`Movie.2` 关闭。
- 简体中文图片以 `_ch` 结尾（`title_bg_ch.akb`），文本表则用 `_cn`。

## 调用记录探针

程序链接地址（映像基址 0x400000），供 `tools/trace/probe.py` 使用（见 [trace.zh-CN.md](../trace.zh-CN.md)）。已在 Wine 下核对从启动到标题菜单的部分，用的是这个版本不带 DRM 外壳的副本（除代码外，各节逐字节相同）。

| 内容 | 地址 |
| --- | --- |
| 脚本操作码分派（操作码 0x18 进入 `native` 处理函数） | 0x461a1f（字节表 0x463434，跳转表 0x463374） |
| `native` 处理函数；弹出类号 | 0x43fa30 |
| 断点：类号已弹出，语句的值仍在栈上 | 0x43fa7e（`esi` = 类号，`[ebp-0xb8]` = 值栈，一个 vector） |
| 弹出值：与 [engine-2026](engine-2026.zh-CN.md) 相同的 0x4c 字节的值 | 0x463490 |

这个版本的 spec 见英文页面。

`Game.10(1, 18)` 在程序中什么也不做，并把它的 1 留在栈上，所以原版之后的调用前面会多一个 1，直到脚本返回。运行时则取走整条语句。

## 游戏

- [同級生 Remake](../games/doukyuusei.zh-CN.md)（Steam）
