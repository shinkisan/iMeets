#!/usr/bin/env python3
"""Merge the curated Swift table with the ROM string translations.

The hand-written entries in Translations.swift win; the translations collected
in tools/fixtures/zh-raw/*.tsv fill the rest. Keys are whitespace-normalised so
a line the matcher re-spaces still hits.
"""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SWIFT = ROOT / "ios/PocketMeets/PocketMeets/Translations.swift"
RAW = ROOT / "tools/fixtures/zh-raw"
OUT_JSON = ROOT / "tools/fixtures/sweets-zh.json"


def norm(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def main() -> None:
    src = SWIFT.read_text()
    marker = "\n    ]\n"
    head, sep, tail = src.partition(marker)
    if not sep:
        raise SystemExit("table end not found")

    curated = {}
    for match in re.finditer(r'"(.+?)"\s*:\s*"(.+?)"', head):
        key, value = match.group(1), match.group(2)
        if value == "（片段）" or not value:
            continue
        curated[norm(key)] = value

    added = {}
    for path in sorted(RAW.glob("*.tsv")):
        for line in path.read_text().split("\n"):
            if "\t" not in line:
                continue
            key, value = line.split("\t", 1)
            key, value = norm(key), value.strip()
            if not key or not value:
                continue
            added.setdefault(key, value)

    merged = dict(added)
    # Space-insensitive aliases: the matcher can join or split runs.
    for key, value in list(merged.items()):
        compact = key.replace(" ", "")
        if compact != key:
            merged.setdefault(compact, value)
    merged.update(curated)  # curated wins
    for key, value in list(curated.items()):
        compact = key.replace(" ", "")
        if compact != key:
            merged.setdefault(compact, value)

    OUT_JSON.write_text(json.dumps(merged, ensure_ascii=False, indent=1, sort_keys=True) + "\n")

    lines = []
    lines.append("import Foundation\n")
    lines.append("\n")
    lines.append("/// Japanese to Chinese table. Curated entries plus the fixed strings\n")
    lines.append("/// extracted from the ROM text tables (tools/extract_strings.py).\n")
    lines.append("enum LocalizedText {\n")
    lines.append("    static let table: [String: String] = [\n")
    for key in sorted(merged):
        value = merged[key].replace("\\", "\\\\").replace('"', '\\"')
        k = key.replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'        "{k}": "{value}",\n')
    lines.append("    ]\n")
    SWIFT.write_text("".join(lines) + tail)
    print(f"curated {len(curated)}, extracted {len(added)}, total {len(merged)}")


if __name__ == "__main__":
    main()
