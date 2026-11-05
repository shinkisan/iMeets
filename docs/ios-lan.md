# iOS 局域网联机（Simulator）

两台 App 通过 TCP 相连，连上后完全由用户操作固件。Host 展示二维码，
Join 扫码或粘贴连接串。iOS Simulator 没有相机，用“复制连接串 / 手动连接”。

## 构建并安装到 Mac 的模拟器

需要 macOS + Xcode。

```sh
DEVICE="iPhone 16" ./ios/install-sim.command
```

脚本会：

1. 为 `aarch64-apple-ios-sim`（Intel Mac 用 `x86_64-apple-ios`）编译 Rust 库；
2. `xcodebuild` 构建 `PocketMeets.app`；
3. 启动模拟器并 `simctl install` / `launch`。

如果模拟器名字不同，先看：

```sh
xcrun simctl list devices available
```

## 两台模拟器

在 Xcode 的 Simulator 里再开一个设备，或：

```sh
xcrun simctl create "iMeets B" com.apple.CoreSimulator.SimDeviceType.iPhone-16 com.apple.CoreSimulator.SimRuntime.iOS-18
```

分别安装启动后：

1. 两台都进 `设置 → 局域网联机`。
2. A 点“展示二维码”。模拟器里点“复制连接串”（或用 `手动连接` 粘贴）。
3. B 点“粘贴连接串”/“手动连接”，连接成功后自动回到设备界面。
4. 顶部出现“联机中”横幅时，就可以在游戏里操作了。

## 原生操作

连接成功后 App 不自动按键。两台各自走：

```
ツーしん → みーつ → あそぶ / アイテムこうかん / プロポーズ / おでかけ
```

到 `つうしん たいき…` 后，一方先按 B，另一方稍后再按。先按的一方会自然
形成固件需要的 lead。

## 权限

第一次会弹：

- 本地网络：必须允许；
- 相机：真机扫码需要；Simulator 没相机，用连接串。

## 已知限制

- App 进入后台会挂起 socket；
- Simulator 没有相机，二维码扫描不可用；
- 真机与模拟器之间、两台真机之间使用同一套 TCP + tick barrier；
- 联机前没有自动快照恢复，v1 先在联机前手动保存。
