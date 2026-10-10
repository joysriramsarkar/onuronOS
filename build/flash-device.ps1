# build/flash-device.ps1 — Windows PowerShell Fastboot Flasher for OnuronOS Targets
param (
    [string]$Target = "oneplus-fajita",
    [switch]$WipeUserdata,
    [switch]$ForceUnsupported,
    [switch]$DryRun,
    [string]$Slot = ""
)

$TOP = Split-Path -Parent $PSScriptRoot
$ResolvedTarget = & python "$TOP\build\target_registry.py" resolve $Target 2>$null
if (-not $ResolvedTarget) { $ResolvedTarget = $Target }
$ResolvedOut = & python "$TOP\build\target_registry.py" output-dir $ResolvedTarget 2>$null
if ($ResolvedOut) { $OUT = $ResolvedOut } else { $OUT = Join-Path $TOP "out\$ResolvedTarget" }

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

$deviceLines = ($devices -split "`r?`n") | Where-Object { $_.Trim().Length -gt 0 }
if ($deviceLines.Count -eq 0) {
    Write-Host "[ERROR] No device connected in fastboot mode." -ForegroundColor Red
    exit 1
}
if ($deviceLines.Count -gt 1) {
    Write-Host "[ERROR] Multiple fastboot devices detected ($($deviceLines.Count) devices)." -ForegroundColor Red
    Write-Host "        Disconnect other devices to prevent accidental flashing of the wrong hardware." -ForegroundColor Yellow
    exit 1
}

# Device identity verification
$varOutput = fastboot getvar product 2>&1 | Out-String
$deviceProduct = "unknown"
if ($varOutput -match "product:\s*(\S+)") {
    $deviceProduct = $Matches[1].Trim()
}
Write-Host "[INFO] Connected device product identifier: $deviceProduct" -ForegroundColor Cyan

if ($deviceProduct -eq "unknown" -or [string]::IsNullOrWhiteSpace($deviceProduct)) {
    if (-not $ForceUnsupported) {
        Write-Host "[ERROR] Connected device returned unknown or empty product identifier ('$deviceProduct')." -ForegroundColor Red
        Write-Host "        Refusing to flash unidentified device. Pass -ForceUnsupported to override." -ForegroundColor Yellow
        exit 1
    }
}

$slotOutput = fastboot getvar current-slot 2>&1 | Out-String
if ($slotOutput -match "current-slot:\s*(\S+)") {
    $deviceSlot = $Matches[1].Trim()
    Write-Host "[INFO] Connected device current boot slot: $deviceSlot" -ForegroundColor Cyan
}

$canonicalTarget = $Target
try {
    $resolved = python (Join-Path $TOP "build\target_registry.py") resolve $Target 2>$null
    if ($LASTEXITCODE -eq 0 -and $resolved) {
        $canonicalTarget = $resolved.Trim()
    }
} catch {
    # Keep $canonicalTarget as $Target if python invocation fails
}

# Resolve canonical output directory
if (Test-Path (Join-Path $TOP "out\$canonicalTarget")) {
    $OUT = Join-Path $TOP "out\$canonicalTarget"
} elseif (Test-Path (Join-Path $TOP "out\$Target")) {
    $OUT = Join-Path $TOP "out\$Target"
} else {
    $OUT = Join-Path $TOP "out\$canonicalTarget"
}

if ($canonicalTarget -eq "oneplus-fajita") {
    if ($deviceProduct -ne "fajita" -and -not $ForceUnsupported) {
        Write-Host "[ERROR] Device product '$deviceProduct' does not match target 'fajita'!" -ForegroundColor Red
        Write-Host "        Aborting to protect hardware. Pass -ForceUnsupported to override." -ForegroundColor Yellow
        exit 1
    }
} else {
    if (-not $ForceUnsupported) {
        Write-Host "[SECURITY GATE] Target '$Target' ($canonicalTarget) is not an authorized hardware flashing target." -ForegroundColor Red
        Write-Host "                Flashing virtual/generic images to physical phone hardware risks permanent bricking." -ForegroundColor Yellow
        Write-Host "                Pass -ForceUnsupported to override if testing in a controlled hardware lab." -ForegroundColor Gray
        exit 1
    }
}

