#!/usr/bin/env bash
# build/qemu-boot.sh — NilOS QEMU Boot Launcher (Linux / macOS / WSL, Phase 1+2)
set -euo pipefail

TOP="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$TOP/out/x86_64-generic"
KERNEL="$OUT/vmlinuz-lts"
INITRD="$OUT/nilos-initramfs.cpio.gz"
DISK="$OUT/nilos.img"

HEADLESS=0
NO_REBUILD=0
NO_DISK=0
NO_NET=0

for arg in "$@"; do
    case "$arg" in
        --headless)    HEADLESS=1 ;;
        --no-rebuild)  NO_REBUILD=1 ;;
        --no-disk)     NO_DISK=1 ;;
        --no-net)      NO_NET=1 ;;
    esac
done

echo "========================================================="
echo "          NilOS QEMU Bootloader (Phase 1+2)              "
echo "========================================================="

# 1. Check QEMU
if ! command -v qemu-system-x86_64 &>/dev/null; then
    echo "Error: qemu-system-x86_64 not found. Install qemu-system-x86 or add to PATH."
    exit 1
fi
echo "[OK] Using QEMU: $(command -v qemu-system-x86_64)"

# 2. Create data partition image if missing
if [ "$NO_DISK" -eq 0 ]; then
    echo "==> Preparing NilOS data partition image..."
    python3 "$TOP/build/mkdisk.py"
    if [ ! -f "$DISK" ]; then
        echo "[WARN] nilos.img not found — booting without persistent disk."
        NO_DISK=1
    else
        echo "[OK]  Data image: $DISK ($(du -m "$DISK" | cut -f1) MB)"
    fi
fi

# 3. Build initramfs if needed
if [ "$NO_REBUILD" -eq 0 ]; then
    echo "==> Generating NilOS initramfs image..."
    python3 "$TOP/build/mkinitramfs.py"
fi

[ -f "$KERNEL" ] || { echo "Error: Kernel missing at $KERNEL"; exit 1; }
[ -f "$INITRD" ] || { echo "Error: Initramfs missing at $INITRD"; exit 1; }

echo "==> Starting NilOS in QEMU..."
echo "    Kernel:    $KERNEL"
echo "    Initramfs: $INITRD"
[ "$NO_DISK" -eq 0 ] && echo "    Data Disk: $DISK (virtio-blk -> /dev/vda -> /data)"

# 4. Assemble QEMU arguments
QEMU_ARGS=(
    -m 1024
    -smp 2
    -kernel "$KERNEL"
    -initrd "$INITRD"
    -append "console=ttyS0 console=tty0 init=/init panic=10 rw"
)

# Persistent data disk
if [ "$NO_DISK" -eq 0 ]; then
    QEMU_ARGS+=(
        -drive "file=$DISK,format=raw,if=none,id=vda_disk,cache=writeback"
        -device "virtio-blk-pci,drive=vda_disk,id=vda"
    )
fi

# Networking (NAT user-mode, no privileges required)
if [ "$NO_NET" -eq 0 ]; then
    QEMU_ARGS+=(
        -netdev user,id=net0
        -device virtio-net-pci,netdev=net0
    )
fi

# Serial + display
if [ "$HEADLESS" -eq 1 ]; then
    echo "    Mode: Headless — all I/O in this terminal"
    echo "    TIP: Type commands here. Exit with Ctrl+A then X."
    QEMU_ARGS+=(-nographic -serial mon:stdio)
else
    echo "    Mode: Graphical Window + Serial console in this terminal"
    echo "    TIP: Type commands in THIS terminal window."
    QEMU_ARGS+=(-vga std -serial stdio)
fi

echo ""
echo "========================================================="
echo "  NilOS UI Controls (once booted):"
echo "    OOBE Wizard:  Follow on-screen prompts"
echo "    Lockscreen:   Enter your PIN"
echo "    Home:         Type 1-8 and Enter to open apps"
echo "    Apps:         Type 'back' or 'home' to navigate"
echo "    Quit QEMU:    Ctrl+A then X (headless mode)"
echo "========================================================="
echo ""

exec qemu-system-x86_64 "${QEMU_ARGS[@]}"
