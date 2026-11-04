#!/usr/bin/env python3
"""Rank 8x8 1-bpp ROM windows against the checked-in menu glyph samples.

A hit is only a candidate: random ROM data also matches small bitmaps. The
program deliberately does not write a character map or modify the ROM.
"""
import argparse
import heapq
import re
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1] / 'ios/PocketMeets/PocketMeets/MenuGlyphs.swift'


def samples():
    pattern = r'MenuGlyph\(char: "([^"]+)", width: \d+, rows: \[([^]]+)\]\)'
    for char, raw in re.findall(pattern, SOURCE.read_text()):
        rows = re.findall(r'"([.#]+)"', raw)
        yield char, [[c == '#' for c in row] for row in rows]


def crop(image):
    points = [(x, y) for y, row in enumerate(image) for x, v in enumerate(row) if v]
    if not points:
        return []
    x0, x1 = min(x for x, y in points), max(x for x, y in points)
    y0, y1 = min(y for x, y in points), max(y for x, y in points)
    return [row[x0:x1 + 1] for row in image[y0:y1 + 1]]


def normalized(image, side=12):
    image = crop(image)
    if not image:
        return None
    h, w = len(image), len(image[0])
    return tuple(image[min(y*h//side, h-1)][min(x*w//side, w-1)]
                 for y in range(side) for x in range(side))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('rom', type=Path)
    parser.add_argument('--start', type=lambda x: int(x, 0), default=0)
    parser.add_argument('--end', type=lambda x: int(x, 0))
    parser.add_argument('--limit', type=int, default=5)
    parser.add_argument('--stride', type=int, default=1)
    args = parser.parse_args()
    data = args.rom.read_bytes()
    end = min(len(data), args.end if args.end is not None else len(data))
    if args.start < 0 or args.stride < 1 or args.limit < 1 or end <= args.start:
        parser.error('invalid range, stride or limit')
    targets = [(char, normalized(rows)) for char, rows in samples()]
    best = [[] for _ in targets]
    for offset in range(args.start, end - 7, args.stride):
        block = data[offset:offset + 8]
        image = [[bool(byte & (128 >> x)) for x in range(8)] for byte in block]
        glyph = normalized(image)
        if glyph is None:
            continue
        for i, (_, target) in enumerate(targets):
            distance = sum(a != b for a, b in zip(glyph, target))
            entry = (-distance, -offset)
            if len(best[i]) < args.limit:
                heapq.heappush(best[i], entry)
            elif entry > best[i][0]:
                heapq.heapreplace(best[i], entry)
    for (char, _), matches in zip(targets, best):
        print(char, ' '.join(f'{-o:#x}:{-d}/144' for d, o in sorted(matches, reverse=True)))


if __name__ == '__main__':
    main()