# Verify flashing authorization from target profile
$flashingPermitted = $false
try {
    $allowedOut = python (Join-Path $TOP "build\target_registry.py") is-flashing-allowed $canonicalTarget 2>$null
    if ($LASTEXITCODE -eq 0 -and $allowedOut -match "true") {
        $flashingPermitted = $true
    }
} catch {
}

if (-not $flashingPermitted -and -not $ForceUnsupported) {
    if (-not $DryRun) {
        Write-Host "[SECURITY GATE] Direct physical flashing is disabled for target '$canonicalTarget' (flashing_allowed=false)." -ForegroundColor Red
        Write-Host "                Milestone M5 (device profile, verified recovery, and bootloader trust) is not yet approved." -ForegroundColor Yellow
        Write-Host "                Flashing physical phone hardware before recovery validation risks permanent bricking." -ForegroundColor Yellow
        Write-Host "                Pass -ForceUnsupported only in a controlled hardware bring-up lab." -ForegroundColor Gray
        exit 1
    } else {
        Write-Host "[WARN] Target '$canonicalTarget' has flashing_allowed=false. Preflight inspection only." -ForegroundColor Yellow
    }
}

$bootImg = Join-Path $OUT "boot.img"
$kernelPath = Join-Path $OUT "kernel"
$initrdPath = Join-Path $OUT "initramfs.cpio.gz"
if (-not (Test-Path $bootImg) -and (Test-Path $kernelPath) -and (Test-Path $initrdPath)) {
    Write-Host "==> Packaging Android boot.img using build/mkbootimg.py with profile '$canonicalTarget'..." -ForegroundColor Yellow
    python (Join-Path $TOP "build\mkbootimg.py") create --profile $canonicalTarget --kernel $kernelPath --ramdisk $initrdPath -o $bootImg
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

$vbmetaImg = Join-Path $OUT "vbmeta_a.img"
if (-not (Test-Path $vbmetaImg)) {
    $vbmetaImg = Join-Path $OUT "vbmeta.img"
}

# Fail-closed if vbmeta is missing on hardware target without explicit override
if ($canonicalTarget -eq "oneplus-fajita" -and (-not (Test-Path $vbmetaImg)) -and -not $ForceUnsupported) {
    Write-Host "[SECURITY GATE] Missing verified boot descriptor (vbmeta.img/vbmeta_a.img) for hardware target '$canonicalTarget'!" -ForegroundColor Red
    Write-Host "                Flashing without verified boot metadata on hardware devices risks bootloops/bricking." -ForegroundColor Yellow
    Write-Host "                Pass -ForceUnsupported if testing in a controlled hardware bring-up lab." -ForegroundColor Gray
    exit 1
}

# Calculate image digests
$bootHash = (Get-FileHash -Path $bootImg -Algorithm SHA256).Hash.ToLower()
$sysHash = (Get-FileHash -Path $systemImg -Algorithm SHA256).Hash.ToLower()
$vbmetaHash = "None"
if (Test-Path $vbmetaImg) {
    $vbmetaHash = (Get-FileHash -Path $vbmetaImg -Algorithm SHA256).Hash.ToLower()
}

# Compare image digests with trusted checksums manifest if present
$checksumsFile = Join-Path $OUT "checksums.txt"
if (Test-Path $checksumsFile) {
    Write-Host "==> Validating image hashes against trusted checksums ($checksumsFile)..." -ForegroundColor Yellow
    $checksumLines = Get-Content $checksumsFile
    function Verify-Checksum([string]$filePath, [string]$actualHash) {
        $fileName = Split-Path $filePath -Leaf
        foreach ($line in $checksumLines) {
            if ($line -match "^\s*([0-9a-fA-F]{64})\s+\*?$([regex]::Escape($fileName))\s*$") {
                $expected = $Matches[1].ToLower()
                if ($actualHash -ne $expected) {
                    Write-Host "[ERROR] Image digest mismatch for $fileName!" -ForegroundColor Red
                    Write-Host "        Expected: $expected" -ForegroundColor Red
                    Write-Host "        Actual:   $actualHash" -ForegroundColor Red
                    exit 1
                }
                Write-Host "    [OK] Hash verified: $fileName -> $actualHash" -ForegroundColor Green
                break
            }
        }
    }
    Verify-Checksum $bootImg $bootHash
    Verify-Checksum $systemImg $sysHash
    if (Test-Path $vbmetaImg) {
        Verify-Checksum $vbmetaImg $vbmetaHash
    }
}

# Resolve target A/B slot partition naming
$targetSlot = if ($Slot) { $Slot } else { $deviceSlot }
if ($canonicalTarget -eq "oneplus-fajita" -or $targetSlot) {
    if ($targetSlot -eq "a" -or $targetSlot -eq "b") {
        $bootPart = "boot_$targetSlot"
        $sysPart = "system_$targetSlot"
        $vbmetaPart = "vbmeta_$targetSlot"
    } else {
        $bootPart = "boot"
        $sysPart = "system"
        $vbmetaPart = "vbmeta"
    }
} else {
    $bootPart = "boot"
    $sysPart = "system"
    $vbmetaPart = "vbmeta"
}

Write-Host "==> Target images validated (Preflight Integrity):" -ForegroundColor Green
Write-Host "    Target:     $Target (canonical: $canonicalTarget)"
if ($targetSlot) {
    Write-Host "    Slot:       $targetSlot (partitions: $bootPart, $sysPart, $vbmetaPart)"
}
Write-Host "    Boot image: $bootImg (SHA-256: $bootHash)"
Write-Host "    System:     $systemImg (SHA-256: $sysHash)"
if (Test-Path $vbmetaImg) {
    Write-Host "    vbmeta:     $vbmetaImg (SHA-256: $vbmetaHash)"
} else {
    Write-Host "    vbmeta:     None"
}
if ($WipeUserdata) {
    Write-Host "    Userdata:   WILL BE ERASED (-WipeUserdata passed)" -ForegroundColor Yellow
} else {
    Write-Host "    Userdata:   Preserved (Use -WipeUserdata to erase)" -ForegroundColor Green
}

Write-Host "==> Planned Fastboot Command Sequence:" -ForegroundColor Cyan
Write-Host "    1. fastboot flash $bootPart `"$bootImg`""
Write-Host "    2. fastboot flash $sysPart `"$systemImg`""
if (Test-Path $vbmetaImg) {
    Write-Host "    3. fastboot flash $vbmetaPart `"$vbmetaImg`""
}
if ($WipeUserdata) {
    Write-Host "    4. fastboot format userdata"
}
Write-Host "    5. fastboot reboot"

if ($DryRun) {
    Write-Host "=========================================================" -ForegroundColor Green
    Write-Host " [DRY-RUN COMPLETE] Preflight passed. Zero writes made.  " -ForegroundColor Green
    Write-Host "=========================================================" -ForegroundColor Green
    exit 0
}

$confirm = Read-Host "Are you sure you want to flash OnuronOS to the connected phone? (y/N)"
if ($confirm -ne "y" -and $confirm -ne "Y") {
    Write-Host "Flashing cancelled." -ForegroundColor DarkGray
    exit 0
}

Write-Host "==> Flashing boot partition ($bootPart)..." -ForegroundColor Yellow
fastboot flash $bootPart $bootImg
if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] Failed to flash $bootPart partition" -ForegroundColor Red; exit 1 }

Write-Host "==> Flashing system partition ($sysPart)..." -ForegroundColor Yellow
fastboot flash $sysPart $systemImg
if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] Failed to flash $sysPart partition" -ForegroundColor Red; exit 1 }

if (Test-Path $vbmetaImg) {
    Write-Host "==> Flashing verified boot vbmeta ($vbmetaPart)..." -ForegroundColor Yellow
    fastboot flash $vbmetaPart $vbmetaImg
    if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] Failed to flash $vbmetaPart partition" -ForegroundColor Red; exit 1 }
}

if ($WipeUserdata) {
    Write-Host "==> Formatting userdata (fscrypt ready)..." -ForegroundColor Yellow
    fastboot erase userdata
    if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] Failed to format userdata" -ForegroundColor Red; exit 1 }
}

Write-Host "=========================================================" -ForegroundColor Green
Write-Host "   OnuronOS Flashed Successfully! Rebooting device...   " -ForegroundColor Green
Write-Host "=========================================================" -ForegroundColor Green
fastboot reboot
if ($LASTEXITCODE -ne 0) { Write-Host "[ERROR] Failed to reboot device" -ForegroundColor Red; exit 1 }
