# silky-rs

[English](README.md) | 简体中文

用 Rust 重新实现的跨平台 Silky's 引擎（SILKY'S PLUS 与 elf 自 2014 年起使用的 "Silky Engine"）。它直接读取原版游戏文件，需要你自己持有游戏。

首个目标游戏是《ふゆから、くるる。》2026 年国际版 1.0.1.0。桌面端首先支持 Windows。

## 进度

还在早期。目前可用的部分：

- `.arc` 封包，包括 LZSS 压缩，以及 `Patch.xml` 规定的覆盖顺序（`script.arc;script1.arc`：后面的封包优先）。
- AKB 图片，包括叠加在基础图上的 `AKB+` 差分图。
- `.MAP` 文本表（每种语言一个分区）。
- `.MES` 脚本与 `LIBLARY.LIB`：文件头、偏移表和完整的字节码解码。
- `.VSD` 视频容器（内部是 MPEG-1 程序流）。
- 脚本虚拟机：栈、变量区、函数、脚本调用和选项。引擎函数交给宿主处理。
- 第一批引擎函数和软件渲染器：背景、事件 CG 及其差分、带表情的立绘、对话框及其名牌、头像和文本、本地化文件名、标题画面图片。
- 引擎自绘的界面：标题菜单按钮（布局取自可执行文件，按语言区分）、对话框的按钮栏和图标（`mes_o.area`）、等待点击的指示图标，以及选项（`Game.20`）。
- 桌面播放器：单击或回车继续，右键隐藏对话框，鼠标滚轮打开回想，Esc 关闭画面，自动和快进模式，标题菜单和选项。
- 声音：音乐（按 WAV 文件中 `smpl` 的循环点循环）、音效和语音，由软件混音；桌面播放器通过 cpal 播放。
- 时间与转场：定时等待（点击可跳过）、淡出到某种颜色和淡入、图片与立绘的交叉淡化、使用游戏 `NN.bin` 遮罩的规则转场、画面震动，以及等待声音播放结束。时间目前暂按 1/60 秒一帧计算。
- 存档和读档画面，按引擎的方式绘制（`saveload.area` 及其图片）：手动存档、自动存档和快速存档三个分页，缩略图、日期、最新存档标记，覆盖存档、删除单个或全部存档，取自 `saveloadhelp.map` 的提示文字，以及带“下次不再显示”的确认对话框。出现选项时自动存档；“继续游戏”读取最新的存档。存档是本运行时自己的 JSON 文件，每个存档附一张 PNG 缩略图。
- 回想（`backlog00.area`）：最近 300 条消息及其名牌，重播某句的语音，依次播放勾选的语音，以及跳回某条消息。
- 设置画面（`config00.area`、`config01.area`），可从标题菜单、对话框的设置按钮或右键打开：文字速度、逐字显示及句读后的停顿、快进（仅已读文本、速度、选项后继续）、自动播放、对话框颜色和不透明度、特效速度、各类确认对话框、右键功能、语言（游戏中即可切换），以及总音量、音乐、音效、语音和每个角色语音的音量。提示文字取自 `confighelp.map` 和 `soundconfighelp.map`。
- 标题菜单的鉴赏模式：CG 鉴赏（`cgmode.area`，九页缩略图，点击可依次浏览一组图片）、剧情回放（`scmode.area`）和音乐鉴赏（`soundmode.area`，带播放进度条）。解锁记录与设置一起保存在存档文件夹的 `system.json` 中：剧情中出现过的图片和音乐，以及读过的文本。CG、剧情和曲目的列表取自《ふゆから、くるる。》的可执行文件。
- 视频（`.vsd`）：MPEG-1 视频和 MP2 音频由 [pl_mpeg](https://github.com/phoboslab/pl_mpeg)（MIT，位于 `crates/silky-runtime/vendor`）解码；画面跟随声音的时钟，点击可结束视频。
- 对话框的“上一个选项”按钮（回到上一个选项前的那句话）和“下一个选项”按钮（快进到下一个选项，未读文本也会跳过）。

在《ふゆから、くるる。》上，全部 56 个脚本（约 790 万条指令）都能解码，所有跳转都落在指令边界上；全部 3,255 张图片都能解码。在无界面模式下，虚拟机能从 `start.mes` 启动，依次走过 logo、注意事项和标题脚本，并从 `fk001.mes` 开始执行剧情，三种语言的文本都能取到。

尚未完成：特写小窗和其他特效。剧情中出现但尚未执行的调用包括 `Game.12` 的子命令 4 和 5（看起来是在事件 CG 上移动镜头）以及 `Game.13` 的子命令 7 和 14（立绘移动），具体作用还需要从可执行文件中读出。“固定文字框”设置会被保存，但不起作用，因为这里的对话框本来就不会移动。`silky-run snap` 会列出运行时尚未执行的所有引擎调用。

已与原版截图（Windows，简体中文）对照：对话框、名牌、对话框头像、中文文本的位置和字距，以及立绘的大小和位置。仍为暂定：英文文本的字体度量、两行文本的行距、换行宽度，以及选项在画面上的位置。

## 结构

| Crate | 内容 |
| --- | --- |
| `silky-format` | 文件格式。不含平台相关代码。 |
| `silky-vm` | 字节码解释器。引擎函数交给宿主处理。 |
| `silky-runtime` | 基于场景模型的引擎函数和软件渲染器。不含平台相关代码。 |
| `silky-tool` | 查看游戏目录的命令行工具。 |
| `silky-run` | 无界面运行器：记录引擎调用，或把画面保存为 PNG。 |
| `silky-player` | 桌面播放器（winit；画面由软件绘制）。 |

`silky-runtime::Session` 持有一个运行中的游戏（解释器、运行时、渲染器和界面），接收指针、按键和时钟输入；桌面播放器以及以后的嵌入方式只负责转发输入并显示它的画面。

## silky-player

```sh
silky-player <游戏目录>                    # 英文
silky-player <游戏目录> --lang 1           # 简体中文
silky-player <游戏目录> --font msyh.ttc --font arial.ttf
```

存档保存在 Windows 的 `%APPDATA%\silky-rs\<游戏目录名>`（macOS 为 `~/Library/Application Support/silky-rs/...`，其他系统为 `~/.local/share/silky-rs/...`），可用 `--saves <目录>` 指定其他位置。不指定 `--font` 时，播放器会先在 Windows 字体目录中查找游戏设置里使用的字体（Arial、微软雅黑、微软正黑体），再查找 macOS 和 Linux 上的常见字体。单击、回车或空格继续；右键隐藏对话框（可在设置中更改）；Esc 关闭画面或结束视频。

## 构建

需要 Rust 1.85 或更新版本。

```sh
cargo build --release
cargo test
```

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
silky-run trace <游戏目录> --result 29.110=1 --text-only --lang 1 --max-calls 50000
```

```sh
silky-run snap <游戏目录> --out snaps --font DejaVuSans.ttf --lines 0,10,50
silky-run snap <游戏目录> --lang 1 --font msyh.ttc --lines 170
```

`snap` 用 `silky-runtime` 运行游戏，把标题画面和指定的对话行（从游戏第一行开始计数）保存为 PNG。字体不属于游戏文件，需要用 `--font` 传入一个或多个字体，按回退顺序排列；不提供字体时画面中不绘制文本。

`trace` 不执行任何引擎函数，只记录每次调用及其参数。脚本会用到返回值的调用默认返回 0，可以用 `--result 类.子命令=值` 指定。选项（`Game.20`）由 `--choose` 决定，从 0 开始计数（默认第一个）。`--lang` 选择 `.MAP` 文本表的分区（此版本中 0 为英文，1 为简体中文，2 为繁体中文）。

## 项目规则

- 游戏数据不进入仓库：图片、脚本、声音、可执行文件和转储都不提交。测试使用在代码中构造的小文件。
- 引擎行为从游戏自身的文件中获得，必要时阅读引擎的可执行文件。运行时既不需要也不加载可执行文件，复制保护不在本项目范围内。
- 未知的操作码和引擎函数会连同参数一起报告，绝不为了让脚本继续运行而当作成功处理。

## 致谢

封包、LZSS、AKB 和 VSD 的格式参考了 [GARbro](https://github.com/morkt/GARbro)（MIT）。脚本操作码表最初来自 TesterTesterov 的 [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler)。详见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 许可证

GPL-3.0-or-later，见 [LICENSE](LICENSE)。
