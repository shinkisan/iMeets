#!/usr/bin/env python3
"""Render every captured Sweets code in context and check the live decoder.

Requires a user-supplied ROM, the corresponding room and paused status-name
snapshots, and a built inspect_sweets_lcd example. Inputs are never modified.
"""
import argparse
import json
import subprocess
import tempfile
import zlib
from pathlib import Path

from capture_menu_font import patch_snapshot
from capture_small_font import locate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rom', required=True, type=Path)
    parser.add_argument('--room-snapshot', required=True, type=Path)
    parser.add_argument('--name-snapshot', required=True, type=Path)
    parser.add_argument('--decoder', required=True, type=Path)
    args = parser.parse_args()
    fixtures = Path(__file__).resolve().parent / 'fixtures'
    modes = [
        ('menu', patch_snapshot(args.room_snapshot), fixtures / 'sweets-menu-glyphs.json'),
        ('name', locate(args.name_snapshot), fixtures / 'sweets-small-glyphs.json'),
    ]
    failures = []
    with tempfile.TemporaryDirectory(prefix='sweets-context-') as tmp:
        snapshot = Path(tmp) / 'input.t18s'
        for mode, (magic, data, offset), fixture in modes:
            entries = json.loads(fixture.read_text())
            # Non-code entries are variants of a separately tested code.
            entries = [entry for entry in entries if len(entry['code']) == 4]
            for index, entry in enumerate(entries, 1):
                code = int(entry['code'], 16).to_bytes(2, 'big')
                char = entry['char']
                if mode == 'menu':
                    data[offset:offset + 12] = bytes.fromhex('0401 0402') + code + bytes.fromhex('0403 0404 0000')
                    expected = f'あい{char}うえ せってい'
                    extra = ['3', '0.3:A']
                else:
                    data[offset:offset + 6] = bytes.fromhex('0406') + code + bytes(2)
                    expected = f'か{char} ふわつのっち'
                    extra = ['.5']
                snapshot.write_bytes(magic + zlib.compress(data, 1))
                result = subprocess.run([str(args.decoder), str(args.rom), str(snapshot), *extra],
                                        capture_output=True, text=True, timeout=10)
                actual = result.stdout.strip()
                if result.returncode != 0 or actual != expected:
                    failures.append((mode, entry['code'], char, expected, actual, result.stderr))
                    print(f'{mode} {entry["code"]} {char}: expected {expected!r}, got {actual!r}', flush=True)
                if index % 25 == 0 or index == len(entries):
                    print(f'{mode}: {index}/{len(entries)}', flush=True)
    print(f'{len(failures)} mismatches')
    return int(bool(failures))


if __name__ == '__main__':
    raise SystemExit(main())
