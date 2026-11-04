// Run with: swiftc ios/PocketMeets/PocketMeets/Translations.swift \
//   tools/tests/TranslationsChecks.swift -o /tmp/translations-check && /tmp/translations-check
import Foundation

@main
enum TranslationsChecks {
    static func main() {
        precondition(LocalizedText.lookup("じょうたい せってい", strict: true) == "状态 / 设置")
        precondition(LocalizedText.lookup("せだい 27代目 8才 0G", strict: true) == "世代 / 第27代 / 8岁 / 0 G")
        precondition(LocalizedText.lookup("ちょっとまってね", strict: true) == "请稍等一下")
        precondition(LocalizedText.lookup("おめでとうございます！", strict: true) == "恭喜！")
        precondition(LocalizedText.lookup("いらっしゃいませ", strict: true) == "欢迎光临")
        precondition(LocalizedText.lookup("プロポーズする？", strict: true) == "要求婚吗？")
        precondition(LocalizedText.lookup("かの ふわつのっち", strict: true) == "那只 / 软软吉")
        // The matcher may re-space a line; the compact variant still hits.
        precondition(LocalizedText.lookup("おめでとう ございます！", strict: true) == "恭喜！")
        precondition(LocalizedText.lookup("あなたの たんじょうび", strict: true) == "你的生日")
        precondition(LocalizedText.lookup("あなたのなまえは？", strict: true) == "你叫什么名字？")
        precondition(LocalizedText.lookup("あなたのなまえは？ ああ", strict: true) == "你叫什么名字？")
        // The complete glyph result is a name, not the familiar suffix まめっち.
        precondition(LocalizedText.lookup("かまめっち", strict: true) == nil)
        precondition(LocalizedText.lookup("か ふわつのっち", strict: true) == nil)
    }
}
