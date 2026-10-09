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
if [ ! -f "$OUT/boot.img" ] && [ -f "$OUT/kernel" ] && [ -f "$OUT/initramfs.cpio.gz" ]; then
  echo "==> Packaging Android boot.img using build/mkbootimg.py..."
  python3 "$TOP/build/mkbootimg.py" create --kernel "$OUT/kernel" --ramdisk "$OUT/initramfs.cpio.gz" -o "$OUT/boot.img"
fi
if [ ! -f "$OUT/boot.img" ]; then
  echo "[ERROR] Boot image not found at $OUT/boot.img" >&2
  exit 1
fi
fastboot flash boot "$OUT/boot.img"

SYS_IMG=""
if [ -f "$OUT/system_a.img" ]; then
  SYS_IMG="$OUT/system_a.img"
elif [ -f "$OUT/system.img" ]; then
  SYS_IMG="$OUT/system.img"
else
  echo "[ERROR] Valid system image not found in $OUT" >&2
  exit 1
fi
fastboot flash system "$SYS_IMG"

if [ -f "$OUT/vbmeta_a.img" ]; then
  echo "==> Verifying and flashing cryptographic vbmeta image..."
  python3 "$TOP/build/mkvbmeta.py" verify --image "$SYS_IMG" --vbmeta "$OUT/vbmeta_a.img"
  fastboot flash vbmeta "$OUT/vbmeta_a.img"
else
  echo "[ERROR] Cryptographic vbmeta_a.img required for Verified Boot was not found in $OUT" >&2
  exit 1
fi

echo "==> Formatting userdata (fscrypt encryption ready)..."
fastboot format userdata || fastboot erase userdata

echo "========================================================="
echo "   NilOS Flashed Successfully! Rebooting device...       "
echo "========================================================="
fastboot reboot
