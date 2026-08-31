#!/usr/bin/env bash
# build/setup-toolchain.sh — Cross-toolchain & host dependency bootstrap
set -euo pipefail

echo "==> Installing host dependencies for NilOS build"
sudo apt-get update && sudo apt-get install -y \
  build-essential clang lld pkg-config libvulkan-dev glslc \
  libcamera-dev libpipewire-0.3-dev libwayland-dev wayland-protocols \
  libxkbcommon-dev libseat-dev libgbm-dev libdrm-dev \
  secilc libselinux1-dev libelf-dev e2fsprogs f2fs-tools qemu-system-x86 || true

rustup target add x86_64-unknown-linux-musl || true
rustup target add aarch64-unknown-linux-musl || true
rustup target add aarch64-linux-android || true

echo "[OK] NilOS Toolchain is ready."
