# 自动存档时机

存档由 Rust 侧 `Session::save()` 统一完成，一次写入三样东西：

- `Saves/<stem>/autosave.t18s`：游戏快照，导出备份与“上一次好存档”都读它；
- `Saves/<stem>/game.flash`：机器 flash；
- `care` 时间线（`Care/` 下的 forecast 等）。

任何触发点最终都调用这一个函数。下面按触发来源整理。

## 触发点

| 触发 | 位置 | 说明 |
|---|---|---|
| 运行中每 30 秒 | `rust/shell/src/session.rs` 的 `pump`，`tick::AUTOSAVE_MS = 30_000` | 以 `last_save_ms` 计时，只在 pump 实际运行时生效 |
| 补时（离开较久回来） | `pump` 的 catch-up 分支，`tick::CATCHUP_SAVE_MS = 10_000` | 补时过程中每 10 秒存一次，补完后再存一次 |
| 切到后台 | `AppModel.scene(.background)` → `engine.saveNow()` | 进入后台时立即存，不依赖后台是否还在跑 |
| 设置 → 立即保存 | `SettingsView` → `engine.saveNow()` | 手动，始终写入 |
| 暂停养育 | `Session::care_pause`（设置开启“暂停养育”） | 开启暂停时立即存一次，锁定进入暂停前的最新状态 |
| 联机前（面对面） | `LanLink.begin()` → `engine.saveNow()` | 局域网连接建立、准备通信前立即存一次 |
| 联机前（WebMeets / Mixing） | `AppModel.onRoute` → `engine.saveNow()` | 固件进入联机路由、跳转页面前立即存一次 |
| 切换 ROM | `session.switch_to_image` 先 `save()` | 切换前存当前进度，各 ROM 分别保留 |
| 连接结束（WebMeets 收礼/发送） | `WebMeetsView` 的 `saveNow()` | 连接成功完成后存 |
| 导入 / 删除存档 | `pm_import_save` / `pm_delete_current_save` | 导入覆盖当前存档前后各有写入，删除会清空 |

## 不会存档的情况

- **暂停养育**：`pump` 在 `care.paused` 时直接返回，30 秒自动存档和补时存档都不跑。但开启暂停的瞬间会立即自动存一次；暂停期间手动保存、切后台保存仍然写入。
- **运行停止**（`stopped`，ROM 不可运行或蓝牙连接失败）：`save()` 直接返回错误，保护“上一次好存档”，导出时改用 `autosave.t18s`。
- **App 被系统挂起**：后台能否继续 pump 取决于 `UIBackgroundModes`（当前有 `bluetooth-peripheral`、`audio`）。没有保活时，依赖“切后台那一次存档”。

## 已知缺口

1. `Engine.closeSession()` 会 `pm_save`，但 Swift 侧没有调用，退出 App 时不会额外存一次。

## 后台任务

以前注册过两个 `BGTaskScheduler` 任务（`app.pocketmeets.ios.refresh` 和
`app.pocketmeets.ios.care`）。它们只写心跳、跑一个 6 小时看门狗、再存一次档，
而 iOS 不保证执行时间；看门狗触发后也会在回前台时被立刻撤销，所以对养育没有
实际作用。现已连同“后台驻留”开关、常驻通知、心跳和看门狗一起移除。

后台养育现在只依赖：切后台时的一次存档、回前台补时，以及按预测时间排好的
本地通知。`Info.plist` 只保留 `audio`（画中画）和 `bluetooth-peripheral`
（WebMeets）两种后台模式。

## 模拟器验证

- 运行中等待 30 秒以上，`autosave.t18s` 修改时间应刷新：
  `xcrun simctl get_app_container <UDID> app.pocketmeets.ios data` 下的 `Library/Application Support/PocketMeets/Saves/`
- 切后台后立即检查，修改时间应与切后台时刻接近。
