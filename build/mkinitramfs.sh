#!/usr/bin/env bash
# build/mkinitramfs.sh — 2-stage initramfs creator with recovery and bootsplash
set -euo pipefail
OUT="${1:-out/x86_64-generic}"
INIT_DIR="$OUT/initramfs_root"

mkdir -p "$INIT_DIR"/{bin,sbin,dev,proc,sys,newroot,etc}
if [ -f "$OUT/rootfs/usr/bin/nilinit" ]; then
  cp "$OUT/rootfs/usr/bin/nilinit" "$INIT_DIR/init"
fi

echo "[OK] initramfs structure prepared in $INIT_DIR"
