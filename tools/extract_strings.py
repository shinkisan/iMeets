#!/usr/bin/env python3
"""Extract the fixed Japanese strings from a Sweets ROM.

The firmware keeps text as 16-bit big-endian kana codes and refers to it with
32-bit pointers into the flash window (0x20000000..0x20800000). This walks
every such pointer, decodes the string it targets, and writes the unique
results to JSON so the translation table can cover the whole script instead of
one screen at a time.

Usage: python3 tools/extract_strings.py ROM.bin [-o OUT.json]
"""

import argparse
import json
import re
import struct
from pathlib import Path

FLASH_BASE = 0x2000_0000
FLASH_END = 0x2080_0000
MAX_CHARS = 120
TEXTBUF = Path(__file__).resolve().parent.parent / "rust/shell/src/textbuf.rs"


def load_codes():
    """Read the code->character table out of the Rust decoder."""
    src = TEXTBUF.read_text()
    codes = {}
    for match in re.finditer(r'0x([0-9A-Fa-f]{4}) => "(.+?)"', src):
        codes[int(match.group(1), 16)] = match.group(2)
    if not codes:
        raise SystemExit(f"no character table found in {TEXTBUF}")
    return codes


def decode(rom, offset, codes):
    out = []
    while offset + 1 < len(rom) and len(out) < MAX_CHARS:
        code = (rom[offset] << 8) | rom[offset + 1]
        if code not in codes:
            break
        out.append(codes[code])
        offset += 2
    return "".join(out)


def useful(text):
    if len(text) < 2 or len(set(text)) <= 1:
        return False
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("rom")
    ap.add_argument("-o", "--out", default="tools/fixtures/sweets-strings.json")
    args = ap.parse_args()

    rom = Path(args.rom).read_bytes()
    codes = load_codes()

    targets = {}
    for pos in range(0, len(rom) - 4, 4):
        value = struct.unpack_from("<I", rom, pos)[0]
        if FLASH_BASE <= value < FLASH_END:
            target = value - FLASH_BASE
            targets.setdefault(target, 0)
            targets[target] += 1

    strings = {}
    for target, uses in targets.items():
        text = decode(rom, target, codes)
        if not useful(text):
            continue
        entry = strings.setdefault(text, {"text": text, "pointers": 0, "offsets": []})
        entry["pointers"] += uses
        if len(entry["offsets"]) < 8:
            entry["offsets"].append(target)

    items = sorted(strings.values(), key=lambda e: (-e["pointers"], e["text"]))
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(items, ensure_ascii=False, indent=1) + "\n")
    print(f"{len(targets)} text pointers -> {len(items)} unique strings -> {out}")


if __name__ == "__main__":
    main()
