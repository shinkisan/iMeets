#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/tools/lan-demo/out"
ROM="${POCKET_MEETS_ROM:-/mnt/c/Users/gurri/Downloads/fw_tg18_jp_v062_2019-09-12_sweets.bin}"
A="${1:-/tmp/male-adult}"
B="${2:-/tmp/female-adult}"
BIN="$ROOT/target/x86_64-unknown-linux-musl/release/examples/irlanqr"

if [ ! -x "$BIN" ]; then
  cargo build --release --manifest-path "$ROOT/rust/shell/Cargo.toml" \
    --target x86_64-unknown-linux-musl \
    --config 'target.x86_64-unknown-linux-musl.linker="rust-lld"' \
    --offline --example irlanqr
fi
[ -d "$ROOT/tools/qr/node_modules" ] || (cd "$ROOT/tools/qr" && npm install --silent)
[ -d "$A" ] && [ -d "$B" ] || { echo "save not found"; exit 1; }

rm -f /tmp/tama-join.png
rm -rf "$OUT"; mkdir -p "$OUT/frames"
cp -R "$A" "$OUT/a"; cp -R "$B" "$OUT/b"

IRLAN_SHOT_DIR="$OUT/frames" POCKET_MEETS_ROM="$ROM" "$BIN" host "$OUT/a" 0 35 10 0 > "$OUT/host.log" 2>&1 &
HOST=$!
for _ in $(seq 1 80); do [ -f /tmp/tama-join.png ] && break; sleep 0.25; done
cp /tmp/tama-join.png "$OUT/qr.png"
PAYLOAD=$(node "$ROOT/tools/qr/qrscan.cjs" "$OUT/qr.png")
printf '%s\n' "$PAYLOAD" > "$OUT/qr.txt"
IRLAN_SHOT_DIR="$OUT/frames" POCKET_MEETS_ROM="$ROM" "$BIN" join "$PAYLOAD" "$OUT/b" 35 10 0 > "$OUT/join.log" 2>&1
wait "$HOST"
python3 "$ROOT/tools/lan-demo/sheet.py" "$OUT/frames" "$OUT/qr.png" "$OUT/report.png"
echo "REPORT $OUT/report.png"
