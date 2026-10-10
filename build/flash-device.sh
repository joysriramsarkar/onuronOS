#!/usr/bin/env bash
# build/flash-device.sh — Safe Fastboot Flash Tool for OnuronOS Targets
set -euo pipefail

TARGET="${1:-aarch64-generic}"
TOP="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$TOP/out/$TARGET"
WIPE_USERDATA=0
FORCE_UNSUPPORTED=0

shift || true
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe-userdata) WIPE_USERDATA=1; shift ;;
    --force-unsupported) FORCE_UNSUPPORTED=1; shift ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

echo "========================================================="
echo "       OnuronOS Safe Fastboot Flasher (Hardware Gate)    "
echo "========================================================="

if ! command -v fastboot >/dev/null 2>&1; then
  echo "[ERROR] 'fastboot' tool not found in PATH. Install Android Platform Tools." >&2
  exit 1
fi

echo "==> Checking connected Fastboot devices..."
DEVICES=$(fastboot devices)
if [ -z "$DEVICES" ]; then
  echo "[ERROR] No device connected in fastboot mode. Connect phone in fastboot/bootloader mode." >&2
  exit 1
fi
echo "$DEVICES"

# Read device product identifier
DEVICE_PRODUCT=$(fastboot getvar product 2>&1 | grep "product:" | awk '{print $2}' || echo "unknown")
echo "[INFO] Connected device product identifier: $DEVICE_PRODUCT"

# Device matching gate
if [ "$TARGET" = "oneplus-fajita" ] || [ "$TARGET" = "fajita" ]; then
  if [ "$DEVICE_PRODUCT" != "fajita" ] && [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
    echo "[ERROR] Device product '$DEVICE_PRODUCT' does not match target 'fajita'!" >&2
    echo "        Refusing to flash incompatible boot/system images to prevent hardware bricking." >&2
    echo "        Pass --force-unsupported if you are an expert and intentional." >&2
    exit 1
  fi
elif [ "$TARGET" = "aarch64-generic" ]; then
  if [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
    echo "[SECURITY GATE] Target 'aarch64-generic' is an unsupported virtual/generic image." >&2
    echo "                Flashing generic images to physical phone hardware risks soft-bricking." >&2
    echo "                Pass --force-unsupported if testing in a controlled hardware bring-up lab." >&2
    exit 1
  fi
fi

# Verify image existence
if [ ! -f "$OUT/boot.img" ] && [ -f "$OUT/kernel" ] && [ -f "$OUT/initramfs.cpio.gz" ]; then
  echo "==> Packaging Android boot.img using build/mkbootimg.py..."
  python3 "$TOP/build/mkbootimg.py" create --kernel "$OUT/kernel" --ramdisk "$OUT/initramfs.cpio.gz" -o "$OUT/boot.img"
fi

if [ ! -f "$OUT/boot.img" ]; then
  echo "[ERROR] Boot image not found at $OUT/boot.img" >&2
  exit 1
fi

SYS_IMG=""
if [ -f "$OUT/system_a.img" ]; then
  SYS_IMG="$OUT/system_a.img"
elif [ -f "$OUT/system.img" ]; then
  SYS_IMG="$OUT/system.img"
else
  echo "[ERROR] Valid system image not found in $OUT" >&2
  exit 1
fi

echo "==> Target images validated:"
echo "    Target:     $TARGET"
echo "    Boot image: $OUT/boot.img"
echo "    System:     $SYS_IMG"
if [ "$WIPE_USERDATA" -eq 1 ]; then
  echo "    Userdata:   WILL BE ERASED (--wipe-userdata active)"
else
  echo "    Userdata:   Preserved (Pass --wipe-userdata to reformat)"
fi

echo "==> Safety Confirmation:"
read -p "Are you sure you want to proceed with flashing? (y/N): " CONFIRM
if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
  echo "Flashing aborted."
  exit 0
fi

echo "==> Flashing boot partition..."
fastboot flash boot "$OUT/boot.img"

echo "==> Flashing system partition..."
fastboot flash system "$SYS_IMG"

if [ -f "$OUT/vbmeta_a.img" ]; then
  echo "==> Flashing verified boot vbmeta..."
  fastboot flash vbmeta "$OUT/vbmeta_a.img"
elif [ -f "$OUT/vbmeta.img" ]; then
  echo "==> Flashing vbmeta..."
  fastboot flash vbmeta "$OUT/vbmeta.img"
fi

if [ "$WIPE_USERDATA" -eq 1 ]; then
  echo "==> Formatting userdata partition..."
  fastboot format userdata || fastboot erase userdata
fi

echo "========================================================="
echo "   OnuronOS Flashed Successfully! Rebooting device...   "
echo "========================================================="
fastboot reboot
