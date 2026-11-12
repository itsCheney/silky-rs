> [!WARNING]
> 项目仍在开发中。目前可运行两款游戏，其他 Silky Engine 游戏尚未测试。

# silky-rs

[English](README.md) | 简体中文

用 Rust 重新实现的跨平台 Silky's 引擎（SILKY'S PLUS 与 elf 自 2014 年起使用的 "Silky Engine"）。它直接读取原版游戏文件，需要你自己持有游戏。引擎行为取自游戏文件，必要时参考引擎的可执行文件，但运行时本身从不加载可执行文件。

## 进度

| 游戏 | 可用部分 | 主要缺口 |
| --- | --- | --- |
| 《ふゆから、くるる。》（2026 年国际版 1.0.1.0） | 全部脚本和图片可解码；英文和简繁中文下所有路线都能玩到结局；标题、对话框、选项、存档、回想、设置、鉴赏模式、视频，以及取自可执行文件的特效 | 英文文字排版和选项位置尚未与原版对照；少数特效时间为近似值 |
| 《きまぐれテンプテーション2》（2024 年日文版，1.02 补丁） | 剧情文本、界面、行动菜单和地图、换装、笔记本和收藏品、鉴赏模式、E-mote 模型、H 模式 | 按钮栏的固定按钮；H 模式的部分功能；无界面运行时，调查环节需要在选项中选对答案 |
| 其他 Silky Engine 游戏 | 尚未测试 | 每个游戏都会加入自己的引擎命令 |

桌面端首先支持 Windows；macOS 和 Linux 在 CI 中可以构建并通过测试。核心 crate 也可以为 iOS 构建。各游戏的详细情况见 [docs/games.zh-CN.md](docs/games.zh-CN.md)，共用的引擎功能见 [docs/features.zh-CN.md](docs/features.zh-CN.md)。

## 开始使用

需要 Rust 1.85 或更新版本。

```sh
cargo build --release
cargo run --release -p silky-player -- <游戏目录>
cargo run --release -p silky-player -- <游戏目录> --lang zh-hans
```

单击、回车或空格继续，右键隐藏对话框，鼠标滚轮打开回想，Esc 关闭画面。字体、存档等选项见 [docs/running.zh-CN.md](docs/running.zh-CN.md)；查看工具和无界面运行器见 [docs/cli.zh-CN.md](docs/cli.zh-CN.md)。

## 文档

- [游戏](docs/games.zh-CN.md)：各游戏的运行情况和尚未完成的部分。
- [功能](docs/features.zh-CN.md)：所有游戏共用的文件格式、虚拟机、渲染器和引擎画面。
- [运行](docs/running.zh-CN.md)：桌面播放器、字体、存档，以及 GPU 或软件渲染。
- [命令行](docs/cli.zh-CN.md)：`silky-tool` 和 `silky-run`。
- [第三方参考](THIRD_PARTY.md)。

## 结构

| Crate | 内容 |
| --- | --- |
| `silky-format` | 文件格式。不含平台相关代码。 |
| `silky-vm` | 字节码解释器。引擎函数交给宿主处理。 |
| `silky-runtime` | 基于场景模型的引擎函数、渲染器和引擎画面。不含平台相关代码。`Session` 接收输入并给出画面。 |
| `silky-tool` | 查看游戏目录的命令行工具。 |
| `silky-run` | 无界面运行器：记录引擎调用，或把画面保存为 PNG。 |
| `silky-player` | 桌面播放器（winit、wgpu）。 |

提交前运行 `cargo test --workspace`。

## 项目规则

- 游戏数据不进入仓库：图片、脚本、声音、可执行文件和转储都不提交。测试使用在代码中构造的小文件。
- 运行时既不需要也不加载可执行文件，复制保护不在本项目范围内。
- 未知的操作码和引擎函数会连同参数一起报告，绝不为了让脚本继续运行而当作成功处理。

## 致谢

封包、LZSS、AKB 和 VSD 的格式参考了 [GARbro](https://github.com/morkt/GARbro)（MIT）。脚本操作码表最初来自 TesterTesterov 的 [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler)。详见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 许可证

GPL-3.0-or-later，见 [LICENSE](LICENSE)。`vendor/eluna_rs`（[Eluna](https://github.com/xmoezzz/eluna) 的修改版副本，见其中的 `README.silky.md`）和 `crates/silky-runtime/src/emote/draws.rs`（改编自 [siglus_rs](https://github.com/xmoezzz/siglus_rs)）为 MPL-2.0。

本项目的许可证不授予原版游戏的程序、图片、声音或脚本的任何权利。
