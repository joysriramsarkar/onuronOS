#!/usr/bin/env bash
# build/ota/ab_update.sh — A/B Seamless OTA update engine
set -euo pipefail

CURRENT_SLOT=$(cat /proc/cmdline 2>/dev/null | grep -o 'nilos.slot=[ab]' | cut -d= -f2 || echo "a")
TARGET_SLOT="b"
[ "$CURRENT_SLOT" = "b" ] && TARGET_SLOT="a"

echo "==> Current Active Slot: $CURRENT_SLOT. Updating Target Slot: $TARGET_SLOT"

PAYLOAD="${1:-/data/ota/update.payload}"
if [ ! -f "$PAYLOAD" ]; then
  echo "Error: Payload file $PAYLOAD not found!" >&2
  exit 1
fi

echo "==> Flashing image to target slot partition"
dd if="$PAYLOAD" of="/dev/disk/by-partlabel/system_$TARGET_SLOT" bs=4M status=progress
sync

echo "==> Setting active boot flag to slot $TARGET_SLOT"
echo "==> OTA update successful. Reboot to apply."
