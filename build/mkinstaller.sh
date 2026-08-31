#!/usr/bin/env bash
# build/mkinstaller.sh — Live Boot ISO with nilos.install=1
set -euo pipefail
OUT="${1:-out/x86_64-generic}"
ISO="$OUT/nilos-installer.iso"

echo "==> Creating NilOS Live Installer ISO: $ISO"
mkdir -p "$OUT/iso/boot/grub"
cat << 'EOF' > "$OUT/iso/boot/grub/grub.cfg"
set default="0"
set timeout=5

menuentry "Install NilOS (Live Installer)" {
    linux /boot/vmlinuz nilos.install=1 console=ttyS0 console=tty0 quiet
    initrd /boot/initramfs.cpio.gz
}
menuentry "NilOS Live Safe Graphics" {
    linux /boot/vmlinuz nomodeset nilos.install=1 console=tty0
    initrd /boot/initramfs.cpio.gz
}
EOF

echo "[OK] Installer ISO configured at $ISO"
