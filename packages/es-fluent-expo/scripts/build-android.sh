#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

bash "$SCRIPT_DIR/generate-bindings.sh"
cargo ndk \
  --target arm64-v8a \
  --target armeabi-v7a \
  --target x86_64 \
  -o "$PACKAGE_DIR/android/src/main/jniLibs" \
  build \
  --manifest-path "$PACKAGE_DIR/rust/Cargo.toml" \
  --release
