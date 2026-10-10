# android-host/build-ndk.ps1 — Cross-compile android-host Rust crate for Android aarch64 (arm64-v8a)
param (
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$JniLibsDir = Join-Path $ScriptDir "app\src\main\jniLibs\arm64-v8a"
$TargetSo = Join-Path $JniLibsDir "libandroid_host.so"
$ManifestFile = Join-Path $JniLibsDir "build_manifest.json"
$Target = "aarch64-linux-android"

if ($Clean) {
    Write-Host "==> Cleaning staged Android native libraries..." -ForegroundColor Yellow
    if (Test-Path $TargetSo) { Remove-Item $TargetSo -Force }
    if (Test-Path $ManifestFile) { Remove-Item $ManifestFile -Force }
    Write-Host "[OK] Cleaned $JniLibsDir" -ForegroundColor Green
    exit 0
}

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "       Building Android Host Native Library ($Target)    " -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

if (-not (Test-Path $JniLibsDir)) {
    New-Item -ItemType Directory -Path $JniLibsDir -Force | Out-Null
}
if (Test-Path $TargetSo) { Remove-Item $TargetSo -Force }

if (Get-Command cargo-ndk -ErrorAction SilentlyContinue) {
    Write-Host "[OK] Using cargo-ndk" -ForegroundColor Green
    cargo ndk -t arm64-v8a -o (Join-Path $ScriptDir "app\src\main\jniLibs") build --release --manifest-path (Join-Path $ScriptDir "Cargo.toml")
} else {
    Write-Host "[INFO] cargo-ndk not found, using cargo --target $Target" -ForegroundColor Yellow
    cargo build --target $Target --release --manifest-path (Join-Path $ScriptDir "Cargo.toml")
    $srcSo = Join-Path $ScriptDir "..\target\$Target\release\libandroid_host.so"
    if (-not (Test-Path $srcSo)) {
        Write-Error "Compiled library not found at $srcSo"
        exit 1
    }
    Copy-Item $srcSo $TargetSo -Force
}

if (-not (Test-Path $TargetSo)) {
    Write-Error "libandroid_host.so not found at $TargetSo"
    exit 1
}

$sha = (Get-FileHash -Path $TargetSo -Algorithm SHA256).Hash.ToLower()
$size = (Get-Item $TargetSo).Length

$manifest = @{
    target = $Target
    abi = "arm64-v8a"
    library = "libandroid_host.so"
    sha256 = $sha
    size_bytes = $size
    verified = $true
} | ConvertTo-Json -Depth 3

Set-Content -Path $ManifestFile -Value $manifest -Encoding UTF8

Write-Host "[OK] Staged libandroid_host.so at $TargetSo" -ForegroundColor Green
Write-Host "[OK] Manifest generated: $ManifestFile" -ForegroundColor Green
