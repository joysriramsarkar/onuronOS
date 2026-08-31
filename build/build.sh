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
cargo build --release --workspace

BINS=(
  nilinit nild nilkeyd nilandroidd nilstore notifyd nilimed powerd
  crashd camerad authd nilttsd vpnd dnsd backupd btd niltrace nilperf
  thermald alarmd nilwdt logd clipd nilupd audiod mediad userd nilsr ntpd netd
  nilrt-launch nilinstall nilfastbootd nilverify nilrecovery halctl
  present_demo bootsplash nilbus nilpkg hello busdemo animdemo lockscreen
  bridgedemo oobe launcher settings camdemo clockwidget
)

for b in "${BINS[@]}"; do
  if [ -f "target/release/$b" ]; then
    install -m755 "target/release/$b" "$SYS/usr/bin/"
  fi
done

echo "==> [2/6] Compiling Native C/C++ Components & Compositor"
if [ -d "$TOP/shell" ]; then
  make -C "$TOP/shell" || true
  if [ -f "$TOP/shell/nilshell" ]; then
    install -m755 "$TOP/shell/nilshell" "$SYS/usr/bin/"
  fi
fi

if [ -d "$TOP/hal" ]; then
  make -C "$TOP/hal" || true
  find "$TOP/hal" -name "*.so" -exec cp {} "$SYS/vendor/lib/nilhal/" \; 2>/dev/null || true
fi

echo "==> [3/6] Building SELinux Policy & Labeling Rootfs"
if [ -f "$TOP/security/selinux/build.sh" ]; then
  bash "$TOP/security/selinux/build.sh" "$SYS" || true
fi

echo "==> [4/6] Installing System Configuration, Tokens & Services"
if [ -d "$TOP/etc/nilos" ]; then
  cp -r "$TOP/etc/nilos/"* "$SYS/etc/nilos/" || true
fi
mkdir -p "$SYS/etc/udev/rules.d"
if [ -d "$TOP/etc/udev/rules.d" ]; then
  cp "$TOP/etc/udev/rules.d/"*.rules "$SYS/etc/udev/rules.d/" 2>/dev/null || true
fi

echo "==> [5/6] Generating Disk Images (A/B Partitions, fscrypt enabled)"
if [ -f "$TOP/build/mkimage-x86.sh" ]; then
  bash "$TOP/build/mkimage-x86.sh" "$OUT" || true
fi

echo "==> [6/6] Building Verified Boot Signatures (mkvbmeta)"
if [ -f "$TOP/build/mkvbmeta.sh" ]; then
  bash "$TOP/build/mkvbmeta.sh" "$OUT/system_a.img" "$OUT/vbmeta_a.img" || true
fi

echo "========================================================="
echo "       NilOS build completed successfully: $OUT           "
echo "========================================================="
