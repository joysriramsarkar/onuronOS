# build/qemu-boot.ps1 — NilOS QEMU Boot Launcher (Phase 1+2)
param (
    [switch]$Headless,
    [switch]$NoRebuild,
    [switch]$NoDisk,
    [switch]$NoNet
)

$ErrorActionPreference = "Stop"
$TOP = Split-Path -Parent $PSScriptRoot
$OUT = Join-Path $TOP "out\x86_64-generic"
$KERNEL = Join-Path $OUT "vmlinuz-lts"
$INITRD = Join-Path $OUT "nilos-initramfs.cpio.gz"
$DISK   = Join-Path $OUT "nilos.img"

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "          NilOS QEMU Bootloader (Phase 1+2)              " -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

# 1. Check QEMU
$qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"
if (-not (Test-Path $qemu)) {
    $cmdQemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
    if ($cmdQemu) { $qemu = $cmdQemu.Source }
    else {
        Write-Error "QEMU not found! Please install QEMU or add it to PATH."
        exit 1
    }
}
Write-Host "[OK] Using QEMU: $qemu" -ForegroundColor Green

# 2. Create data partition image if missing
if (-not $NoDisk) {
    Write-Host "==> Preparing NilOS data partition image..." -ForegroundColor Yellow
    python (Join-Path $TOP "build\mkdisk.py")
    if (-not (Test-Path $DISK)) {
        Write-Host "[WARN] nilos.img not found — booting without persistent disk." -ForegroundColor Yellow
        $NoDisk = $true
    } else {
        Write-Host "[OK]  Data image: $DISK ($([math]::Round((Get-Item $DISK).Length / 1MB, 1)) MB)" -ForegroundColor Green
    }
}

# 3. Build initramfs & download kernel if needed
if (-not $NoRebuild) {
    Write-Host "==> Generating NilOS initramfs image..." -ForegroundColor Yellow
    python (Join-Path $TOP "build\mkinitramfs.py")
}

if (-not (Test-Path $KERNEL)) {
    Write-Error "Kernel image missing at $KERNEL"
    exit 1
}
if (-not (Test-Path $INITRD)) {
    Write-Error "Initramfs missing at $INITRD"
    exit 1
}

Write-Host "==> Starting NilOS in QEMU..." -ForegroundColor Yellow
Write-Host "    Kernel:    $KERNEL" -ForegroundColor DarkGray
Write-Host "    Initramfs: $INITRD" -ForegroundColor DarkGray
if (-not $NoDisk) {
    Write-Host "    Data Disk: $DISK (virtio-blk -> /dev/vda -> /data)" -ForegroundColor DarkGray
}

# 4. Build QEMU arguments
$qemuArgs = @(
    "-m", "1024",
    "-smp", "2",
    "-kernel", $KERNEL,
    "-initrd", $INITRD,
    "-append", "console=ttyS0 console=tty0 init=/init panic=10 rw"
)

# Persistent data disk (virtio-blk)
if (-not $NoDisk) {
    $qemuArgs += @(
        "-drive", "file=$DISK,format=raw,if=virtio,cache=writeback",
        "-device", "virtio-blk-pci,drive=vda,id=vda"
    )
    $qemuArgs[-3] = "file=$DISK,format=raw,if=none,id=vda_disk,cache=writeback"
    $qemuArgs[-1] = "virtio-blk-pci,drive=vda_disk,id=vda"
}

# Networking (NAT + user mode — no privileges needed)
if (-not $NoNet) {
    $qemuArgs += @(
        "-netdev", "user,id=net0",
        "-device", "virtio-net-pci,netdev=net0"
    )
}

# Serial + display mode
if (-not $Headless) {
    Write-Host "    Mode: Graphical Window + Serial console" -ForegroundColor DarkCyan
    Write-Host "    (QEMU window opens + serial output in this terminal)" -ForegroundColor Gray
    Write-Host "    TIP: Type commands in THIS terminal window." -ForegroundColor Yellow
    $qemuArgs += @("-vga", "std", "-serial", "stdio")
} else {
    Write-Host "    Mode: Headless — all I/O in this terminal" -ForegroundColor DarkCyan
    Write-Host "    TIP: Type commands directly here." -ForegroundColor Yellow
    $qemuArgs += @("-nographic", "-serial", "mon:stdio")
}

Write-Host ""
Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "  NilOS UI Controls (once booted):" -ForegroundColor White
Write-Host "    OOBE Wizard:  Follow on-screen prompts" -ForegroundColor Gray
Write-Host "    Lockscreen:   Enter your PIN" -ForegroundColor Gray
Write-Host "    Home:         Type 1-8 and Enter to open apps" -ForegroundColor Gray
Write-Host "    Apps:         Type 'back' or 'home' to navigate" -ForegroundColor Gray
Write-Host "    Quit QEMU:    Press Ctrl+A then X (headless)" -ForegroundColor Gray
Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host ""

& $qemu @qemuArgs
