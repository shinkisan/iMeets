import json
import os
import struct
import subprocess
import sys
import zlib

W = H = 128
COLS = ROWS = 5
CELL = 128
SHEET_W = COLS * CELL
SHEET_H = ROWS * CELL
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def read_rgb(path, w, h):
    data = open(path, 'rb').read()
    if len(data) < w * h * 3:
        return None
    return data[: w * h * 3]


def scale_nearest(raw, sw, sh, dw, dh):
    out = bytearray(dw * dh * 3)
    for y in range(dh):
        sy = min(sh - 1, y * sh // dh)
        for x in range(dw):
            sx = min(sw - 1, x * sw // dw)
            si = (sy * sw + sx) * 3
            di = (y * dw + x) * 3
            out[di:di + 3] = raw[si:si + 3]
    return bytes(out)


def blit(sheet, raw, x0, y0, w, h, border=None):
    for y in range(h):
        row = (y0 + y) * SHEET_W
        src = y * w * 3
        sheet[(row + x0) * 3:(row + x0 + w) * 3] = raw[src:src + w * 3]
    if border:
        for x in range(x0, x0 + w):
            for yy in (y0, y0 + h - 1):
                i = (yy * SHEET_W + x) * 3
                sheet[i:i + 3] = bytes(border)
        for y in range(y0, y0 + h):
            for xx in (x0, x0 + w - 1):
                i = (y * SHEET_W + xx) * 3
                sheet[i:i + 3] = bytes(border)


def write_png(path, raw, w, h):
    rows = b''.join(b'\x00' + raw[y * w * 3:(y + 1) * w * 3] for y in range(h))
    def chunk(t, d):
        c = t + d
        return struct.pack('>I', len(d)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b'')
    open(path, 'wb').write(png)


def pick(paths, n):
    if len(paths) <= n:
        return paths
    return [paths[i * (len(paths) - 1) // (n - 1)] for i in range(n)]


def main():
    frames_dir, qr_png, out = sys.argv[1], sys.argv[2], sys.argv[3]
    payload_path = os.path.join(os.path.dirname(out), 'qr.txt')
    payload = open(payload_path).read().strip() if os.path.exists(payload_path) else ''
    qr_raw_path = os.path.join(os.path.dirname(out), 'qr.rgb')
    qr_dim = 0
    if payload:
        info = subprocess.check_output(['node', os.path.join(ROOT, 'tools/qr/qrraw.cjs'), payload, qr_raw_path])
        qr_dim = json.loads(info)['dim']
    sheet = bytearray(b'\xff' * (SHEET_W * SHEET_H * 3))
    if qr_dim:
        raw = open(qr_raw_path, 'rb').read()
        qr = scale_nearest(raw, qr_dim, qr_dim, CELL * 2, CELL * 2)
        blit(sheet, qr, 0, 0, CELL * 2, CELL * 2)
    host = pick(sorted(os.path.join(frames_dir, f) for f in os.listdir(frames_dir) if f.startswith('host-')), 10)
    join = pick(sorted(os.path.join(frames_dir, f) for f in os.listdir(frames_dir) if f.startswith('join-')), 11)
    cells = [(x, y) for y in range(ROWS) for x in range(COLS) if not (x < 2 and y < 2)]
    for idx, path in enumerate(host + join):
        if idx >= len(cells):
            break
        x, y = cells[idx]
        raw = read_rgb(path, W, H)
        if raw is None:
            continue
        border = (40, 90, 200) if idx < len(host) else (220, 60, 120)
        blit(sheet, raw, x * CELL, y * CELL, CELL, CELL, border)
    write_png(out, bytes(sheet), SHEET_W, SHEET_H)
    print(out)


if __name__ == '__main__':
    main()
