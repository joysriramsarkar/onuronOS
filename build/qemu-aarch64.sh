#!/usr/bin/env bash
# build/qemu-aarch64.sh — OnuronOS ARM64 QEMU Virt Boot Launcher
set -euo pipefail

TOP="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$TOP/out/aarch64-qemu"
KERNEL="$OUT/vmlinuz-lts"
INITRD="$OUT/initramfs.cpio.gz"
DISK="$OUT/data.img"

HEADLESS=1
NO_REBUILD=0
NO_DISK=0
NO_NET=0
MEM="1024"
SMP="2"
CPU="cortex-a57"

for arg in "$@"; do
    case "$arg" in
        --gui)          HEADLESS=0 ;;
        --headless)     HEADLESS=1 ;;
        --no-rebuild)   NO_REBUILD=0 ;;
        --no-disk)      NO_DISK=1 ;;
        --no-net)       NO_NET=1 ;;
        -h|--help)
            echo "Usage: $0 [--gui] [--headless] [--no-rebuild] [--no-disk] [--no-net]"
            exit 0
            ;;
        *) echo "Error: unknown option: $arg" >&2; exit 2 ;;
    esac
done

echo "========================================================="
echo "       OnuronOS ARM64 QEMU Virt Target Launcher          "
echo "========================================================="

# 1. Verify qemu-system-aarch64
if ! command -v qemu-system-aarch64 &>/dev/null; then
    echo "Error: qemu-system-aarch64 not found in PATH." >&2
    echo "Install QEMU system emulators (e.g. qemu-system-arm / qemu-system-aarch64)." >&2
    exit 1
fi
echo "[OK] QEMU binary: $(command -v qemu-system-aarch64)"

# 2. Build or verify aarch64 initramfs and kernel
if [ "$NO_REBUILD" -eq 0 ] && ([ ! -f "$KERNEL" ] || [ ! -f "$INITRD" ]); then
    echo "==> Preparing ARM64 kernel & initramfs..."
    python3 "$TOP/build/mkinitramfs.py" --arch aarch64
fi

[ -f "$KERNEL" ] || { echo "Error: ARM64 kernel missing at $KERNEL" >&2; exit 1; }
[ -f "$INITRD" ] || { echo "Error: ARM64 initramfs missing at $INITRD" >&2; exit 1; }

# 3. Create persistent disk if requested
if [ "$NO_DISK" -eq 0 ] && [ ! -f "$DISK" ]; then
    echo "==> Creating 512MB data disk image at $DISK..."
    mkdir -p "$OUT"
    truncate -s 512M "$DISK"
fi

echo "==> Launching OnuronOS aarch64 virt machine..."
echo "    Kernel:    $KERNEL"
echo "    Initramfs: $INITRD"
echo "    CPU:       $CPU (Cores: $SMP, RAM: ${MEM}MB)"

QEMU_ARGS=(
    -M virt
    -cpu "$CPU"
    -smp "$SMP"
    -m "$MEM"
    -kernel "$KERNEL"
    -initrd "$INITRD"
    -append "console=ttyAMA0 earlycon root=/dev/ram0 rdinit=/init panic=10 rw"
)

# Persistent storage over VirtIO block
if [ "$NO_DISK" -eq 0 ] && [ -f "$DISK" ]; then
    QEMU_ARGS+=(
        -drive "file=$DISK,format=raw,if=none,id=vda_disk,cache=writeback"
        -device "virtio-blk-pci,drive=vda_disk,id=vda"
    )
fi

# VirtIO network (user mode NAT)
if [ "$NO_NET" -eq 0 ]; then
    QEMU_ARGS+=(
        -netdev user,id=net0
        -device virtio-net-pci,netdev=net0
    )
fi

# Display & Serial config
if [ "$HEADLESS" -eq 1 ]; then
    echo "    Console:   Serial console (ttyAMA0) mapped to stdio"
    echo "    Notice:    To exit QEMU, press Ctrl+A then X"
    QEMU_ARGS+=(-nographic -serial mon:stdio)
else
    echo "    Console:   VirtIO GPU Display + ttyAMA0 on stdio"
    QEMU_ARGS+=(
        -device virtio-gpu-pci
        -device virtio-keyboard-pci
        -device virtio-tablet-pci
        -serial stdio
    )
fi

exec qemu-system-aarch64 "${QEMU_ARGS[@]}"
