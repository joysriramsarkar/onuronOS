#!/usr/bin/env bash
# build/qemu-run.sh — Test NilOS in QEMU with 120Hz display & virtio GPU
set -euo pipefail
TOP="$(cd "$(dirname "$0")/.." && pwd)"
IMG="$TOP/out/x86_64-generic/nilos-disk.raw"

qemu-system-x86_64 \
  -enable-kvm \
  -m 2048 \
  -smp 4 \
  -drive file="$IMG",format=raw,if=virtio \
  -device virtio-vga-gl \
  -display gtk,gl=on \
  -device virtio-tablet-pci \
  -device virtio-keyboard-pci \
  -serial mon:stdio \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net-pci,netdev=net0
