#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$(cd -- "$PACKAGE_DIR/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$WORKSPACE_DIR/target}"

case "$(uname -s)" in
  Darwin) LIBRARY_PATH="$TARGET_DIR/debug/libes_fluent_expo_native.dylib" ;;
  Linux) LIBRARY_PATH="$TARGET_DIR/debug/libes_fluent_expo_native.so" ;;
  *) echo "UniFFI binding generation requires macOS or Linux" >&2; exit 1 ;;
esac

cargo build --manifest-path "$PACKAGE_DIR/rust/Cargo.toml"
cargo run \
  --manifest-path "$PACKAGE_DIR/rust/Cargo.toml" \
  --features bindgen \
  --bin uniffi-bindgen \
  -- generate "$LIBRARY_PATH" \
  --language swift \
  --out-dir "$PACKAGE_DIR/ios/generated" \
  --no-format
cargo run \
  --manifest-path "$PACKAGE_DIR/rust/Cargo.toml" \
  --features bindgen \
  --bin uniffi-bindgen \
  -- generate "$LIBRARY_PATH" \
  --language kotlin \
  --out-dir "$PACKAGE_DIR/android/src/main/java" \
  --no-format

GENERATED_FILES=(
  "$PACKAGE_DIR/ios/generated/EsFluentExpoNative.swift"
  "$PACKAGE_DIR/ios/generated/EsFluentExpoNativeFFI.h"
  "$PACKAGE_DIR/ios/generated/EsFluentExpoNativeFFI.modulemap"
  "$PACKAGE_DIR/android/src/main/java/expo/modules/esfluent/uniffi/es_fluent_expo_native.kt"
)
if [[ "$(uname -s)" == "Darwin" ]]; then
  sed -i '' -e 's/[[:blank:]]*$//' "${GENERATED_FILES[@]}"
else
  sed -i -e 's/[[:blank:]]*$//' "${GENERATED_FILES[@]}"
fi
