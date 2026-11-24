# 引擎版本

[English](README.md) | 简体中文

每个游戏的程序文件里都链接着一份自己的引擎，引擎类（1 到 28）在这些版本之间有变化。运行时读取程序的链接时间戳和文件大小，选出对应版本的配置（`crates/silky-format/src/engine.rs`）。不在表中的程序使用同名、链接时间最接近的已知版本的配置，未知游戏使用 `engine-2026`。

| 配置 | 程序 | 版本 | 链接时间 (UTC) | 工具链 | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| [engine-2017](engine-2017.zh-CN.md) | `nanairo.exe`（なないろリンカネーション） | 1.0.0.2 | 2014-09-09 19:27:20 | Visual Studio 2013（链接器 12.0） | 尚未记录 |
| [engine-2017](engine-2017.zh-CN.md) | `apeiria.exe`（景の海のアペイリア） | 1.0.3.0 | 2017-11-09 07:06:49 | Visual Studio 2013（链接器 12.0） | `dba4857ac6adb02b453a73694737124725bd4c91a9a7ecd60816fa7266e6317e` |
| [engine-2017](engine-2017.zh-CN.md) | `casablanca.exe`（景の海のアペイリア ～カサブランカの騎士～） | 1.0.0.0 | 2017-12-10 14:52:07 | Visual Studio 2013（链接器 12.0） | `4865b23597087c03a4dd7017ced9e5c48e8d4479c923705cd41bd91b4246d716` |
| [engine-2017](engine-2017.zh-CN.md) | `kimaten.exe`（きまぐれテンプテーション，带更新 1.04） | 1.0.4.0 | 2019-12-16 04:48:49 | Visual Studio 2013（链接器 12.0） | `36148a0286bd9d31ceaec4323f4be987319bd759e68b5961c59c992b07b54700` |
| [engine-2021](engine-2021.zh-CN.md) | `fuyukuru.exe`（ふゆから、くるる。盒装版） | 1.0.0.0 | 2021-09-06 20:52:27 | Visual Studio 2019（链接器 14.29） | `9797a7cb71955c0ddb4b4865e0e592166f82386a5e642bd25072922cf9d6d9fa` |
| [engine-2021](engine-2021.zh-CN.md) | `fuyukuru.exe`（盒装版，官方更新 1.01） | 1.0.1.0 | 2021-09-29 05:47:46 | Visual Studio 2019（链接器 14.29） | `b5b163ceb979d9825c6586401da005d51a57f4005a93cf227ca304bf9e831988` |
| [engine-2022](engine-2022.zh-CN.md) | `doukyuusei.exe`（同級生 Remake，Steam 版） | 1.0.4.0 | 2022-03-15 07:54:45 | Visual Studio 2017（链接器 14.16） | `9c9841fbca3a422a36f7ddef696f6164b4206ff622ecdb5cc1b4f45c8b231451` |
| [engine-2024-08](engine-2024-08.zh-CN.md) | `doujin.exe`（先輩が私の妄想にドージンする） | 1.0.1.0 | 2024-08-30 07:40:15 | Visual Studio 2019（链接器 14.29） | 未记录 |
| [engine-2024](engine-2024.zh-CN.md) | `kimaten2.exe`（きまぐれテンプテーション2） | 1.0.0.0 | 2024-10-15 01:13:11 | Visual Studio 2019（链接器 14.29） | `5cbd99107b94befe4935bbb7b8fdd7015d715a795945b434788657dee0a50d29` |
| [engine-2026](engine-2026.zh-CN.md) | `fuyukuru.exe`（ふゆから、くるる。下载版，日英中） | 1.0.1.0 | 2026-03-18 14:12:42 | Visual Studio 2022（链接器 14.44） | `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707` |

版本号来自程序的版本资源；引擎本身没有版本号，所以配置按程序的链接年份命名。同一个游戏可能有不同年份的版本：ふゆから、くるる。下载版在 2026 年用比 2021 年盒装版更新的引擎重新链接过。

## 配置的内容

只记录已核对版本之间不同的地方：

| 字段 | 含义 |
| --- | --- |
| `sprites_and_flags` | `Sprite.1`、其他 `Sprite` 调用、`Flag` 和 `Mouse.4` 按 2026 版本的行为处理 |
| `sheet_store` | `Sprite.1` 隐藏创建、用来存放按钮图集的图层 |
| `backlog_voice_end` | 无参数的 `BackLog.3` 结束该条目的语音标记 |
| `movie_2_closes` | `Movie.0` 只接收文件名，`Movie.3` 播放，`Movie.2` 关闭且不返回值 |
| `heffect_sounds` | `HEffect`（类 17）像 `Effect` 一样播放音效 |
| `results` | 返回值与通用列表不同的引擎调用 |

`Game` 命令（类 29）不属于配置：它们属于各个游戏（`crates/silky-runtime/src/games/`）。

## 添加版本

读取 PE 头偏移 8 处的时间戳、文件大小和版本资源，在 `BUILDS` 中添加一个 `Build`，并指定它符合的配置。如果它和所有配置都不同，就新建一个配置并在这里添加页面。
