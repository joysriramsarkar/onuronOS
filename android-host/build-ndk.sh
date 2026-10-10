#!/usr/bin/env bash
# android-host/build-ndk.sh — Cross-compile android-host Rust crate for Android aarch64 (arm64-v8a)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
JNILIBS_DIR="$SCRIPT_DIR/app/src/main/jniLibs/arm64-v8a"
TARGET_SO="$JNILIBS_DIR/libandroid_host.so"
TARGET="aarch64-linux-android"

if [ "${1:-}" = "--clean" ]; then
    echo "==> Cleaning staged Android native libraries..."
    rm -f "$TARGET_SO" "$JNILIBS_DIR/build_manifest.json"
    echo "[OK] Cleaned $JNILIBS_DIR"
    exit 0
fi

# Ensure clean staging directory (prevent stale .so on failure)
mkdir -p "$JNILIBS_DIR"
rm -f "$TARGET_SO"

echo "========================================================="
echo "       Building Android Host Native Library ($TARGET)    "
echo "========================================================="

# Check toolchain prerequisites
if ! rustup target list 2>/dev/null | grep -q "$TARGET (installed)"; then
    echo "[INFO] Target $TARGET not installed. Attempting 'rustup target add $TARGET'..."
    rustup target add "$TARGET" || {
        echo "[ERROR] Failed to install rust target $TARGET. Please run: rustup target add $TARGET" >&2
        exit 1
    }
fi

if command -v cargo-ndk >/dev/null 2>&1; then
    echo "[OK] Using cargo-ndk"
    cargo ndk -t arm64-v8a -o "$SCRIPT_DIR/app/src/main/jniLibs" build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
else
    echo "[INFO] cargo-ndk not found, attempting direct target build with target $TARGET"
    if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${NDK_HOME:-}" ]; then
        echo "[WARN] ANDROID_NDK_HOME is not set. cargo build --target $TARGET may fail if linker is missing."
    fi
    cargo build --target "$TARGET" --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
    SRC_SO="$SCRIPT_DIR/../target/$TARGET/release/libandroid_host.so"
    if [ ! -f "$SRC_SO" ]; then
        echo "[ERROR] Compiled library missing at $SRC_SO" >&2
        exit 1
    fi
    cp "$SRC_SO" "$TARGET_SO"
fi

if [ ! -f "$TARGET_SO" ]; then
    echo "[ERROR] libandroid_host.so was not generated in $JNILIBS_DIR" >&2
    exit 1
fi

# ELF Machine architecture check (Byte 18-19 must be 0xB7 for AArch64)
if command -v readelf >/dev/null 2>&1; then
    MACHINE=$(readelf -h "$TARGET_SO" | grep "Machine:" | awk '{print $2}' || true)
    if [ "$MACHINE" != "AArch64" ]; then
        echo "[ERROR] Built library is not AArch64! Detected: $MACHINE" >&2
        exit 1
    fi
    echo "[OK] ELF Machine verified: AArch64"
fi

# Generate manifest
SO_SHA=$(sha256sum "$TARGET_SO" | awk '{print $1}')
SO_SIZE=$(wc -c < "$TARGET_SO")
GIT_REV=$(git rev-parse HEAD 2>/dev/null || echo "unknown")

cat << EOF > "$JNILIBS_DIR/build_manifest.json"
{
  "target": "$TARGET",
  "abi": "arm64-v8a",
  "library": "libandroid_host.so",
  "sha256": "$SO_SHA",
  "size_bytes": $SO_SIZE,
  "git_revision": "$GIT_REV",
  "verified": true
}
EOF

echo "[OK] Staged libandroid_host.so in $JNILIBS_DIR (sha256: ${SO_SHA:0:16}...)"
echo "[OK] Native build manifest generated: $JNILIBS_DIR/build_manifest.json"
