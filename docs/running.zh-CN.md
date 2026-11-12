# 运行

[English](running.md) | 简体中文

## 构建

需要 Rust 1.85 或更新版本。

```sh
cargo build --release
cargo test --workspace
```

生成的程序位于 `target/release`。每次推送后 CI 都会构建 Windows 版程序（`silky-rs-windows` 构件）。

## silky-player

```sh
silky-player <游戏目录>                    # 游戏的第一种语言
silky-player <游戏目录> --lang zh-hans     # 简体中文（或 1）
silky-player <游戏目录> --font msyh.ttc --font arial.ttf
```

存档保存在 Windows 的 `%APPDATA%\silky-rs\<游戏目录名>`（macOS 为 `~/Library/Application Support/silky-rs/...`，其他系统为 `~/.local/share/silky-rs/...`），可用 `--saves <目录>` 指定其他位置。不指定 `--font` 时，播放器会先在 Windows 字体目录中查找游戏设置里使用的字体（Arial、微软雅黑、微软正黑体），再查找 macOS 和 Linux 上的常见字体。单击、回车或空格继续；右键隐藏对话框（可在设置中更改）；Esc 关闭画面或结束视频。播放器通过 wgpu（Metal、Vulkan、D3D12）在 GPU 上合成画面：运行时记录每帧要绘制的内容，由 `silky_runtime::compose` 按窗口尺寸绘制，图片、字形和 E-mote 模型都保留在 GPU 上。加 `--software`（或机器没有可用的 GPU）时改为软件绘制。

## 嵌入

`silky-runtime::Session` 持有一个运行中的游戏（解释器、运行时、渲染器和界面），接收指针、按键和时钟输入；桌面播放器以及以后的嵌入方式只负责转发输入并显示它的画面。
