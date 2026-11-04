import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
STRINGS = ROOT / "tools/fixtures/sweets-strings.json"
ZH = ROOT / "tools/fixtures/sweets-zh.json"
SWIFT = ROOT / "ios/PocketMeets/PocketMeets/Translations.swift"


def norm(text):
    return re.sub(r"\s+", " ", text).strip()


def is_debug(text):
    return bool(
        re.fullmatch(r"さくじょ\d+", text)
        or re.fullmatch(r"にわダミー\d+", text)
        or re.fullmatch(r"[A-Z]{1,5}→.*", text)
        or text.startswith("D-")
        or re.fullmatch(r"[ -~]+", text)
    )


@unittest.skipUnless(
    STRINGS.exists() and ZH.exists(),
    "ROM fixture dumps are not published",
)
class TranslationCoverage(unittest.TestCase):
    def setUp(self):
        self.items = json.loads(STRINGS.read_text())
        self.zh = json.loads(ZH.read_text())
        self.swift = SWIFT.read_text()

    def test_every_real_string_is_translated(self):
        missing = []
        for item in self.items:
            text = item["text"]
            if is_debug(text):
                continue
            key = norm(text)
            if key not in self.zh and key.replace(" ", "") not in self.zh:
                missing.append(text)
        self.assertEqual(missing, [], f"{len(missing)} strings without a translation")

    def test_swift_table_matches_json(self):
        head = self.swift.split("\n    ]\n", 1)[0]
        entries = dict(re.findall(r'"(.+?)"\s*:\s*"(.+?)"', head))
        self.assertEqual(len(entries), len(self.zh))
        for key, value in self.zh.items():
            self.assertEqual(entries.get(key), value, key)


if __name__ == "__main__":
    unittest.main()
