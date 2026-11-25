> [!WARNING]
> 项目仍在开发中。目前可运行四款游戏，其他 Silky Engine 游戏尚未测试。

# silky-rs

[English](README.md) | 简体中文

用 Rust 重新实现的跨平台 Silky's 引擎（SILKY'S PLUS 与 elf 自 2014 年起使用的 "Silky Engine"）。它直接读取原版游戏文件，需要你自己持有游戏。引擎行为取自游戏文件，必要时参考引擎的可执行文件，但运行时本身从不加载可执行文件。

## 进度

| 游戏 | 可用部分 | 主要缺口 |
| --- | --- | --- |
| 《ふゆから、くるる。》（2026 年国际版 1.0.1.0） | 全部脚本和图片可解码；英文和简繁中文下所有路线都能玩到结局；标题、对话框、选项、存档、回想、设置、鉴赏模式、视频，以及取自可执行文件的特效 | 英文文字排版和选项位置尚未与原版对照；少数特效时间为近似值 |
| 《きまぐれテンプテーション》（日文版，带官方更新 1.04） | 标题、对话框和按钮栏、行动菜单、调查、证据和选项列表、地图、笔记、提示、E-mote 模型（同伴、立绘、CG 和与语音同步的模型）、特效、CG、场景和音乐鉴赏、Anne viewer、工作人员名单 | `Game.1001` 和 `1002` 的特效没有与原版对比；画面的少数细节是猜的（见该页）；无界面模式下用 `K1_ROUTE` 能到达所有结局 |
| 《きまぐれテンプテーション2》（2024 年日文版，1.02 补丁） | 剧情文本、界面、行动菜单和地图、换装画面、笔记本和收藏品、鉴赏模式、E-mote 模型、H 模式（已与原版对照，包括打码马赛克） | 无界面运行时，调查环节需要在选项中选对答案 |
| 《先輩が私の妄想にドージンする》（日文版 1.0.1.0） | 无界面模式下剧情文本可播放到结局 | 界面、游戏自有命令和影片尚未完成 |
| 《景の海のアペイリア》（日文版，带官方更新） | 全部脚本可解码，剧情可以运行到结局；标题、对话框、选项、注音，游戏内画面（αMAIL、DNA PC、战斗血条），攻击和法术特效，场景上方图片的动画，邮件转发画面，CG、场景和音乐鉴赏；与原版对照追踪过的路线，直到结局引擎调用都一致 | 只有一个特效与原版逐帧对照过；其余按可执行文件的代码实现 |
| 《景の海のアペイリア ～カサブランカの騎士～》（日文版 1.0.0.0） | 两条路线和最后一章都能运行到结局；Apeiria 的各个画面和特效、新增的擦除和攻击种类 18，CG、场景和音乐鉴赏；开头部分的引擎调用与原版一致（原版会重复一次名字和语音调用） | 少数画面与原版仍有差异（推理板的擦除、返回时的 CG 页）；特效没有逐帧对照 |
| 《なないろリンカネーション》（2014 年日文版） | 带解锁的标题菜单、带按钮的对话框、文字、说话人名字、立绘、头像窗口、CG 登记、事件图（事件动画）、结局叠加画面、制作人员名单 | 日历画面、场景影片；事件动画和制作人员名单的部分数值为近似；文字行长和等待标记动画尚未测量；立绘位置尚未与原版对照 |
| 《同級生 Remake》（Steam 版，英文和中文） | 标题、主角房间、小时转盘、带地图跳转和地点标签的行走地图、对话和选项、旗标列表及其跳转、菜单的滑出、名字输入（含按键和粘贴）、角色选择画面、带备注的存档/读档、以 GDI 字体绘制的提示栏、回想记录、设置、鉴赏模式（相册、回忆片段、音乐、留言） | 其他地点画面；带标记的选项、设置画面的淡入和新的键盘输入尚未与原版对照；GDI 字体由前端的 TrueType 字体代替 SimHei |
| 其他 Silky Engine 游戏 | 尚未测试；可以用共用的引擎和通用命令运行 | 每个游戏都会加入自己的引擎命令，加入之前会被报告出来（见 [docs/adding-a-game.zh-CN.md](docs/adding-a-game.zh-CN.md)） |

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
- [引擎](docs/engine.zh-CN.md)：游戏之间共用什么、各自加入什么，以及各部分的可靠程度。
- [引擎版本](docs/builds/README.zh-CN.md)：核对过的程序版本，以及它们的引擎有哪些不同。
- [Game 命令](docs/game-commands.zh-CN.md)：各游戏自己的引擎命令。
- [功能](docs/features.zh-CN.md)：所有游戏共用的文件格式、虚拟机、渲染器和引擎画面。
- [加入新游戏](docs/adding-a-game.zh-CN.md)。
- [运行](docs/running.zh-CN.md)：桌面播放器、字体、存档，以及 GPU 或软件渲染。
- [命令行](docs/cli.zh-CN.md)：`silky-tool` 和 `silky-run`。
- [调用记录](docs/trace.zh-CN.md)：一次运行的引擎调用，与原版程序或另一次运行对拍。
- [第三方参考](THIRD_PARTY.md)。

## 结构

| Crate | 内容 |
| --- | --- |
| `silky-format` | 文件格式。不含平台相关代码。 |
| `silky-vm` | 字节码解释器。引擎函数交给宿主处理。 |
| `silky-runtime` | 基于场景模型的引擎函数、渲染器和引擎画面；各游戏自己的部分在 `games/` 中。不含平台相关代码。`Session` 接收输入并给出画面。 |
| `silky-tool` | 查看游戏目录的命令行工具。 |
| `silky-run` | 无界面运行器：记录引擎调用，或把画面保存为 PNG；写出并比较调用记录。 |
| `silky-trace` | 引擎调用记录文件及两份记录的比较。运行时不依赖它。 |
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
