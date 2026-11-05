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
case "$(uname -m)" in
  arm64) TARGET=aarch64-apple-ios-sim ;;
  *)     TARGET=x86_64-apple-ios ;;
esac

echo "== rust $TARGET =="
rustup target list --installed | grep -qx "$TARGET" || rustup target add "$TARGET"
cargo build --release --target "$TARGET" \
  --manifest-path rust/shell/Cargo.toml --offline

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
xcrun simctl install "$DEVICE" "$APP"
xcrun simctl launch "$DEVICE" "$BUNDLE"
echo "done"
