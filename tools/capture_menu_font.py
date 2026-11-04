#!/usr/bin/env python3
"""Capture labelled glyphs by asking the emulated firmware to draw them.

The supplied snapshot must be a compatible Sweets game at the room screen.
Only a temporary copy is modified. Firmware and snapshots are never embedded
in the output. The status label's five-code sequence is replaced in the
snapshot's flash RAM; the resulting single character is captured from the
first line of the status/settings menu.
"""
import argparse
import json
import re
import struct
import subprocess
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NEEDLE = bytes.fromhex('043e 0437 0403 0410 0402 0000')


def png_rgb(path):
    data = path.read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('invalid PNG')
    offset, chunks, width, height = 8, [], None, None
    while offset < len(data):
        size = int.from_bytes(data[offset:offset+4], 'big')
        kind = data[offset+4:offset+8]
        body = data[offset+8:offset+8+size]
        offset += size + 12
        if kind == b'IHDR':
            width, height, depth, color, comp, filt, interlace = struct.unpack('>IIBBBBB', body)
            if (depth, color, comp, filt, interlace) != (8, 2, 0, 0, 0):
                raise ValueError('unsupported screenshot PNG format')
        if kind == b'IDAT': chunks.append(body)
        if kind == b'IEND': break
    raw = zlib.decompress(b''.join(chunks))
    row_size = width * 3
    if len(raw) != (row_size + 1) * height:
        raise ValueError('bad PNG size')
    rows = []
    for y in range(height):
        row = raw[y*(row_size+1):(y+1)*(row_size+1)]
        if row[0] != 0: raise ValueError('unsupported PNG filter')
        rows.append(row[1:])
    return width, height, rows


def glyph(path):
    width, height, rows = png_rgb(path)
    if (width, height) != (384, 384): raise ValueError('unexpected LCD size')
    # tg18cli writes a 3x nearest-neighbor enlargement of the 128x128 LCD.
    points = []
    for y in range(18, 38):
        for x in range(40, 88):
            r, g, b = rows[y*3][x*9:x*9+3]
            if r < 100 and g < 100 and b > 60:
                points.append((x, y))
    if not points: return None
    x0, x1 = min(x for x, _ in points), max(x for x, _ in points)
    y0, y1 = min(y for _, y in points), max(y for _, y in points)
    if x1-x0 >= 16 or y1-y0 >= 16:
        raise ValueError(f'unexpected shape bounds {(x0,y0,x1,y1)}')
    ps = set(points)
    return {'rows': [''.join('#' if (x,y) in ps else '.' for x in range(x0,x1+1))
                     for y in range(y0,y1+1)], 'x': x0, 'y': y0}


def patch_snapshot(path):
    snapshot = path.read_bytes()
    if snapshot[:8] != b'T18SNAP1': raise ValueError('bad snapshot magic')
    data = bytearray(zlib.decompress(snapshot[8:]))
    i = 0
    while i < len(data):
        n = data[i]; i += 1
        name = bytes(data[i:i+n]).decode(); i += n
        size = struct.unpack_from('<I', data, i)[0]; i += 4
        if name == 'ram':
            # Status label belongs to the flash image, not any copied code/data.
            pos = data.find(NEEDLE, i, i + min(size, 8*1024*1024))
            if pos == -1: raise ValueError('status label not found in flash')
            if data.find(NEEDLE, pos + 1, i + min(size, 8*1024*1024)) != -1:
                raise ValueError('status label is not unique')
            return snapshot[:8], data, pos
        i += size
    raise ValueError('snapshot has no RAM chunk')


def codes():
    source = (ROOT / 'rust/shell/src/textbuf.rs').read_text()
    yield 0x045B, '・'
    for number, char in re.findall(r'0x([0-9A-F]{4}) => "([^"\\]+)"', source):
        if len(char) == 1 and char != ' ':
            yield int(number, 16), char


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rom', required=True, type=Path)
    parser.add_argument('--snapshot', required=True, type=Path)
    parser.add_argument('--emulator', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--only', type=lambda x: int(x, 0), action='append')
    parser.add_argument('--probe', nargs=2, type=lambda x: int(x, 0), metavar=('START', 'END'), help='capture all codes in inclusive range without labels')
    args = parser.parse_args()
    magic, data, pos = patch_snapshot(args.snapshot)
    captured = []
    with tempfile.TemporaryDirectory(prefix='font-capture-') as tmp:
        snap, image = Path(tmp)/'input.t18s', Path(tmp)/'screen.png'
        selected = [(i, '?') for i in range(args.probe[0], args.probe[1] + 1)] if args.probe else list(codes())
        for code, char in selected:
            if args.only and code not in args.only: continue
            data[pos:pos+12] = code.to_bytes(2, 'big') + bytes(10)
            snap.write_bytes(magic + zlib.compress(data, 1))
            result = subprocess.run([str(args.emulator), str(args.rom), '--snapshot-in', str(snap),
                                     '--seconds', '3', '--press', '0.3:A', '--screenshot', str(image), '--quiet'],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            if result.returncode or 'result: Limit' not in result.stdout:
                raise RuntimeError(f'capture {code:04x} failed: {result.stderr}\n{result.stdout}')
            sample = glyph(image)
            if sample is None:
                if args.probe:
                    print(f'{code:04x} blank', flush=True)
                    continue
                raise RuntimeError(f'no glyph rendered for {code:04x} {char}')
            captured.append({'code': f'{code:04x}', 'char': char, **sample})
            print(f"{code:04x} {char} {sample['x']},{sample['y']} {len(sample['rows'][0])}x{len(sample['rows'])}", flush=True)
    args.output.write_text(json.dumps(captured, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__': main()
