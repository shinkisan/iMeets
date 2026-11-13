# iMeets

简体中文 · [English](README.en.md)

一个 iOS 上的**拓麻歌子 Meets / On** 模拟器外壳，目前主要针对日版 Sweets 固件优化。内核采用 MIT 协议的 [nijiiro-egg](https://github.com/oreo-fox/nijiiro-egg)，交互学习 [Open Meets](https://github.com/kanatallow/Open-Meets)。

iMeets 与任何 Bandai 官方产品无关，也未获 Bandai 认可。本仓库不提供固件。

## 功能

- **运行 8 MiB 的 Meets / On 固件。**
- **面对面联机。** 两台运行 iMeets 的设备可通过局域网扫码或连接串建立通道，再在固件内操作原版联机流程；已在日版 Sweets 验证两台实机的「みーつ → あそぶ」。模拟器也可使用连接串联机，详见[联机说明](docs/ios-lan.md)。
- **后台画中画。** 切换到其他 App 时仍可查看宠物画面；画中画的播放／暂停按钮同步控制养育暂停状态。
- **离线补时。** 重新打开后按真实离开时长追赶，期间有进度提示，结束后时间与手机一致。
- **暂停养育。** 已验证的日版 Sweets v062 普通角色恢复养育时，默认只补算暂停区间与最近一次睡眠的交集；跳过清醒时间和更早的夜晚，再对齐手机时钟。
- **养育信息。** 顶栏 ⓘ 查看成长余量、养育失误、呼叫忽视预算及需求计时；目前仅支持已验证的日版 Sweets v062，不兼容 ROM 会显示提示。
- **便意提醒。** 已验证的 Sweets 固件出现便意时，前台应用内提示并播放短提示音，后台使用系统通知；声音尊重 App 内声音设置。
- **实时翻译面板。** 游戏里的文字即时显示为中文，面板常驻；画面没有文字时显示「……」。
- **快速按键（实验性，可在设置中开启）。** 按键到画面的响应从 63 ms 缩短到 25 ms，操作更跟手。
- **外观。** 真机与极简各有白天、夜间皮肤，均可跟随系统自动切换；另有小屏模式。
- **自定义顶栏。** 按使用习惯调整常用按钮的显示与顺序，收起的功能仍可从「更多」打开。
- **按键触感。** 每次按下外壳按键有一次轻触反馈，可在设置里关闭。

## 环境要求

- macOS + Xcode（iOS 16+）以及装好 iOS target 的 Rust
- 真机或模拟器
- 你自己的 8 MiB 固件（`.bin`）

## 安装

**直接安装现成包**

从 [Releases](https://github.com/shinkisan/iMeets/releases) 下载最新的 `iMeets-*-unsigned.ipa`，再用下面的工具之一签名安装：

| 工具 | 说明 |
|---|---|
| AltStore / Sidekick | 先在电脑上装好 AltServer，用它把 IPA 装到手机上，用你的 Apple ID 签名 |
| Sideloadly | 电脑连上手机，拖入 IPA，输入 Apple ID 后开始安装 |
| TrollStore | 系统版本支持时可直接安装，无需签名 |

**从源码编译**

见下面的「编译」一节。

**首次打开**

点「选择 ROM 文件」，从「文件」中选择你自己准备的 8 MiB 固件（`.zip` 也可以）。导入后即可开始养育。

## 编译

Rust 内核与测试（Linux 或 macOS）：

```sh
cargo test --manifest-path rust/shell/Cargo.toml
```

测试通过 `POCKET_MEETS_ROM` 指定固件，设置后会自动加载。菜单语料测试依赖本地生成的真机抓屏 dump，`tools/` 中提供了生成脚本。

iOS：

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --release --target aarch64-apple-ios \
  --manifest-path rust/shell/Cargo.toml
cargo build --release --target aarch64-apple-ios-sim \
  --manifest-path rust/shell/Cargo.toml
open ios/PocketMeets/PocketMeets.xcodeproj
```

在 target 设置里选自己的 Development Team 后运行。Xcode 工程从 `target/aarch64-apple-ios{,-sim}/release` 链接静态库，并通过 bridging header 引入 `rust/shell/include/pocket_meets.h`。

Intel 模拟器需要另外编译一份：

```sh
cargo build --release --target x86_64-apple-ios \
  --manifest-path rust/shell/Cargo.toml
```

首次启动时导入固件。App 容器位于 Application Support，并已排除备份。

## 目录

| 路径 | 说明 |
|---|---|
| `third_party/nijiiro-egg` | MIT 模拟器内核（CPU、LCD、GPIO、RTC、SPI flash、声音） |
| `rust/shell` | 会话 pump、补时、按键补丁、文字缓冲、设置、FFI（`include/pocket_meets.h`） |
| `ios/PocketMeets` | SwiftUI App |
| `tools` | 字符串抽取、翻译表生成、字模/菜单研究、覆盖率检查 |
| `docs` | 红外协议逆向笔记与本机验证 |
| `rust/shell/examples` | 无头探针：菜单遍历、延迟、按键扫描、时钟跳变 |

## 红外联机

两台机器的原版红外联机（`つうしん → みーつ → あそぶ`）已经在模拟器里跑通：链路帧、OBEX `TAMA` 服务和告别流程都由固件自己完成。协议细节和本机验证方法见 [docs/infrared.md](docs/infrared.md)。

## 相关项目

- [Open Meets](https://github.com/kanatallow/Open-Meets) —— 安卓上的姊妹项目，本项目外壳学习其进行开发。
- [nijiiro-egg](https://github.com/oreo-fox/nijiiro-egg) —— 本项目内置的 MIT 内核。

## 法律声明

iMeets 是独立项目。Tamagotchi、Meets、On 及相关商标与美术资源归 Bandai 所有。本仓库不分发固件及其资源。

## 协议

模拟器内核为 MIT（Copyright (c) 2026 oreo-fox，见 [third_party/nijiiro-egg/LICENSE](third_party/nijiiro-egg/LICENSE)），外壳为 GPL-3.0（见 [LICENSE](LICENSE)）。
