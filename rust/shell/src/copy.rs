//! User-facing sentences. English matches the spec's quoted copy.
//! `zh` is the default chrome. The ROM language is never changed here.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn parse(s: &str) -> Option<Lang> {
        match s {
            "zh" => Some(Lang::Zh),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    pub fn from_ffi(n: i32) -> Lang {
        if n == 1 {
            Lang::En
        } else {
            Lang::Zh
        }
    }
}

struct Row {
    key: &'static str,
    zh: &'static str,
    en: &'static str,
}

const ROWS: &[Row] = &[
    Row { key: "app_name", zh: "iMeets", en: "iMeets" },
    Row { key: "notes_title", zh: "1.0.1", en: "1.0.1" },
    Row {
        key: "notes_body",
        zh: "第一次使用后，游戏内的相机可以再次打开。拓麻书架会区分“还不是成人”和“已经使用了成长道具、进化还没完成”。覆盖安装会保留 ROM、存档和设置。",
        en: "The in-game camera can be opened again after the first use. Tama Shelf distinguishes “not yet adult” from “age-up done, evolution not finished”. Installing over a previous build keeps ROMs, saves, and settings.",
    },
    Row { key: "nothing_imported", zh: "尚未导入", en: "nothing imported yet" },
    Row { key: "no_rom_imported", zh: "还没有导入 ROM", en: "no ROM imported yet" },
    Row { key: "import_rom_first", zh: "请先导入 ROM。", en: "Import a ROM first." },
    Row { key: "choose_rom", zh: "请选择支持的 Meets / On ROM。", en: "choose a supported Meets / On ROM." },
    Row { key: "import_failed_again", zh: "导入失败，请重新选择", en: "import failed, choose again" },
    Row { key: "cannot_read_zip", zh: "无法读取这个 ZIP", en: "cannot read this ZIP" },
    Row { key: "zip_too_large", zh: "ZIP 太大或文件太多", en: "ZIP too large or too many files" },
    Row { key: "zip_no_rom", zh: "ZIP 里没有支持的 ROM", en: "ZIP contains no supported ROM" },
    Row { key: "local_rom_kept", zh: "已保留本地 ROM", en: "local ROM kept" },
    Row { key: "err_not_8mib", zh: "Expected an 8 MiB Meets/On SPI flash dump.", en: "Expected an 8 MiB Meets/On SPI flash dump." },
    Row { key: "err_unsupported", zh: "This firmware version is not supported by tg18-emu yet.", en: "This firmware version is not supported by tg18-emu yet." },
    Row { key: "switch_save_prefix", zh: "当前进度无法保存，已取消切换", en: "current progress could not be saved; switch cancelled" },
    Row { key: "switch_kept_prefix", zh: "无法切换，原来的 ROM 和进度已保留", en: "could not switch; the original ROM and progress were kept" },
    Row { key: "loaded_prefix", zh: "已载入：", en: "loaded: " },
    Row { key: "mixing_busy", zh: "请先结束或取消 Mixing Meets", en: "finish or cancel Mixing Meets first" },
    Row { key: "emulator_stopped", zh: "模拟器已停止", en: "the emulator stopped" },
    Row { key: "unsupported_link", zh: "此版本不支持这个连接，已回到上一个画面", en: "this version does not support this connection; returned to the previous screen" },
    Row { key: "link_native", zh: "Link functions are not available yet. Returned to the previous screen.", en: "Link functions are not available yet. Returned to the previous screen." },
    Row { key: "link_stopped_firmware", zh: "A link function stopped the firmware. Reopen the ROM to resume your save.", en: "A link function stopped the firmware. Reopen the ROM to resume your save." },
    Row { key: "no_recovery", zh: "No connection recovery point is available", en: "No connection recovery point is available" },
    Row { key: "link_restored", zh: "连接已中断，进度已恢复到连接前", en: "the connection was interrupted and pre-connection progress was restored" },
    Row { key: "progress_saved", zh: "进度已保存", en: "progress saved" },
    Row { key: "save_failed_prefix", zh: "保存失败：", en: "save failed: " },
    Row { key: "save_exported", zh: "存档已导出", en: "save exported" },
    Row { key: "save_imported", zh: "存档已导入，可以继续", en: "save imported, you can continue" },
    Row { key: "import_failed", zh: "导入失败", en: "import failed" },
    Row { key: "choose_matching_save", zh: "choose a .t18s that matches the current ROM version.", en: "choose a .t18s that matches the current ROM version." },
    Row { key: "different_rom", zh: "it belongs to a different ROM version", en: "it belongs to a different ROM version" },
    Row { key: "export_last_good", zh: "export the last good autosave instead", en: "export the last good autosave instead" },
    Row { key: "delete_ok", zh: "当前存档已删除，机器已重置", en: "current save deleted, the machine was reset" },
    Row { key: "delete_failed_prefix", zh: "删除失败：", en: "delete failed: " },
    Row { key: "delete_confirm", zh: "确定删除当前 ROM 的存档并重新开始吗？此操作无法撤销。其他 ROM 的存档和已保存的截图不受影响。", en: "Delete the save for the current ROM and start over? This cannot be undone. Saves for other ROMs and your screenshots are kept." },
    Row { key: "delete_yes", zh: "删除并重置", en: "Delete and reset" },
    Row { key: "delete_alert_title", zh: "删除存档？", en: "Delete this save?" },
    Row { key: "cancel", zh: "取消", en: "Cancel" },
    Row { key: "resume_pet_first", zh: "请先恢复你的宠物", en: "resume your pet first" },
    Row { key: "shelf_blocked", zh: "书架暂不可用", en: "Shelf blocked." },
    Row { key: "already_sent", zh: "已经发送过了", en: "Already sent." },
    Row { key: "slots_full", zh: "下载栏已满，请先在游戏里整理", en: "download slots are full; tidy them in the game" },
    Row { key: "balance_range", zh: "balance must be 0–99999", en: "balance must be 0–99999" },
    Row { key: "sanrio_first", zh: "Switch to Sanrio first", en: "Switch to Sanrio first" },
    Row { key: "evolution_pending", zh: "进化尚未完成。请关闭书架，回到主画面，完成进化后再结婚。", en: "evolution is pending; close the shelf, return to the home screen, finish evolving, then marry." },
    Row { key: "marriage_young", zh: "结婚需要成人形态。魔法屋一次只成长一个阶段。", en: "marriage needs the adult form; Magic Hut grows one stage at a time." },
    Row { key: "growth_unavailable", zh: "无法读取宠物的成长状态。", en: "Pet growth state is unavailable." },
    Row { key: "invalid_pet", zh: "宠物状态无效", en: "invalid pet state" },
    Row { key: "home_screen", zh: "请回到宠物的主画面后再试", en: "return to the pet home screen and retry" },
    Row { key: "screenshot_failed_prefix", zh: "截图失败：", en: "screenshot failed: " },
    Row { key: "screenshot_ok", zh: "已保存到 Pictures/iMeets", en: "saved to Pictures/iMeets" },
    Row { key: "call_title", zh: "拓麻歌子在叫你", en: "your Tamagotchi is calling" },
    Row { key: "call_body", zh: "它刚刚醒来并叫了你，点按查看。", en: "it just woke and called; tap to look" },
    Row { key: "forecast_stopped", zh: "固件预测在下一次呼叫前停止了。", en: "Firmware forecast stopped before its next call." },
    Row { key: "gift_saved", zh: "礼物已收到，进度已保存", en: "gift received, progress saved" },
    Row { key: "conn_interrupted", zh: "连接已中断，进度已恢复到连接前", en: "connection interrupted, progress restored to before the connection" },
    Row { key: "ble_failed", zh: "蓝牙接收失败，这次连接已结束", en: "Bluetooth receive failed, this connection has ended" },
    Row { key: "ble_timeout", zh: "等太久了，这次接收已结束", en: "waited too long, this receive has ended" },
    Row { key: "ble_name_fail", zh: "无法设置蓝牙名称", en: "could not set the Bluetooth name" },
    Row { key: "ble_no_radio", zh: "这台设备没有 LE 广播，无法接收礼物", en: "this device has no LE advertising; gifts cannot be received" },
    Row { key: "ble_off", zh: "蓝牙已关闭，这次接收已结束", en: "Bluetooth is off; this receive has ended" },
    Row { key: "ble_permission", zh: "需要附近设备权限", en: "nearby-devices permission is required" },
    Row { key: "ble_waiting", zh: "等待 WebMeets 连接 · TMGC_meets", en: "等待 WebMeets 连接 · TMGC_meets" },
    Row { key: "service_not_ready", zh: "正在连接照顾服务，请再试一次。进度已保存。", en: "Connecting the care service; try again. Progress was saved." },
    Row { key: "load_pack_first", zh: "请先载入素材 ZIP", en: "load a material ZIP first" },
    Row { key: "chest_sentence", zh: "已送到你的宝箱，可以拿来使用或穿戴", en: "sent to your chest to use or wear" },
    Row { key: "food_sentence", zh: "已送到餐厅菜单，可以购买", en: "sent to the restaurant menu to be bought" },
    Row { key: "not_a_match", zh: "Sorry, not a match!", en: "Sorry, not a match!" },
    // TODO(propose-ok-wording): SPEC §10.4 names the key. The success sentence was not recovered.
    Row { key: "propose_ok", zh: "propose_ok", en: "propose_ok" },
    Row { key: "propose_ng", zh: "抱歉，不是合适的对象！", en: "Sorry, not a match!" },
    Row { key: "network_error", zh: "网络错误", en: "network error" },
    Row { key: "keep_display_lit", zh: "显示常亮", en: "keep the display lit" },
    Row { key: "alarm_exact", zh: "呼叫使用精确定时", en: "notifications use an exact alarm" },
    Row { key: "alarm_inexact", zh: "通知可能会晚到", en: "notifications may be late" },
    Row { key: "back_confirm", zh: "现在结束会恢复到连接前的进度，并丢弃外出奖励。想保留奖励，请在游戏里选择返回。", en: "Ending now restores pre-connection progress and discards outing rewards. Keeping rewards is done with the in-game Return." },
    Row { key: "shelf_title", zh: "拓麻书架", en: "Tama Shelf" },
    Row { key: "tab_wallpaper", zh: "壁纸", en: "Wallpapers" },
    Row { key: "tab_food", zh: "食物", en: "Meals and snacks" },
    Row { key: "tab_item", zh: "物品", en: "Items" },
    Row { key: "tab_accessory", zh: "装饰", en: "Accessories" },
    Row { key: "tab_game", zh: "游戏", en: "Mini-games" },
    Row { key: "tab_station", zh: "月台", en: "Destinations" },
    Row { key: "magic_hut", zh: "请在宠物的主画面、对已经是成人的角色使用。魔法屋成长之后，请先完成进化。发送前会保留一份备份。", en: "Use this with an adult on the pet’s home screen. After Magic Hut aging, finish evolving first. A backup is kept before sending." },
    Row { key: "stage_egg", zh: "蛋", en: "Egg" },
    Row { key: "stage_baby", zh: "婴儿", en: "Baby" },
    Row { key: "stage_child", zh: "儿童", en: "Child" },
    Row { key: "stage_teen", zh: "少年", en: "Teen" },
    Row { key: "sigs_unavailable", zh: "这个固件还没有书架签名，无法定位存档字段。", en: "shelf signatures are not available for this firmware" },
    Row { key: "pose_unknown", zh: "姿态 PNG 的像素格式未知，无法合成角色。", en: "pose PNG encoding is unknown; the character was not synthesized" },
    Row { key: "null_control", zh: "null session or request", en: "null session or request" },
    Row { key: "read_limit", zh: "read is at most 65536 bytes", en: "read is at most 65536 bytes" },
    Row { key: "advance_limit", zh: "advance is at most 60 seconds", en: "advance is at most 60 seconds" },
    Row { key: "scale_limit", zh: "clock scale must be finite and positive", en: "clock scale must be finite and positive" },
    Row { key: "rtc_limit", zh: "RTC value stays in a 48-bit range", en: "RTC value stays in a 48-bit range" },
    Row { key: "key_limit", zh: "key index is 0, 1, or 2", en: "key index is 0, 1, or 2" },
    Row { key: "confirm_wake", zh: "确认 / 唤醒", en: "confirm / wake" },
    Row { key: "small_a11y", zh: "左侧 A 选择，中间 B 确认，右下三分之二是 C，右上角长按回到简洁界面。支持双指同时按住 A 和 C。", en: "Left A selects, middle B confirms, lower-right two thirds is C, upper-right third long-press returns to minimal, and a two-finger A+C chord is supported." },
    Row { key: "guide_body", zh: "小屏模式：点按左、中、右操作。右上角按住约一秒可离开。", en: "Small screen: tap left, middle, and right. Hold the upper-right corner for about a second to leave." },
    // TODO(double-tap-wording): SPEC §7.2 names a double-tap hint and does not quote it.
    Row { key: "double_tap_hint", zh: "双击画面，或在设置中选择截取当前画面。", en: "Double-tap the LCD, or choose capture the current screen in Settings." },
    Row { key: "pause", zh: "暂停", en: "Pause" },
    Row { key: "resume_care", zh: "继续养育", en: "Resume care" },
    Row { key: "mixing_title", zh: "Mixing Meets", en: "Mixing Meets" },
    Row { key: "toy", zh: "玩具", en: "Toy" },
    Row { key: "back_to_app", zh: "返回 iMeets", en: "Back to iMeets" },
    Row { key: "about_simple_body", zh: "iMeets 与任何 Bandai 官方产品无关，也未获 Bandai 认可。您需要从自己的设备中提取固件。", en: "iMeets is not affiliated with or endorsed by Bandai. You need to extract the firmware from your own device." },
    Row { key: "about_core_line", zh: "内核：nijiiro-egg（MIT）", en: "Core: nijiiro-egg (MIT)" },
    Row { key: "about_sibling_line", zh: "安卓上的姊妹项目：Open Meets", en: "Sister project on Android: Open Meets" },
    Row { key: "about_body", zh: "iMeets 1.1（构建 10019）。这不是 Open Meets，也没有得到万代的认可或授权。固件需要你自己提供。", en: "iMeets 1.1 (build 10019). This is not Open Meets and is not endorsed by Bandai. Firmware is supplied by you." },
    Row { key: "mit_notice", zh: "模拟器核心 tg18-emu（nijiiro-egg）以 MIT 许可证发布。Copyright (c) 2026 oreo-fox。", en: "The emulator core tg18-emu (nijiiro-egg) is MIT licensed. Copyright (c) 2026 oreo-fox." },
    Row { key: "fw_update_log", zh: "虚拟控制器不接受固件更新。", en: "the virtual controller does not accept firmware updates" },
    Row { key: "switch_starting", zh: "正在切换 ROM", en: "a switch is starting" },
    Row { key: "help_web", zh: "在另一台设备的 Chrome 打开 tamagotchicenter.com/webmeets/items，选一个物品，再选名为 TMGC_meets 的设备。发送端需要网络。这台手机不下载物品目录。名称是临时的。", en: "On a Mac or another device, open Chrome at tamagotchicenter.com/webmeets/items, pick an item, and choose the device TMGC_meets. The sending page needs a network. This phone does not download an item catalog. The name is temporary." },
    Row { key: "photo_permission", zh: "需要照片权限才能保存截图。", en: "Photo library permission is required to save." },
    Row { key: "wallpaper_encode", zh: "壁纸容器已保存。游戏内的像素编码未知，画面可能不会按原图显示。", en: "The wallpaper container was saved. The in-game pixel encoding is unknown, so the picture may not appear as drawn." },
    Row { key: "done", zh: "完成", en: "Done" },
    Row { key: "back", zh: "返回", en: "Back" },
    Row { key: "about_title", zh: "关于", en: "About" },
    Row { key: "version", zh: "版本", en: "Version" },
    Row { key: "build", zh: "构建", en: "Build" },
    Row { key: "ok", zh: "好", en: "OK" },
    Row { key: "end", zh: "结束", en: "End" },
    Row { key: "about_license", zh: "iMeets 外壳代码以 GPL-3.0 发布。模拟器核心仍为 MIT，不随之重新授权。", en: "iMeets shell code is GPL-3.0. The emulator core stays MIT and is not relicensed." },
    Row { key: "about_font", zh: "使用系统字体。不包含角色美术资源，也不包含固件。", en: "The system font is used. No character art and no firmware are included." },
    Row { key: "about_drawn", zh: "为 iMeets 绘制。", en: "Drawn for iMeets." },
    Row { key: "setup_start", zh: "开始养育", en: "Start caring" },
    Row { key: "setup_no_bundle", zh: "这个版本不包含任何 ROM 或测试存档。第一次使用，请从本机选择你自己的 ROM。", en: "This build includes no ROM and no test save. The first time, choose your own ROM on this device." },
    Row { key: "setup_import", zh: "导入你的 ROM", en: "Import your ROM" },
    Row { key: "setup_import_body", zh: "可选择 ROM 或 ZIP 压缩包。应用会自动识别包内的 ROM，保存在 ROMs 文件夹，供你随时切换。已有存档会保留。", en: "Choose a ROM or a ZIP. ROMs inside the archive are recognized, stored in the ROMs folder, and can be switched later. Existing saves are kept." },
    Row { key: "pick_rom", zh: "选择 ROM 文件", en: "Choose a ROM file" },
    Row { key: "setup_shelf_body", zh: "可选：加载素材 ZIP，收藏礼物并搭配结婚角色。也可以稍后在设置中开启。", en: "Optional: load a material ZIP, keep gifts, and assemble a marriage character. You can turn this on later in Settings." },
    Row { key: "shelf_enable", zh: "开启拓麻书架", en: "Turn on Tama Shelf" },
    Row { key: "load_pack_optional", zh: "加载素材 ZIP（可选）", en: "Load a material ZIP (optional)" },
    Row { key: "setup_settings", zh: "设置", en: "Settings" },
    Row { key: "language_title", zh: "语言 / Language", en: "语言 / Language" },
    Row { key: "language_note", zh: "切换应用界面与通知语言，不改变游戏 ROM 内的文字。", en: "This changes the app and notification language. It does not change the language inside the ROM." },
    Row { key: "help_title", zh: "使用说明", en: "How to use" },
    Row { key: "notes_heading", zh: "1.0.1 本次更新", en: "1.0.1 this update" },
    Row { key: "notes_camera_title", zh: "相机可以重复使用", en: "The camera can be used again" },
    Row { key: "notes_camera_body", zh: "第一次拍照之后可以再次进入、拍摄和保存照片。", en: "After the first photo you can enter, shoot, and save again." },
    Row { key: "notes_hut_title", zh: "魔法小屋与结婚", en: "Magic Hut and marriage" },
    Row { key: "notes_hut_body", zh: "书架会区分尚未成年和加龄后进化未完成，并显示对应提示。使用沙漏后，请先回到主画面完成进化，再发送婚姻角色。", en: "The shelf distinguishes not yet adult from age-up done, evolution not finished, and shows the matching message. After the hourglass, return to the home screen, finish evolving, then send the marriage character." },
    Row { key: "notes_keep_title", zh: "继续陪伴原来的宝宝", en: "Keep the same pet" },
    Row { key: "notes_keep_body", zh: "直接覆盖安装即可保留已有 ROM、存档和设置。", en: "Installing over this build keeps your ROMs, saves, and settings." },
    Row { key: "help_controls_title", zh: "操作", en: "Controls" },
    Row { key: "help_controls_body", zh: "A 选择，B 确认，C 返回。支持同时按 A + C，也可使用键盘 A / S / D。游戏屏幕休眠时，按任意键唤醒。", en: "A selects, B confirms, C goes back. A and C can be held together. The keyboard uses A / S / D. Any key wakes the sleeping screen." },
    Row { key: "help_save_title", zh: "保存", en: "Saving" },
    Row { key: "help_save_body", zh: "养育运行中每 30 秒自动保存，切到后台时也会保存。可在设置中立即保存、导出备份或导入同版本的存档。", en: "While care is running, progress is saved every 30 seconds, and again when the app goes to the background. Settings can save now, export a backup, or import a save from the same version." },
    Row { key: "help_care_title", zh: "养育模式与提醒", en: "Care and reminders" },
    Row { key: "help_care_body", zh: "关闭程序后养育时间仍与现实时间同步，重新打开时补上离线成长。请允许通知。自动唤醒并发出呼叫声时提醒，普通计时唤醒不提醒；五分钟内的重复呼叫合并。游戏内需保持声音开启；应用静音不影响检测。", en: "While the app is closed, time still matches the wall clock, and growth catches up the next time you open it. Allow notifications. A wake that plays the call sound notifies; an ordinary timed wake does not. Repeat calls inside five minutes collapse into one. Leave the in-game sound on; app mute does not stop detection." },
    Row { key: "help_pause_title", zh: "暂停", en: "Pause" },
    Row { key: "help_pause_body", zh: "设置里可以暂停养育。暂停时养育停止成长和照料提醒，重新打开程序也保持暂停，玩具自己的时钟也一起停住。继续养育后不补算暂停时长，时钟会对齐回手机时间。系统结束 App 后，这段时间不会响铃，重新打开后恢复照顾。", en: "Settings can pause care. While paused, growth and care reminders stop, pause stays if you reopen the app, and the toy's own clock stops with it. Resuming does not make up the paused time, and the clock lines back up with the phone. If the system stops the app, alarms do not run until the next open." },
    Row { key: "help_look_title", zh: "外观", en: "Appearance" },
    Row { key: "help_look_body", zh: "可在设置中切换真机、极简与小屏模式，不影响游戏进度。小屏模式可轻触游戏画面的左、中、右三分之一操作 A、B、C。", en: "Settings can switch device, minimal, and small-screen mode without changing progress. In small-screen mode the left, middle, and right thirds of the picture are A, B, and C." },
    Row { key: "help_link_title", zh: "联机", en: "Linking" },
    Row { key: "help_link_body", zh: "在游戏内选择连接 → 下载（Download），允许蓝牙。在 Mac 或另一台设备的 Chrome 中打开 WebMeets，选择物品并连接 TMGC_meets。发送网页需要联网；本机无需下载物品库。连接结束后自动保存，取消或中断会恢复连接前的进度。去玩和接收礼物进入 Mixing Meets。暂不支持与实体机器连接和红外联机。", en: "In the game choose Connect → Download and allow Bluetooth. On a Mac or another device, open WebMeets in Chrome, pick an item, and connect to TMGC_meets. The sending page needs a network. This device does not download an item catalog. A finished connection is saved. Cancel or an interruption restores the progress from before the connection. Play and Receive a gift open Mixing Meets. A physical toy and infrared are not hosted." },
    Row { key: "help_rom_title", zh: "ROM 与语言", en: "ROMs and language" },
    Row { key: "help_rom_body", zh: "本版本不附带 ROM 或测试存档。首次启动需导入你自己的 ROM，也可直接选择 ZIP，自动提取其中受支持的 ROM。导入的 ROM 按版本去重；覆盖升级会保留 ROM 和进度。设置 → 切换 ROM 会先保存当前进度，再载入所选 ROM 及其已有存档；没有存档则开始新的游戏。各版本分别保存，同版本共用一份进度。包内只有一个版本会直接载入，多个版本会全部保存后供你选择。只有当前 ROM 继续养育，未选中的进度只在回来时补时钟，不补算养育。切换失败会保留原来的 ROM 和进度。设置 → 语言可切换中文或英文，仅改变应用界面，不改变 ROM 内的语言。", en: "This build includes no ROM and no test save. Import your own ROM, or a ZIP. Supported ROMs in the archive are extracted. Imports are deduplicated by version, and an upgrade keeps ROMs and progress. Settings → Switch ROM saves the current progress first, then loads that ROM and its save, or starts fresh. Each version has its own progress. One ROM in a ZIP loads immediately. Several are stored and then you pick one. Only the current ROM keeps growing. A ROM you left catches its clock up when you return, and does not replay care. A failed switch keeps the previous ROM and progress. Settings → Language switches Chinese or English for the app only." },
    Row { key: "appearance_shell", zh: "真机", en: "Device" },
    Row { key: "appearance_minimal", zh: "极简", en: "Minimal" },
    Row { key: "appearance_small", zh: "小屏", en: "Small screen" },
    Row { key: "appearance_day", zh: "白天", en: "Light" },
    Row { key: "appearance_night", zh: "夜间", en: "Dark" },
    Row { key: "appearance_minimal_night", zh: "极简夜间", en: "Minimal night" },
    Row { key: "save_now", zh: "立即保存", en: "Save now" },
    Row { key: "export_backup", zh: "导出存档", en: "Export save" },
    Row { key: "import_save", zh: "导入存档", en: "Import a save" },
    Row { key: "switch_rom", zh: "切换 ROM", en: "Switch ROM" },
    Row { key: "dot_matrix_label", zh: "点阵屏幕", en: "LCD pixel grid" },
    Row { key: "key_haptics_label", zh: "按键震动", en: "Button vibration" },
    Row { key: "fast_keys_label", zh: "快速按键（实验）", en: "Faster buttons (experimental)" },
    Row { key: "fast_keys_note", zh: "缩短固件内部的按键判定时间，让按键更跟手。只在固件与预期一致时生效，不会改动 ROM 文件。", en: "Shortens the firmware's own key debounce so presses register sooner. Only applies when the firmware matches what it expects, and never changes the ROM file." },
    Row { key: "screenshots_label", zh: "截图按钮", en: "Screenshot button" },
    Row { key: "mute_label", zh: "静音", en: "Mute" },
    Row { key: "delete_save", zh: "删除当前存档", en: "Delete the current save" },
    Row { key: "capture_screen", zh: "截取当前画面", en: "Capture the current screen" },
    Row { key: "sound_label", zh: "声音", en: "Sound" },
    Row { key: "pause_care", zh: "暂停养育", en: "Pause care" },
    Row { key: "pip_background_toast", zh: "将在应用进入后台时显示画中画", en: "Picture in Picture will show when backgrounded" },
    Row { key: "pip_disabled_toast", zh: "已关闭后台画中画", en: "Background Picture in Picture turned off" },
    Row { key: "appearance_auto", zh: "随系统切换", en: "Follow system" },
    Row { key: "appearance_auto_hint", zh: "系统切到深色时用夜间，浅色时用真机", en: "Night when the system is dark, the shell look when it is light" },
    Row { key: "settings_display", zh: "外观与显示", en: "Appearance & display" },
    Row { key: "settings_sound", zh: "声音与振动", en: "Sound & haptics" },
    Row { key: "settings_system", zh: "系统", en: "System" },
    Row { key: "settings_other", zh: "其它", en: "Other" },
    Row { key: "game_settings", zh: "游戏设置", en: "Game settings" },
    Row { key: "progress_section", zh: "进度管理", en: "Your progress" },
    Row { key: "help_section", zh: "帮助", en: "Help" },
    Row { key: "switch_note", zh: "切换前自动保存，各版本分别保留进度。切回来，接着养。", en: "Progress is saved before switching and kept separately for each version. Switch back to continue." },
    Row { key: "shelf_status_on", zh: "已开启", en: "On" },
    Row { key: "shelf_status_off", zh: "未开启", en: "Off" },
    Row { key: "shelf_not_loaded", zh: "尚未加载素材包", en: "No material pack loaded" },
    Row { key: "shelf_loaded", zh: "素材包已放入书架", en: "Material pack added to Shelf" },
    Row { key: "shelf_loading", zh: "正在加载素材包…", en: "Loading material pack…" },
    Row { key: "gift_suffix", zh: " 件礼物", en: " gifts" },
    Row { key: "shelf_replace", zh: "加载 / 更换素材 ZIP", en: "Load / replace material ZIP" },
    Row { key: "shelf_open_action", zh: "打开书架", en: "Open Shelf" },
    Row { key: "shelf_small_note", zh: "小屏模式不显示左上角按钮，可从这里进入。发送时自动识别当前 ROM 的数据位置。", en: "Small-screen mode keeps the top-left button hidden; open Shelf here. Shelf locates the current ROM’s data when sending." },
    Row { key: "shelf_close", zh: "关闭书架", en: "Close Shelf" },
    Row { key: "shelf_marriage", zh: "婚姻", en: "Marriage" },
    Row { key: "shelf_import", zh: "导入", en: "Import" },
    Row { key: "shelf_fill", zh: "加载素材包后，礼物和角色部件会摆上书架。", en: "Load your material pack to fill the shelves." },
    Row { key: "shelf_layer_empty", zh: "这一层暂时是空的。", en: "This shelf is empty." },
    Row { key: "shelf_money", zh: "改钱", en: "Money" },
    Row { key: "money_hint", zh: "输入 0–99999，可增加金额或直接设置余额。", en: "Enter 0–99999 to add points or set the balance." },
    Row { key: "money_add", zh: "增加这笔钱", en: "Add these points" },
    Row { key: "money_set", zh: "将余额设为此金额", en: "Set balance to this amount" },
    Row { key: "shelf_lang", zh: "素材语言", en: "Resource language" },
    Row { key: "shelf_pack_row", zh: "素材包与开关", en: "Materials & settings" },
    Row { key: "part_body", zh: "身体", en: "Body" },
    Row { key: "part_face", zh: "面部", en: "Face" },
    Row { key: "part_head", zh: "头饰", en: "Head" },
    Row { key: "part_extra", zh: "装饰物", en: "Accessory" },
    Row { key: "more_options", zh: "更多选项", en: "More options" },
    Row { key: "send_method", zh: "发送方式", en: "Send mode" },
    Row { key: "send_marry", zh: "婚姻角色", en: "Marry character" },
    Row { key: "send_obtain", zh: "获得角色", en: "Get character" },
    Row { key: "send_random", zh: "随机基因", en: "Random genes" },
    Row { key: "send_marry_body", zh: "与预览角色结婚，混合双方基因。", en: "Marry the preview character and mix both parents’ genes." },
    Row { key: "send_obtain_body", zh: "使用 MyMeets 的获得角色遗传模式。", en: "Use MyMeets’ Get Character inheritance mode." },
    Row { key: "send_random_body", zh: "随机选择遗传基因组合。", en: "Randomize the inherited gene combination." },
    Row { key: "marry_gate", zh: "进化至成年形态后，在宠物主画面使用。魔法小屋加龄后需先完成进化。发送前会保留一份进度备份。", en: "Use with an adult pet on its home screen. After aging in Magic Hut, finish evolving first. A save backup is kept before sending." },
    Row { key: "send_action", zh: "发送", en: "Send" },
    Row { key: "send_food", zh: "发送到餐厅菜单，在游戏里购买食用。", en: "Sent to the restaurant menu for purchase." },
    Row { key: "send_wear", zh: "发送到宝箱，在游戏里选择使用或佩戴。", en: "Sent to your treasure chest to use or wear." },
    Row { key: "send_room", zh: "发送到装修菜单，在游戏里更换房间。", en: "Sent to the room decoration menu." },
    Row { key: "send_download", zh: "发送到游戏内的下载列表。请先回到宠物主画面。", en: "Sent to the in-game download list. Return to the pet home screen first." },
    Row { key: "price_prefix", zh: "游戏内价格：", en: "In-game price: " },
    Row { key: "sex_child", zh: "性别 · 子代", en: "Gender · offspring" },
    Row { key: "sex_boy", zh: "男孩", en: "Boy" },
    Row { key: "sex_girl", zh: "女孩", en: "Girl" },
    Row { key: "sex_random", zh: "随机性别", en: "Random gender" },
    Row { key: "parts_colors", zh: "搭配与颜色", en: "Parts & colors" },
    Row { key: "random_character", zh: "随机生成", en: "Random character" },
    Row { key: "random_colors", zh: "随机颜色", en: "Random colors" },
    Row { key: "color_body", zh: "身体颜色", en: "Body color" },
    Row { key: "color_head", zh: "头饰颜色", en: "Head color" },
    Row { key: "color_extra", zh: "装饰颜色", en: "Accessory color" },
    Row { key: "choose_color", zh: "选择颜色", en: "Choose color" },
    Row { key: "character_options", zh: "角色设定", en: "Character options" },
    Row { key: "shelf_lang_note", zh: "这里选择 MyMeets 素材与角色名称的语言。", en: "This selects the language of MyMeets resources and character names." },
    Row { key: "import_collection", zh: "自己的收藏", en: "Your own collection" },
    Row { key: "import_resource", zh: "导入物品资源文件", en: "Import resource file" },
    Row { key: "import_wallpaper", zh: "用图片制作背景", en: "Make wallpaper from an image" },
    Row { key: "import_character", zh: "导入原创角色", en: "Import original character" },
    Row { key: "import_character_note", zh: "角色支持 32 KB 角色文件，或含 character.json 和姿态 PNG 的 ZIP。", en: "Characters: a 32 KB character file, or a ZIP with character.json and pose PNGs." },
    Row { key: "restore_shelf", zh: "恢复最近一次书架操作前的进度", en: "Restore the last pre-Shelf save" },
    Row { key: "look_style", zh: "界面风格", en: "Appearance" },
    Row { key: "small_mode", zh: "小屏模式", en: "Small screen" },
    Row { key: "look_heading", zh: "三种外观，同一只拓麻歌子", en: "Three styles, one pet" },
    Row { key: "look_body", zh: "真机是圆角外壳和三个圆按钮。极简是纯色背景、大屏幕和圆按钮。小屏模式默认尽量显示完整方形画面，可调整大小。左侧 A，中间 B，右下 C，右上长按回到极简。", en: "Device is a rounded shell with three round buttons. Minimal is a plain background, a large display and round buttons. Small screen starts as a large square and can be resized. Left is A, middle is B, lower right is C, and a long-press at the upper right returns to Minimal." },
    Row { key: "look_hint", zh: "点选即刻生效，不影响游戏进度。", en: "Changes apply instantly without affecting your progress." },
    Row { key: "small_adjust", zh: "调整大小", en: "Resize" },
    Row { key: "small_pinch", zh: "在预览中双指捏合，调整画面大小", en: "Pinch the preview to resize your game" },
    Row { key: "small_reset", zh: "恢复默认", en: "Reset size" },
    Row { key: "support_mixing", zh: "支持 Mixing Meets 社区", en: "Support the Mixing Meets community" },
    Row { key: "color_0", zh: "黄色", en: "Yellow" },
    Row { key: "color_1", zh: "红色", en: "Red" },
    Row { key: "color_2", zh: "蓝色", en: "Blue" },
    Row { key: "color_3", zh: "灰色", en: "Gray" },
    Row { key: "color_4", zh: "白色", en: "White" },
    Row { key: "color_5", zh: "粉色", en: "Pink" },
    Row { key: "color_6", zh: "橙色", en: "Orange" },
    Row { key: "color_7", zh: "奶油色", en: "Cream" },
    Row { key: "color_8", zh: "绿色", en: "Green" },
    Row { key: "color_9", zh: "天蓝色", en: "Sky" },
    Row { key: "color_10", zh: "紫色", en: "Purple" },
    Row { key: "color_11", zh: "珍珠色", en: "Pearl" },
    Row { key: "color_12", zh: "原色", en: "Original" },
];

