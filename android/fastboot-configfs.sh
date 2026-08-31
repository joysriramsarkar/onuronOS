#!/usr/bin/env bash
# android/fastboot-configfs.sh — USB ConfigFS gadget setup for Fastboot
set -euo pipefail

CONFIGFS="/sys/kernel/config/usb_gadget/g1"
if [ -d "/sys/kernel/config/usb_gadget" ]; then
  mkdir -p "$CONFIGFS"
  echo "0x18d1" > "$CONFIGFS/idVendor"
  echo "0xd00d" > "$CONFIGFS/idProduct"
  echo "[OK] Fastboot USB gadget configured."
fi
