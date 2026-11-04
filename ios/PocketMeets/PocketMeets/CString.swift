import Foundation

enum CString {
    static func take(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
        guard let pointer else { return "" }
        let text = String(cString: pointer)
        pm_string_free(pointer)
        return text
    }

    static func object(_ text: String) -> [String: Any] {
        guard let data = text.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return [:]
        }
        return object
    }

    static func int64(_ value: Any?) -> Int64? {
        if let number = value as? NSNumber { return number.int64Value }
        if let number = value as? Int { return Int64(number) }
        return nil
    }

    static func ok(_ text: String) -> (Bool, String, Any?) {
        let object = object(text)
        let success = object["ok"] as? Bool ?? false
        let error = object["error"] as? String ?? ""
        return (success, error, object["result"])
    }
}

struct Prefs: Codable, Equatable {
    var language: String = "zh"
    var screenshots: Bool = false
    var dot_matrix: Bool = false
    var fast_keys: Bool = false
    var key_haptics: Bool = true
    var muted: Bool = false
    var follow_system_sleep: Bool = false
    var background_care: Bool = true
    var care_paused: Bool = false
    var care_auto_resume: Bool = false
    var care_fallback_notice: Bool = false
    var care_resumed_notice: Bool = false
    var care_intro_version: Int = 0
    var notifications_requested: Bool = false
    var last_call_notification: Int64 = 0
    var appearance: Int = 0
    var appearance_auto: Bool = false
    var small_screen_scale: Int = 100
    var small_screen_size_configured: Bool = false
    var small_screen_guide_version: Int = 0
    var small_screen_capture_guide_seen: Bool = false
    var shelf_enabled: Bool = false
    var shelf_pack: String = ""
    var shelf_language: String = "jp"
    var shelf_character: String = ""
    var ble_previous_name: String = ""

    var langCode: Int32 { language == "en" ? 1 : 0 }

    static func load(_ json: String) -> Prefs {
        guard let data = json.data(using: .utf8),
              let prefs = try? JSONDecoder().decode(Prefs.self, from: data) else {
            return Prefs()
        }
        return prefs
    }
}

struct ShelfItem: Identifiable, Equatable, Hashable {
    var id: String { path }
    var name: String
    var kind: String
    var path: String
    var giftId: Int = 0
    var price: Int = 0
}