pub fn text(lang: Lang, key: &str) -> &'static str {
    for row in ROWS {
        if row.key == key {
            return match lang {
                Lang::Zh => row.zh,
                Lang::En => row.en,
            };
        }
    }
    ""
}

pub fn text_arg(lang: Lang, key: &str, arg: &str) -> String {
    let base = text(lang, key);
    if base.is_empty() {
        return arg.to_string();
    }
    format!("{base}{arg}")
}

/// Maps an English shell sentence back to the requested language. Messages are
/// produced deep in the shell where the stored language is not threaded
/// through, so the FFI boundary calls this before handing text to the app.
/// An exact row wins; otherwise the longest matching prefix is translated and
/// any runtime detail is kept as-is.
pub fn localize(lang: Lang, text: &str) -> String {
    if lang == Lang::En || text.is_empty() {
        return text.to_string();
    }
    if let Some(row) = ROWS.iter().find(|row| row.en == text) {
        return row.zh.to_string();
    }
    let mut best: Option<&Row> = None;
    for row in ROWS {
        if row.en.len() < 8 || !text.starts_with(row.en) {
            continue;
        }
        if best.map_or(true, |b| row.en.len() > b.en.len()) {
            best = Some(row);
        }
    }
    match best {
        Some(row) => format!("{}{}", row.zh, &text[row.en.len()..]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_sentences_exist_in_english() {
        let keys = [
            "nothing_imported",
            "no_rom_imported",
            "import_rom_first",
            "choose_rom",
            "cannot_read_zip",
            "zip_too_large",
            "zip_no_rom",
            "local_rom_kept",
            "import_failed_again",
            "err_not_8mib",
            "err_unsupported",
            "switch_save_prefix",
            "switch_kept_prefix",
            "loaded_prefix",
            "mixing_busy",
            "emulator_stopped",
            "unsupported_link",
            "link_native",
            "link_stopped_firmware",
            "no_recovery",
            "progress_saved",
            "save_failed_prefix",
            "save_exported",
            "save_imported",
            "import_failed",
            "choose_matching_save",
            "different_rom",
            "export_last_good",
            "delete_ok",
            "delete_failed_prefix",
            "resume_pet_first",
            "already_sent",
            "slots_full",
            "balance_range",
            "sanrio_first",
            "evolution_pending",
            "marriage_young",
            "growth_unavailable",
            "invalid_pet",
            "home_screen",
            "screenshot_failed_prefix",
            "screenshot_ok",
            "call_title",
            "forecast_stopped",
            "gift_saved",
            "ble_no_radio",
            "ble_off",
            "ble_permission",
            "ble_name_fail",
            "ble_timeout",
            "ble_failed",
            "food_sentence",
            "chest_sentence",
            "not_a_match",
            "network_error",
            "null_control",
            "load_pack_first",
        ];
        for k in keys {
            assert!(!text(Lang::En, k).is_empty(), "{k}");
            assert!(!text(Lang::Zh, k).is_empty(), "{k}");
        }
        assert_eq!(text(Lang::En, "call_title"), "your Tamagotchi is calling");
        assert_eq!(text(Lang::Zh, "call_title"), "拓麻歌子在叫你");
        assert_eq!(text(Lang::En, "sanrio_first"), "Switch to Sanrio first");
        assert_eq!(text(Lang::En, "balance_range"), "balance must be 0–99999");
        assert_eq!(
            text(Lang::En, "ble_waiting"),
            "等待 WebMeets 连接 · TMGC_meets"
        );
        assert!(text(Lang::En, "notes_body").contains("not yet adult"));
        assert!(text(Lang::En, "notes_body").contains("age-up done, evolution not finished"));
        assert_eq!(text(Lang::En, "missing"), "");
        assert_eq!(text(Lang::Zh, "version"), "版本");
        assert_eq!(text(Lang::Zh, "ok"), "好");
        assert_eq!(
            localize(Lang::Zh, "save failed: disk full"),
            "保存失败：disk full"
        );
        assert_eq!(localize(Lang::En, "anything"), "anything");
        assert_eq!(
            localize(Lang::Zh, "no matching sentence"),
            "no matching sentence"
        );
    }
}
