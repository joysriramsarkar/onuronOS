#!/data/data/com.termux/files/usr/bin/bash
# build/termux-boot.sh — Run NilOS in Termux (Android, no root required)
#
# Modes:
#   bash termux-boot.sh --gui     — Run NilOS Graphical Touch Shell in Termux-X11 (Recommended, Fast!)
#   bash termux-boot.sh --native  — Run NilOS Console Shell in Termux terminal
#   bash termux-boot.sh           — Boot Full NilOS in QEMU x86_64 emulator

set -euo pipefail

TOP="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$TOP/out/x86_64-generic"
KERNEL="$OUT/vmlinuz-lts"
INITRD="$OUT/nilos-initramfs.cpio.gz"
DISK="$OUT/nilos.img"

MODE="qemu"
if [[ "${1:-}" == "--gui" ]]; then
    MODE="gui"
elif [[ "${1:-}" == "--native" ]]; then
    MODE="native"
fi

echo "========================================================="
echo "   NilOS on Android via Termux"
echo "========================================================="

# ─── 1. Graphical Mode: Run nilgui under Termux-X11 ──────────────────────────
if [[ "$MODE" == "gui" ]]; then
    echo "[*] Mode: Graphical Touch UI (Termux-X11)"
    echo ""

    NILOS_DATA="$HOME/nilos-data"
    mkdir -p \
        "$NILOS_DATA/nilos" \
        "$NILOS_DATA/app" \
        "$NILOS_DATA/contacts" \
        "$NILOS_DATA/sms" \
        "$NILOS_DATA/config" \
        "$NILOS_DATA/media" \
        "$NILOS_DATA/logs"

    GUI_BIN="$HOME/nilgui"
    if [ ! -f "$GUI_BIN" ] && [ -f "$TOP/target/aarch64-unknown-linux-musl/release/nilgui" ]; then
        cp "$TOP/target/aarch64-unknown-linux-musl/release/nilgui" "$GUI_BIN"
        chmod +x "$GUI_BIN"
    fi

    if [ ! -f "$GUI_BIN" ]; then
        echo "[!] nilgui binary not found at ~/nilgui!"
        echo "    Download it from your PC or build it with:"
        echo "    cargo build -p nilgui --target aarch64-unknown-linux-musl --release"
        exit 1
    fi

    if ! command -v proot &>/dev/null; then
        echo "[*] Installing proot..."
        pkg install -y proot
    fi

    echo "[OK] Ready! Launching NilOS Graphical Compositor..."
    echo "     Make sure the Termux-X11 app is opened on your screen."
    echo ""
    export DISPLAY="${DISPLAY:-:0}"
    exec proot -b "$NILOS_DATA:/data" "$GUI_BIN"
fi

# ─── 2. Native Console Mode: Run nilshell directly in Termux ─────────────────
if [[ "$MODE" == "native" ]]; then
    echo "[*] Mode: Native ARM64 Terminal Shell"
    echo ""

    NILOS_DATA="$HOME/nilos-data"
    mkdir -p \
        "$NILOS_DATA/nilos" \
        "$NILOS_DATA/app" \
        "$NILOS_DATA/contacts" \
        "$NILOS_DATA/sms"

    SHELL_BIN="$HOME/nilshell"
    if [ ! -f "$SHELL_BIN" ] && [ -f "$TOP/target/aarch64-unknown-linux-musl/release/nilshell" ]; then
        cp "$TOP/target/aarch64-unknown-linux-musl/release/nilshell" "$SHELL_BIN"
        chmod +x "$SHELL_BIN"
    fi

    if ! command -v proot &>/dev/null; then
        pkg install -y proot
    fi

    exec proot -b "$NILOS_DATA:/data" "$SHELL_BIN"
fi

# ─── 3. QEMU Mode: Full x86_64 NilOS Emulation ──────────────────────────────
echo "[*] Mode: Full NilOS x86_64 Virtual Machine in QEMU"
if ! command -v qemu-system-x86_64 &>/dev/null; then
    echo "[!] Installing qemu-system-x86-64..."
    pkg install -y qemu-system-x86-64
fi

exec qemu-system-x86_64 \
    -m 512 \
    -smp 2 \
    -kernel "$KERNEL" \
    -initrd "$INITRD" \
    -append "console=ttyS0 init=/init panic=10 rw" \
    -drive "file=$DISK,format=raw,if=none,id=vda_disk,cache=writeback" \
    -device "virtio-blk-pci,drive=vda_disk,id=vda" \
    -nographic \
    -serial mon:stdio \
    -no-acpi \
    -cpu max
