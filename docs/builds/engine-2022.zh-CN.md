# engine-2022

[English](engine-2022.md) | 简体中文

同級生 Remake Steam 版（英文、简体中文、繁体中文）的引擎：`doukyuusei.exe` 1.0.4.0，2022-03-15 07:54:45 用 Visual Studio 2017（链接器 14.16）链接，11,159,880 字节，SHA-256 `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451`。程序带有 Steam 的 DRM 外壳。

画面为 1920 x 1080。引擎类暂按 [engine-2026](engine-2026.zh-CN.md) 处理（不含 `sprites_and_flags`），还没有逐个调用对照过。

目前看到的与其他版本的不同：

- 文本表：每个脚本每种语言一个 MAP 文件（`X_EN.MAP`、`X_CN.MAP`、`X_TW.MAP`）：u32 条数，然后是这么多对 u32 行号和 u32 偏移，然后是以 0 单元结尾的 UTF-16 行。脚本本身不带文本，`Game.24` 按行号显示一行（见游戏页面）。
- 消息文字用位图字体：`font_han.akb`（半角，20 x 40 格）、`font_han_i.akb`（同样的字母，斜体）和 `font_zen.akb`（全角，40 x 40 格），每行 20 格，顺序与 `font_han.map`、`font_zen.map` 相同（u16 个数，然后是这么多个 UTF-16 字符）。
- `update.arc` 存放各个档案中文件的较新版本，优先读取。
- 简体中文图片以 `_ch` 结尾（`title_bg_ch.akb`），文本表则用 `_cn`。

## 游戏

- [同級生 Remake](../games/doukyuusei.zh-CN.md)（Steam）
