# 游戏

[English](games.md) | 简体中文

各个游戏在 silky-rs 上的运行情况。所有游戏共用的引擎见 [engine.zh-CN.md](engine.zh-CN.md) 和 [features.zh-CN.md](features.zh-CN.md)。`silky-run snap` 会列出运行时尚未执行的所有引擎调用。

| 游戏 | 识别依据 | 页面 |
| --- | --- | --- |
| ふゆから、くるる。（国际版 1.0.1.0；日文盒装版 1.0.0.0 和 1.0.1.0） | `fuyukuru.exe` | [games/fuyukuru.zh-CN.md](games/fuyukuru.zh-CN.md) |
| きまぐれテンプテーション2（2024 年日文版，1.02 补丁） | `kimaten2.exe` | [games/kimaten2.zh-CN.md](games/kimaten2.zh-CN.md) |
| 景の海のアペイリア（日文版，带官方更新），剧情和鉴赏模式可用 | `apeiria.exe` | [games/apeiria.zh-CN.md](games/apeiria.zh-CN.md) |
| 景の海のアペイリア ～カサブランカの騎士～（日文版 1.0.0.0），剧情和鉴赏模式可用 | `casablanca.exe` | [games/casablanca.zh-CN.md](games/casablanca.zh-CN.md) |
| 同級生 Remake（Steam 版，英文和中文），可以游玩房间、行走地图和对话，有原版样式的存档/读档画面、回想记录和设置画面（系统和声音），以及日程按钮打开的旗标列表 | `doukyuusei.exe` | [games/doukyuusei.zh-CN.md](games/doukyuusei.zh-CN.md) |

运行时根据游戏目录中程序文件的名字区分游戏，从不读取这个文件。这些名字都没有的目录会作为尚未支持的游戏运行：共用的引擎和通用的 `Game` 命令可以工作，其余的调用会被报告出来。如何加入新游戏见 [adding-a-game.zh-CN.md](adding-a-game.zh-CN.md)。
