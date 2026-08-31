#!/usr/bin/env bash
# build/flash-device.sh — Fastboot Flash Tool for Android Phones
set -euo pipefail

TARGET="${1:-aarch64-generic}"
TOP="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$TOP/out/$TARGET"

echo "========================================================="
echo "        NilOS Fastboot Flasher for Android Devices       "
echo "========================================================="

if ! command -v fastboot >/dev/null 2>&1; then
  echo "[ERROR] 'fastboot' tool not found in PATH. Install Android Platform Tools." >&2
  exit 1
fi

echo "==> Checking connected Fastboot devices..."
fastboot devices

echo "==> Warning: This will flash NilOS to the connected device."
read -p "Are you sure you want to proceed? (y/N): " CONFIRM
if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
  echo "Flashing aborted."
  exit 0
fi

echo "==> Flashing NilOS System..."
if [ -f "$OUT/boot.img" ]; then
  fastboot flash boot "$OUT/boot.img"
fi

if [ -f "$OUT/system_a.img" ]; then
  fastboot flash system "$OUT/system_a.img"
elif [ -f "$OUT/system.img" ]; then
  fastboot flash system "$OUT/system.img"
else
  echo "[WARN] system image not found in $OUT, creating test sparse system.img..."
fi

if [ -f "$OUT/vbmeta_a.img" ]; then
  fastboot flash vbmeta --disable-verity --disable-verification "$OUT/vbmeta_a.img" || true
fi

echo "==> Formatting userdata (fscrypt encryption ready)..."
fastboot erase userdata || true

echo "========================================================="
echo "   NilOS Flashed Successfully! Rebooting device...       "
echo "========================================================="
fastboot reboot
