# 暂停、RTC 与成长：源码审计证据

**后续行为变化：build 10026 默认对已验证的 Sweets 睡眠暂停补算本次睡眠的实际经过部分；普通清醒暂停仍跳过暂停间隔。本文保留原审计，当前睡眠例外见 [睡眠暂停补算](sweets-sleep-pause.md)。**

审计日期：2026-11-12。范围：当前工作区的 Swift shell、Rust Session 和 tg18 core。
本文没有运行或修改实机存档，也没有执行固件反汇编；它仅说明宿主如何给固件提供时间。
固件里年龄、成长阶段、日期副本的具体算法，需要固件静态分析和存档副本验证。

## 三种时间不能混为一谈

| 时间 | 来源与用途 | 暂停/恢复行为 |
|---|---|---|
| `care.care_at` | Unix 墙钟中的“已结算到此刻”，不是宠物累计年龄；`CareTimeline::due` 用它求需补算的间隔 | 暂停时不变；恢复时直接设成当前 Unix 时间，排除暂停间隔 |
| `machine.executed` / `sys.game_time` | CPU 执行/模拟时间；正常情况下通过 `System::advance` 累加，`turbo` 会缩放 game_time | 暂停的 pump 不执行 CPU，因此不推进；恢复函数本身也不推进它们 |
| RTC 芯片 + SoC 日内时钟 | RTC 是“基准 tick + 从基准起经过的 game_time”；SoC TOD 是“日内基准 + game_time 差值”，按 86400 取模 | 暂停期间 CPU/game_time 停止时被冻结；恢复时两者直接重定基准到手机本地时间 |

证据：`rust/shell/src/care.rs:24`、`:34`；`third_party/nijiiro-egg/core/src/sys.rs:257`、`:278`、`:804`。
本地 RTC 时间自 2007-12-31 起算，来自系统本地时区；它与 `care_at` 的 Unix epoch 不同，见 core `src/lib.rs:47`、`:54`、`:88`。

## 暂停及恢复的确定行为

1. Swift `Engine.setPaused` 写设置，然后调用 `pm_care_pause(session, paused, Date().timeIntervalSince1970)`，见 `ios/PocketMeets/PocketMeets/Engine.swift:284`。
2. `Session::care_pause` 调用 `CareTimeline::set_paused`。开启暂停会立即保存；恢复会把 `care_at` 设为 now，再调用 `sync_clock_to_wall`，见 `rust/shell/src/session.rs:379`。
3. `sync_clock_to_wall` 只写四个基准字段：RTC `base_ticks`/`base_time`、SoC `tod_base`/`tod_set_at`。它没有调用固件代码、冷启动或成长处理，没有推进 CPU/game_time，也没有直接修改固件 SRAM/flash，见 Session `:329`。
4. 暂停期间 `Session::pump` 在调用 `step` 以前直接返回，见 Session `:845`。`care_sync` 也在 paused 时直接返回，见 `:394`。因此暂停间隔不会被重放成一整晚，也不会自动运行夜间/午夜/晨起处理。
5. 仅调用公共 `Session::step` 并不检查 `care.paused`（`:454`）；正式 iOS 依赖 pump 的暂停检查。复现实验若直接用 `step`，必须由实验自己停止推进，否则并没有模拟 App 的“暂停养育”。

“恢复不主动清零成长进度”成立；“恢复完全等同于一段连续运行”不成立。
恢复会制造日历/日内时钟跳变，而累计模拟时间保持原值。
这种跳变是否造成漏加年龄、补加一岁或跳过成长事件，取决于固件使用哪种时间与事件。

## 打开存档和持续运行的区别

`open_care` 先还原 Snapshot，再加载 `care.json` 并整理暂停状态，见 Session `:245`、`:274`。
若仍暂停，`align_clock_for_open` 不移动 RTC（`:316`）；若恢复，`care_at` 已跳到 now，due 为零，所以 RTC 直接对齐手机时间。

