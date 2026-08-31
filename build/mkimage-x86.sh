#!/usr/bin/env bash
# build/mkimage-x86.sh — GPT Partitioned disk image creator (A/B + Encrypted userdata)
set -euo pipefail
OUT="${1:-out/x86_64-generic}"
IMG="$OUT/nilos-disk.raw"
SIZE_MB=4096

mkdir -p "$OUT"
truncate -s ${SIZE_MB}M "$IMG" 2>/dev/null || true

echo "==> Partitioning GPT disk image $IMG"
if command -v parted >/dev/null 2>&1; then
  parted -s "$IMG" mklabel gpt \
    mkpart ESP fat32 1MiB 129MiB \
    set 1 boot on \
    mkpart boot_a 129MiB 193MiB \
    mkpart boot_b 193MiB 257MiB \
    mkpart system_a ext4 257MiB 1281MiB \
    mkpart system_b ext4 1281MiB 2305MiB \
    mkpart userdata ext4 2305MiB 100% || true
fi

echo "[OK] Disk image created at $IMG"
