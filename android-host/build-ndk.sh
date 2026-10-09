#!/usr/bin/env bash
# android-host/build-ndk.sh — Cross-compile android-host Rust crate for Android aarch64 (arm64-v8a)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
JNILIBS_DIR="$SCRIPT_DIR/app/src/main/jniLibs/arm64-v8a"
TARGET="aarch64-linux-android"

mkdir -p "$JNILIBS_DIR"

echo "==> Building android-host crate for target: $TARGET"
if command -v cargo-ndk >/dev/null 2>&1; then
    echo "[OK] Using cargo-ndk"
    cargo ndk -t arm64-v8a -o "$SCRIPT_DIR/app/src/main/jniLibs" build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
else
    echo "[INFO] cargo-ndk not found, using standard cargo build with target $TARGET"
    cargo build --target "$TARGET" --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
    cp "$SCRIPT_DIR/../target/$TARGET/release/libandroid_host.so" "$JNILIBS_DIR/"
fi

echo "[OK] Staged libandroid_host.so in $JNILIBS_DIR"