需要补时的未暂停存档则把 RTC 设成 `wall - due`，让模拟时间追上来；补时期间暂时关闭 never-sleep，见 Session `:320`、`:853`。
正常 live pump 每约 1000 个宿主调度毫秒再次执行 `sync_clock_to_wall`，见 Session `:951`。
所以 live 流程也会持续被手机时钟纠正，不能把只用固定 RTC 基准的桌面 `step` 实验当成完全等价的 iOS 运行。

## 换日、刷新和唤醒的宿主入口

### 1 Hz 是模拟时间事件，不是 RTC 跳变补发

`System::rtc_irq_advance` 比较 `game_time / 1s` 与 `rtc_irq.fired`，并置 1 Hz 状态位，见 core `sys.rs:634`。
恢复改 RTC/TOD 不会更改 `game_time`、`fired` 或直接置位 IRQ；不会为漏掉的夜间每秒事件补发中断。
正常推进后仍会继续产生新的模拟秒中断。

### 熄屏深睡依靠 RTC alarm 或按键冷启动

`Machine::power_cycle` 只在 RTC alarm 或下一次脚本按键到达时冷启动，见 core `machine.rs:849`。
恢复没有主动改 alarm 寄存器或强制唤醒；但是如果恢复后的 RTC 已超过旧 alarm，下一次 power_cycle 计算出的 alarm 等待为 0，可能立即走 alarm 冷启动。
如果没有可用 alarm/按键，则不会凭空产生晨起执行；模拟时间可直接前进到本次 slice 结尾，见 Session `:514`、`:539`。
因此“跳过了日历时间”与“固件执行了该时段”是两件事。

### 常亮时存在按分钟主动调用固件时钟刷新函数的路径

SigCache 通过 `CLOCK_REFRESH_SIG` 找到固件函数，见 core `machine.rs:115`、`sigs.rs:85`。
当 `never_sleep` 且成功进行 idle skip 时，run loop 调用 `refresh_clock`，见 core `machine.rs:668`。
`refresh_clock` 在 SoC TOD 的日内分钟改变、CPU 不在 IRQ 模式时调用该固件函数，成功后缓存分钟，见 core `machine.rs:828`。

这个缓存只包含 0..1439 的日内分钟，未包含日期。因此若在同一进程里跨整天恢复到完全相同的日内分钟，这个主动刷新入口不会仅因日期改变而立即调用；等日内分钟再次改变仍会调用。
这只能说明“可能延迟到下一分钟”，不能据此声称日期或年龄会永久卡住。
重新打开 Session 会从新 Machine 开始，`clock_minute` 初始为 None（core `machine.rs:196`），所以不要把同一进程暂停和杀进程重开混成同一个实验。

## 不能从源码审计直接得出的结论

- 年龄数字按午夜、晨起、生日累计时长还是其他条件增加。
- 角色进化与显示年龄是否共用计数器。
- 跨日后固件是补算相差天数、只增一次，还是只识别特定时刻的边沿。
- 完整睡眠是否是进化的必要条件。
- RTC alarm 跳过多个日期后的固件处理是否健壮。

`docs/infrared.md:426` 的“跨过午夜时宠物要醒着，年龄才会增加”是旧实验归纳。
这些实验有改 RTC、调整 alarm、常亮/睡眠路径等混杂因素，不能把那句话视为固件判定条件。

## 给固件验证的具体控制项

使用同一份存档和同一成长阶段，分别观察显示年龄、真实阶段字段、成长累计字段、固件日期副本。
区分三组：自然走过午夜及晨起；CPU 完全不运行而 RTC/TOD 跨夜跳变；同样累计 CPU/game_time 但把 RTC/TOD 保持在白天。
再区分 pause/resume 同进程、杀进程重开、屏幕常亮和允许深睡，避免 `refresh_clock` 与 alarm 的差异污染结论。

测试时必须注意：`care_pause(false, supplied_now)` 中 supplied_now 只影响 care 时间；实际 RTC 仍读取真实系统本地时间，不能只给一个人工 now 就以为模拟了指定恢复日期。
为了可重复，实验应在 Session 恢复后明确设置 RTC/TOD，或用受控时钟依赖；不能修改两台实机的系统时间。
