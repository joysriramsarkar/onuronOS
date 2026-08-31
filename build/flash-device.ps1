# build/flash-device.ps1 — Windows PowerShell Fastboot Flasher for Android Devices
param (
    [string]$Target = "aarch64-generic"
)

$TOP = Split-Path -Parent $PSScriptRoot
$OUT = Join-Path $TOP "out\$Target"

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "        NilOS Fastboot Flasher for Android Devices       " -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

if (-not (Get-Command fastboot -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] 'fastboot' not found in PATH." -ForegroundColor Red
    Write-Host "        Please install Android Platform Tools (ADB/Fastboot) from:" -ForegroundColor Yellow
    Write-Host "        https://developer.android.com/tools/releases/platform-tools" -ForegroundColor Gray
    exit 1
}

Write-Host "==> Checking connected Fastboot devices..." -ForegroundColor Yellow
fastboot devices

$confirm = Read-Host "Are you sure you want to flash NilOS to the connected phone? (y/N)"
if ($confirm -ne "y" -and $confirm -ne "Y") {
    Write-Host "Flashing cancelled." -ForegroundColor DarkGray
    exit 0
}

Write-Host "==> Flashing NilOS System..." -ForegroundColor Yellow
$bootImg = Join-Path $OUT "boot.img"
if (Test-Path $bootImg) {
    fastboot flash boot $bootImg
}

$systemImg = Join-Path $OUT "system_a.img"
if (Test-Path $systemImg) {
    fastboot flash system $systemImg
}

$vbmetaImg = Join-Path $OUT "vbmeta_a.img"
if (Test-Path $vbmetaImg) {
    fastboot flash vbmeta --disable-verity --disable-verification $vbmetaImg
}

Write-Host "==> Formatting userdata (fscrypt ready)..." -ForegroundColor Yellow
fastboot erase userdata

Write-Host "=========================================================" -ForegroundColor Green
Write-Host "   NilOS Flashed Successfully! Rebooting device...       " -ForegroundColor Green
Write-Host "=========================================================" -ForegroundColor Green
fastboot reboot
