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
| 后台保活定时任务 | `AppModel` 注册的 `app.pocketmeets.ios.care`（BGProcessing） | iOS 决定何时执行，`earliestBeginDate` 为 60 秒后，不保证触发 |
| 设置 → 立即保存 | `SettingsView` → `engine.saveNow()` | 手动，始终写入 |
| 切换 ROM | `session.switch_to_image` 先 `save()` | 切换前存当前进度，各 ROM 分别保留 |
| 连接结束（WebMeets 收礼/发送） | `WebMeetsView` 的 `saveNow()` | 连接成功完成后存 |
| 导入 / 删除存档 | `pm_import_save` / `pm_delete_current_save` | 导入覆盖当前存档前后各有写入，删除会清空 |

## 不会存档的情况

- **暂停养育**：`pump` 在 `care.paused` 时直接返回，30 秒自动存档和补时存档都不跑。手动保存、切后台保存仍然写入。
- **运行停止**（`stopped`，ROM 不可运行或蓝牙连接失败）：`save()` 直接返回错误，保护“上一次好存档”，导出时改用 `autosave.t18s`。
- **App 被系统挂起**：后台能否继续 pump 取决于 `UIBackgroundModes`（当前有 `bluetooth-peripheral`、`audio`）。没有保活时，依赖“切后台那一次存档”。

## 已知缺口

1. `Engine.closeSession()` 会 `pm_save`，但 Swift 侧没有调用，退出 App 时不会额外存一次。
2. 面对面（LAN）联机没有专门的存档点，只靠 30 秒自动存档。
3. 模拟器上 BGTaskScheduler 任务不会自动触发，`app.pocketmeets.ios.care` 只能靠调试器模拟；模拟器测试时“后台存档”实际只来自切后台那一次 `saveNow()`。
4. 帮助文案“运行中每 30 秒自动保存”在暂停养育时不成立，需要补充说明。

## 模拟器验证

- 运行中等待 30 秒以上，`autosave.t18s` 修改时间应刷新：
  `xcrun simctl get_app_container <UDID> app.pocketmeets.ios data` 下的 `Library/Application Support/PocketMeets/Saves/`
- 切后台后立即检查，修改时间应与切后台时刻接近。
