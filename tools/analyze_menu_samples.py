#!/usr/bin/env python3
"""Measure how many isolated glyph templates occur exactly in menu screens.

This is a coverage diagnostic, not a decoder: adjacent glyphs can touch,
character matches can be coincidental, and their order is not checked.
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent / 'fixtures'


def matches(screen, glyph):
    height, width = len(glyph), len(glyph[0])
    for y in range(len(screen) - height + 1):
        for x in range(len(screen[0]) - width + 1):
            if all(screen[y + dy][x:x + width] == row for dy, row in enumerate(glyph)):
                yield x, y


def main():
    glyphs = json.loads((HERE / 'sweets-menu-glyphs.json').read_text())
    screens = json.loads((HERE / 'sweets-menu-screens.json').read_text())
    for screen in screens:
        chars = set(screen['label']) - {' '}
        missing = []
        for char in sorted(chars):
            candidates = (item['rows'] for item in glyphs if item['char'] == char)
            if not any(next(matches(screen['rows'], rows), None) is not None for rows in candidates):
                missing.append(char)
        print(screen['label'], 'missing:', ''.join(missing) or 'none')


if __name__ == '__main__': main()
