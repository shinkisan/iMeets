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

1. 两台都点设备界面顶部的天线按钮。
2. 窗口会直接显示本机二维码；对方点“扫码加入”扫它。
3. 连上后窗口自动关闭，顶部按钮变成绿点 +“面对面”。
4. 这时就可以在游戏里操作了。

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
- 相机：真机扫码需要；Simulator 没相机，用调试参数
  `-pmLanJoin <payload>` 加入（payload 在 host 容器的
  `Documents/lan-payload.txt`）。

## 已知限制

- App 进入后台会挂起 socket；
- Simulator 没有相机，二维码扫描不可用；
- 真机与模拟器之间、两台真机之间使用同一套 TCP + tick barrier；
- 面对面联机连接成功建立时会自动保存一次进度；

## 实机联机诊断（build 10023 起默认关闭 / lan-trace-v1）

默认不记录联机诊断日志，也不进行相关 UART 字符串格式化、状态/屏幕采样和耗时统计。
需要排查时，显式添加启动参数 `-pmLanTrace`，再点 Host 或 Join 开始记录。
该开关只影响本次进程，不存入用户设置；普通重新启动会恢复关闭。
关闭不会删除之前的日志；`lan-payload.txt` 连接信息仍保留，供已有自动连接流程使用。
例如 Simulator：`xcrun simctl launch <UDID> app.pocketmeets.ios -pmLanHost -pmLanTrace`。
真机可在 Xcode 的 Scheme → Run → Arguments 添加 `-pmLanTrace` 后运行。
每次尝试生成独立 JSONL 文件，存在 App 容器的 `Documents/LanDiagnostics/`。
日志包含本次角色、安装标识、版本、墙钟/单调时钟、按键、UART 原始字节、
网络状态、tick 等待和超时，以及每秒一次的固件时间、UART 队列、IR 状态和屏幕文字。
Sweets v062 以外的固件不会按 Sweets 的内存布局解读 IR 上下文。

文件写入在独立队列运行；单文件最多 4 MiB，最多保留 12 个文件。
写入积压时会丢弃诊断事件并在下条记录标记 `dropped_events`，不会阻塞网络等待磁盘。
UART 记录最多保留每次调用的前 8192 字节，超出时标记 `truncated`。
日志包含游戏协议数据，仅保存在本机，不自动上传；连接串的 token/code 不写入这些文件。
停止或连接失败会结束当前文件；只要不卸载 App，重启后仍可取回日志。
系统控制台类别 `app.pocketmeets.ios / LAN` 会输出 `PM_LAN_LOG` 文件路径或写入错误。

注意：当前 iOS 的 20 ms tick 只限制网络循环，**没有暂停独立运行的固件 pump**。
日志记录这项现状，用于定位真机握手问题；这个诊断版本没有改变联机协议或固件时序。

两台真机安装同一版本后，同 Wi-Fi、前台、解除养育暂停，复现一次“玩耍”。
一端先按 B，另一端约 0.5 秒后按，记录开始时间，失败后先保持现场。
通过连接/配对到 Mac 的设备导出两端日志（不必保持调试器附加）：

```sh
xcrun devicectl list devices
xcrun devicectl device copy from --device <UDID> \
  --domain-type appDataContainer --domain-identifier app.pocketmeets.ios \
  --source Documents/LanDiagnostics --destination /tmp/lan-logs-phone-a
```

对另一台使用不同的目标目录。Simulator 则通过
`xcrun simctl get_app_container <UDID> app.pocketmeets.ios data` 找容器，再复制同一路径。
比对 `uart_tx` 与对端 `uart_rx` 的字节流（分块可以不同），结合 `sim_s`、
`pump.dt_ms`、`transport_tick.wait_ms`、队列长度和 B 键时间判断首包、重传和超时的位置。
跨设备的 `uptime_s` 不能直接相减；它只用于本机耗时，跨设备关联使用字节流与墙钟。
