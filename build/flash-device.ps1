# build/flash-device.ps1 — Windows PowerShell Fastboot Flasher for OnuronOS Targets
param (
    [string]$Target = "aarch64-generic",
    [switch]$WipeUserdata,
    [switch]$ForceUnsupported
)

$TOP = Split-Path -Parent $PSScriptRoot
$OUT = Join-Path $TOP "out\$Target"

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "       OnuronOS Safe Fastboot Flasher (Hardware Gate)    " -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

if (-not (Get-Command fastboot -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] 'fastboot' not found in PATH." -ForegroundColor Red
    Write-Host "        Please install Android Platform Tools (ADB/Fastboot) from:" -ForegroundColor Yellow
    Write-Host "        https://developer.android.com/tools/releases/platform-tools" -ForegroundColor Gray
    exit 1
}

Write-Host "==> Checking connected Fastboot devices..." -ForegroundColor Yellow
$devices = fastboot devices
if (-not $devices) {
    Write-Host "[ERROR] No device connected in fastboot mode." -ForegroundColor Red
    exit 1
}
Write-Host $devices

# Device identity verification
$varOutput = fastboot getvar product 2>&1 | Out-String
$deviceProduct = "unknown"
if ($varOutput -match "product:\s*(\S+)") {
    $deviceProduct = $Matches[1]
}
Write-Host "[INFO] Connected device product identifier: $deviceProduct" -ForegroundColor Cyan

if ($Target -eq "oneplus-fajita" -or $Target -eq "fajita") {
    if ($deviceProduct -ne "fajita" -and -not $ForceUnsupported) {
        Write-Host "[ERROR] Device product '$deviceProduct' does not match target 'fajita'!" -ForegroundColor Red
        Write-Host "        Aborting to protect hardware. Pass -ForceUnsupported to override." -ForegroundColor Yellow
        exit 1
    }
} elseif ($Target -eq "aarch64-generic" -and -not $ForceUnsupported) {
    Write-Host "[SECURITY GATE] Target 'aarch64-generic' is an unsupported generic/virtual image." -ForegroundColor Red
    Write-Host "                Flashing generic image to real hardware can brick it. Pass -ForceUnsupported to override." -ForegroundColor Yellow
    exit 1
}

$bootImg = Join-Path $OUT "boot.img"
$kernelPath = Join-Path $OUT "kernel"
$initrdPath = Join-Path $OUT "initramfs.cpio.gz"
if (-not (Test-Path $bootImg) -and (Test-Path $kernelPath) -and (Test-Path $initrdPath)) {
    Write-Host "==> Packaging Android boot.img using build/mkbootimg.py..." -ForegroundColor Yellow
    python (Join-Path $TOP "build\mkbootimg.py") create --kernel $kernelPath --ramdisk $initrdPath -o $bootImg
}
if (-not (Test-Path $bootImg)) {
    Write-Host "[ERROR] Boot image not found at $bootImg" -ForegroundColor Red
    exit 1
}

$systemImg = Join-Path $OUT "system_a.img"
if (-not (Test-Path $systemImg)) {
    $systemImg = Join-Path $OUT "system.img"
}
if (-not (Test-Path $systemImg)) {
    Write-Host "[ERROR] Valid system image not found in $OUT" -ForegroundColor Red
    exit 1
}

Write-Host "==> Target images validated:" -ForegroundColor Green
Write-Host "    Target:     $Target"
Write-Host "    Boot image: $bootImg"
Write-Host "    System:     $systemImg"
if ($WipeUserdata) {
    Write-Host "    Userdata:   WILL BE ERASED (-WipeUserdata passed)" -ForegroundColor Yellow
} else {
    Write-Host "    Userdata:   Preserved (Use -WipeUserdata to erase)" -ForegroundColor Green
}

$confirm = Read-Host "Are you sure you want to flash OnuronOS to the connected phone? (y/N)"
if ($confirm -ne "y" -and $confirm -ne "Y") {
    Write-Host "Flashing cancelled." -ForegroundColor DarkGray
    exit 0
}

Write-Host "==> Flashing boot partition..." -ForegroundColor Yellow
fastboot flash boot $bootImg

Write-Host "==> Flashing system partition..." -ForegroundColor Yellow
fastboot flash system $systemImg

$vbmetaImg = Join-Path $OUT "vbmeta_a.img"
if (-not (Test-Path $vbmetaImg)) {
    $vbmetaImg = Join-Path $OUT "vbmeta.img"
}
if (Test-Path $vbmetaImg) {
    Write-Host "==> Flashing vbmeta..." -ForegroundColor Yellow
    fastboot flash vbmeta --disable-verity --disable-verification $vbmetaImg
}

if ($WipeUserdata) {
    Write-Host "==> Formatting userdata (fscrypt ready)..." -ForegroundColor Yellow
    fastboot erase userdata
}

Write-Host "=========================================================" -ForegroundColor Green
Write-Host "   OnuronOS Flashed Successfully! Rebooting device...   " -ForegroundColor Green
Write-Host "=========================================================" -ForegroundColor Green
fastboot reboot
