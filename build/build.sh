#!/usr/bin/env bash
# build/build.sh — NilOS Complete Image Builder
# Usage: ./build.sh [x86_64-generic | arm64-generic | target-device]
set -euo pipefail

DEVICE="${1:-x86_64-generic}"
TOP="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$TOP/out/$DEVICE"
SYS="$OUT/rootfs"

echo "========================================================="
echo "             Building NilOS for $DEVICE                  "
echo "========================================================="

rm -rf "$OUT"
mkdir -p "$SYS"/{bin,usr/bin,usr/lib,etc/nilos/apps,data/app,data/user,proc,sys,dev,run/nilos,mnt,vendor/lib/nilhal}

echo "==> [1/6] Compiling Userspace (Rust statically linked)"
TARGET_DIR="target/release"
if [ "$DEVICE" = "arm64-generic" ] || [ "$DEVICE" = "aarch64-generic" ] || [ "$DEVICE" = "aarch64-qemu" ]; then
  cargo build --release --workspace --target aarch64-unknown-linux-musl
  TARGET_DIR="target/aarch64-unknown-linux-musl/release"
elif [ "$DEVICE" = "x86_64-generic" ]; then
  if rustup target list 2>/dev/null | grep -q "x86_64-unknown-linux-musl (installed)"; then
    cargo build --release --workspace --target x86_64-unknown-linux-musl
    TARGET_DIR="target/x86_64-unknown-linux-musl/release"
  else
    cargo build --release --workspace
  fi
else
  cargo build --release --workspace
fi

BINS=(
  nilinit nild nilkeyd nilandroidd nilstore notifyd nilimed powerd
  crashd camerad authd nilttsd vpnd dnsd backupd btd telephonyd niltrace nilperf
  thermald alarmd nilwdt logd clipd nilupd audiod mediad userd nilsr ntpd netd
  nilrt-launch nilinstall nilfastbootd nilverify nilrecovery halctl
  present_demo bootsplash nilbus nilpkg hello busdemo animdemo lockscreen
  bridgedemo oobe launcher settings camdemo clockwidget nilshell nilc nilrt
)

for b in "${BINS[@]}"; do
  if [ -f "$TARGET_DIR/$b" ]; then
    install -m755 "$TARGET_DIR/$b" "$SYS/usr/bin/"
  fi
done

echo "==> [2/6] Compiling Native C/C++ Components & Compositor"
if [ -f "$TOP/shell/Makefile" ]; then
  make -C "$TOP/shell"
  if [ -f "$TOP/shell/nilshell" ]; then
    install -m755 "$TOP/shell/nilshell" "$SYS/usr/bin/"
  fi
fi

if [ -f "$TOP/hal/Makefile" ]; then
  make -C "$TOP/hal"
  find "$TOP/hal" -name "*.so" -exec cp {} "$SYS/vendor/lib/nilhal/" \; 2>/dev/null || true
fi

echo "==> [3/6] Building SELinux Policy & Labeling Rootfs"
if [ -f "$TOP/security/selinux/build.sh" ]; then
  bash "$TOP/security/selinux/build.sh" "$SYS"
fi

echo "==> [4/6] Installing System Configuration, Tokens & Services"
if [ -d "$TOP/etc/nilos" ]; then
  cp -r "$TOP/etc/nilos/"* "$SYS/etc/nilos/"
fi
mkdir -p "$SYS/etc/udev/rules.d"
if [ -d "$TOP/etc/udev/rules.d" ]; then
  cp "$TOP/etc/udev/rules.d/"*.rules "$SYS/etc/udev/rules.d/" 2>/dev/null || true
fi

echo "==> [5/6] Generating Disk Images (A/B Partitions, fscrypt enabled)"
if [ -f "$TOP/build/mkimage-x86.sh" ] && [ "$DEVICE" = "x86_64-generic" ]; then
  bash "$TOP/build/mkimage-x86.sh" "$OUT"
fi

echo "==> [6/6] Building Initramfs, Checksums & Verified Boot Signatures"
ARCH_PARAM="x86_64"
if [ "$DEVICE" = "arm64-generic" ] || [ "$DEVICE" = "aarch64-generic" ] || [ "$DEVICE" = "aarch64-qemu" ]; then
  ARCH_PARAM="aarch64"
fi

python3 "$TOP/build/mkinitramfs.py" --arch "$ARCH_PARAM"

if [ -f "$TOP/build/mkvbmeta.sh" ] && [ -f "$OUT/system_a.img" ]; then
  bash "$TOP/build/mkvbmeta.sh" "$OUT/system_a.img" "$OUT/vbmeta_a.img"
fi

echo "========================================================="
echo "       NilOS build completed successfully: $OUT           "
echo "========================================================="
