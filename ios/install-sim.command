#!/bin/zsh
# Build the Rust simulator library, build the app, install it on the iOS
# Simulator and launch it. Run this on a Mac with Xcode.
#
#   DEVICE="iPhone 16" ./ios/install-sim.command
set -euo pipefail
cd "$(dirname "$0")/.."

DEVICE=${DEVICE:-iPhone 16}
BUNDLE=app.pocketmeets.ios
DD=${DD:-$HOME/derived-tama-sim}
echo "== rust simulator library (Apple Silicon) =="
rustup target add aarch64-apple-ios-sim
cargo build --release --target aarch64-apple-ios-sim \
  --manifest-path rust/shell/Cargo.toml

echo "== xcode =="
xcodebuild -project ios/PocketMeets/PocketMeets.xcodeproj \
  -scheme PocketMeets -configuration Debug -sdk iphonesimulator \
  -destination "platform=iOS Simulator,name=$DEVICE" \
  -derivedDataPath "$DD" \
  CODE_SIGNING_ALLOWED=NO build

APP="$DD/Build/Products/Debug-iphonesimulator/PocketMeets.app"
echo "== simulator =="
xcrun simctl boot "$DEVICE" 2>/dev/null || true
open -a Simulator
xcrun simctl bootstatus "$DEVICE" -b
# install can silently keep an old bundle while the app is still running
xcrun simctl terminate "$DEVICE" "$BUNDLE" 2>/dev/null || true
xcrun simctl install "$DEVICE" "$APP"
xcrun simctl launch "$DEVICE" "$BUNDLE"
echo "done"
