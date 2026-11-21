# engine-2026

[English](engine-2026.md) | 简体中文

ふゆから、くるる。下载版中的引擎：`fuyukuru.exe` 1.0.1.0，含日文、英文和中文，2026-03-18 用 Visual Studio 2022 链接，SHA-256 `eeaf1f25cfcc8105c0d0720a46266815c2bb3ee28696cc1374f863f8cdfdb707`。

这是基础配置：`crates/silky-runtime/src/host.rs` 中的引擎类最初就是按这个版本写的，未知游戏也使用它。各个类的处理见 [engine.zh-CN.md](../engine.zh-CN.md)。它的影片调用和 [engine-2024](engine-2024.zh-CN.md) 相同，而不是同一游戏 2021 年盒装版（[engine-2021](engine-2021.zh-CN.md)）的那一套。

## 游戏

- [ふゆから、くるる。](../games/fuyukuru.zh-CN.md)（下载版）

## 调用记录探针

程序链接地址（映像基址 0x400000），供 `tools/trace/probe.py` 使用（见 [trace.zh-CN.md](../trace.zh-CN.md)）。已在 Wine 下核对从启动到标题菜单的部分。

| 内容 | 地址 |
| --- | --- |
| 脚本操作码分派（操作码 0x18 进入 `native` 处理函数） | 0x44ae7c（字节表 0x44cac4，跳转表 0x44ca04） |
| `native` 处理函数；弹出类号，再按 类号 − 1 分派（表 0x430458） | 0x42f8e0 |
| 断点：类号已弹出，语句的值仍在栈上 | 0x42f941（`esi` = 类号，`[ebp-0x58]` = 脚本对象，其值栈是一个 vector） |
| 弹出值：复制栈顶的值，每个 0x4c 字节；+0 为种类（0 数字，1 字符串），+4 为数字，+0xc 为 `std::string` | 0x44cb20 |
| 类 25（`Xml`）等几个类在分派内部直接处理，不经过各自的 `CFunc` 对象 | 0x42ffd9 |

这个版本的 spec 见英文页面。
