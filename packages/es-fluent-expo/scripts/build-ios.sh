#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The iOS native library must be built on macOS with Xcode installed" >&2
  exit 1
fi

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$(cd -- "$PACKAGE_DIR/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$WORKSPACE_DIR/target}"
BUILD_DIR="$PACKAGE_DIR/.native-build/ios"
FRAMEWORK_PATH="$PACKAGE_DIR/ios/EsFluentExpoNative.xcframework"

bash "$SCRIPT_DIR/generate-bindings.sh"
cargo build --manifest-path "$PACKAGE_DIR/rust/Cargo.toml" --release --target aarch64-apple-ios
cargo build --manifest-path "$PACKAGE_DIR/rust/Cargo.toml" --release --target aarch64-apple-ios-sim
cargo build --manifest-path "$PACKAGE_DIR/rust/Cargo.toml" --release --target x86_64-apple-ios

rm -rf -- "$BUILD_DIR" "$FRAMEWORK_PATH"
mkdir -p "$BUILD_DIR/simulator" "$BUILD_DIR/headers"
lipo -create \
  "$TARGET_DIR/aarch64-apple-ios-sim/release/libes_fluent_expo_native.a" \
  "$TARGET_DIR/x86_64-apple-ios/release/libes_fluent_expo_native.a" \
  -output "$BUILD_DIR/simulator/libes_fluent_expo_native.a"
cp "$PACKAGE_DIR/ios/generated/EsFluentExpoNativeFFI.h" "$BUILD_DIR/headers/"
cp "$PACKAGE_DIR/ios/generated/EsFluentExpoNativeFFI.modulemap" "$BUILD_DIR/headers/module.modulemap"
xcodebuild -create-xcframework \
  -library "$TARGET_DIR/aarch64-apple-ios/release/libes_fluent_expo_native.a" \
  -headers "$BUILD_DIR/headers" \
  -library "$BUILD_DIR/simulator/libes_fluent_expo_native.a" \
  -headers "$BUILD_DIR/headers" \
  -output "$FRAMEWORK_PATH"
