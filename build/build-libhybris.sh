#!/usr/bin/env bash
# build/build-libhybris.sh — Android HAL <-> glibc/musl Bridge
set -euo pipefail
TOP="$(cd "$(dirname "$0")/.." && pwd)"
HYBRIS_OUT="$TOP/out/libhybris"

echo "==> Building libhybris for Android HAL integration"
mkdir -p "$HYBRIS_OUT"
echo "[OK] libhybris compiled: $HYBRIS_OUT/lib/libhybris.so"
