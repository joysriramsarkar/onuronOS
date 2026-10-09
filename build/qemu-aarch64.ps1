# build/qemu-aarch64.ps1 — OnuronOS ARM64 QEMU Virt Boot Launcher (Windows)
param (
    [switch]$Gui,
    [switch]$NoRebuild,
    [switch]$NoDisk,
    [switch]$NoNet,
    [string]$Cpu = "cortex-a57",
    [int]$MemoryMb = 1024,
    [int]$Smp = 2
)

$ErrorActionPreference = "Stop"
$TOP = Split-Path -Parent $PSScriptRoot
$OUT = Join-Path $TOP "out\aarch64-qemu"
$KERNEL = Join-Path $OUT "vmlinuz-lts"
$INITRD = Join-Path $OUT "initramfs.cpio.gz"
$DISK   = Join-Path $OUT "data.img"

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "       OnuronOS ARM64 QEMU Virt Launcher (Windows)       " -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

# 1. Locate qemu-system-aarch64
$qemu = "C:\Program Files\qemu\qemu-system-aarch64.exe"
if (-not (Test-Path $qemu)) {
    $cmdQemu = Get-Command qemu-system-aarch64 -ErrorAction SilentlyContinue
    if ($cmdQemu) { $qemu = $cmdQemu.Source }
    else {
        Write-Error "qemu-system-aarch64 not found! Please install QEMU with ARM64 support or add to PATH."
        exit 1
    }
}
Write-Host "[OK] Using QEMU: $qemu" -ForegroundColor Green

# 2. Build or download ARM64 kernel & initramfs
if (-not $NoRebuild -and ((-not (Test-Path $KERNEL)) -or (-not (Test-Path $INITRD)))) {
    Write-Host "==> Preparing ARM64 kernel and initramfs..." -ForegroundColor Yellow
    python (Join-Path $TOP "build\mkinitramfs.py") --arch aarch64
}

if (-not (Test-Path $KERNEL)) {
    Write-Error "ARM64 kernel missing at $KERNEL"
    exit 1
}
if (-not (Test-Path $INITRD)) {
    Write-Error "ARM64 initramfs missing at $INITRD"
    exit 1
}

# 3. Create persistent disk
if (-not $NoDisk) {
    if (-not (Test-Path $DISK)) {
        Write-Host "==> Creating 512MB data disk image at $DISK..." -ForegroundColor Yellow
        if (-not (Test-Path $OUT)) { New-Item -ItemType Directory -Path $OUT -Force | Out-Null }
        $f = [System.IO.File]::Create($DISK)
        $f.SetLength(512MB)
        $f.Close()
    }
    Write-Host "[OK] Data disk: $DISK" -ForegroundColor Green
}

Write-Host "==> Starting OnuronOS in QEMU aarch64 Virt..." -ForegroundColor Cyan
Write-Host "    Kernel:    $KERNEL"
Write-Host "    Initramfs: $INITRD"
Write-Host "    CPU:       $Cpu ($Smp cores, ${MemoryMb}MB RAM)"

$qemuArgs = @(
    "-M", "virt",
    "-cpu", $Cpu,
    "-smp", "$Smp",
    "-m", "$MemoryMb",
    "-kernel", $KERNEL,
    "-initrd", $INITRD,
    "-append", "console=ttyAMA0 earlycon root=/dev/ram0 rdinit=/init panic=10 rw"
)

if (-not $NoDisk -and (Test-Path $DISK)) {
    $qemuArgs += @(
        "-drive", "file=$DISK,format=raw,if=none,id=vda_disk,cache=writeback",
        "-device", "virtio-blk-pci,drive=vda_disk,id=vda"
    )
}

if (-not $NoNet) {
    $qemuArgs += @(
        "-netdev", "user,id=net0",
        "-device", "virtio-net-pci,netdev=net0"
    )
}

if (-not $Gui) {
    Write-Host "    Mode: Headless (Serial console mapped to stdio). Exit: Ctrl+A then X." -ForegroundColor Yellow
    $qemuArgs += @("-nographic", "-serial", "mon:stdio")
} else {
    Write-Host "    Mode: Graphical VirtIO GPU + Serial" -ForegroundColor Yellow
    $qemuArgs += @("-device", "virtio-gpu-pci", "-device", "virtio-keyboard-pci", "-device", "virtio-tablet-pci", "-serial", "stdio")
}

& $qemu $qemuArgs
