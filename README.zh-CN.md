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
- 第一批引擎函数和软件渲染器：背景、事件 CG 及其差分、带表情的立绘、对话框和文本、本地化文件名、标题画面图片。

在《ふゆから、くるる。》上，全部 56 个脚本（约 790 万条指令）都能解码，所有跳转都落在指令边界上；全部 3,255 张图片都能解码。在无界面模式下，虚拟机能从 `start.mes` 启动，依次走过 logo、注意事项和标题脚本，并从 `fk001.mes` 开始执行剧情，三种语言的文本都能取到。

尚未完成：标题菜单按钮、对话框的按钮栏和图标、特写小窗、转场与特效、声音、视频、回想、存档、设置，以及桌面播放器。`silky-run snap` 会列出运行时尚未执行的所有引擎调用。

已与原版截图（Windows，简体中文）对照：对话框、名牌、对话框头像、中文文本的位置和字距，以及立绘的大小和位置。仍为暂定：英文文本的字体度量、两行文本的行距和换行宽度。

## 结构

| Crate | 内容 |
| --- | --- |
| `silky-format` | 文件格式。不含平台相关代码。 |
| `silky-vm` | 字节码解释器。引擎函数交给宿主处理。 |
| `silky-runtime` | 基于场景模型的引擎函数和软件渲染器。不含平台相关代码。 |
| `silky-tool` | 查看游戏目录的命令行工具。 |
| `silky-run` | 无界面运行器。目前执行脚本，并记录每一次引擎调用。 |

计划中：`silky-player`（桌面播放器：winit、wgpu、cpal）。

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

`trace` 不执行任何引擎函数，只记录每次调用及其参数。脚本会用到返回值的调用默认返回 0，可以用 `--result 类.子命令=值` 指定。选项由 `--choose` 决定（默认第一个）。`--lang` 选择 `.MAP` 文本表的分区（此版本中 0 为英文，1 为简体中文，2 为繁体中文）。

## 项目规则

- 游戏数据不进入仓库：图片、脚本、声音、可执行文件和转储都不提交。测试使用在代码中构造的小文件。
- 引擎行为从游戏自身的文件中获得，必要时阅读引擎的可执行文件。运行时既不需要也不加载可执行文件，复制保护不在本项目范围内。
- 未知的操作码和引擎函数会连同参数一起报告，绝不为了让脚本继续运行而当作成功处理。

## 致谢

封包、LZSS、AKB 和 VSD 的格式参考了 [GARbro](https://github.com/morkt/GARbro)（MIT）。脚本操作码表最初来自 TesterTesterov 的 [mesScriptAsseAndDisassembler](https://github.com/TesterTesterov/mesScriptAsseAndDisassembler)。详见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 许可证

GPL-3.0-or-later，见 [LICENSE](LICENSE)。
