#!/usr/bin/env bash
# build/flash-device.sh — Safe Fastboot Flash Tool for OnuronOS Targets
set -euo pipefail

RAW_TARGET="${1:-oneplus-fajita}"
TOP="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$(python3 "$TOP/build/target_registry.py" resolve "$RAW_TARGET" 2>/dev/null || echo "$RAW_TARGET")"
OUT="$(python3 "$TOP/build/target_registry.py" output-dir "$TARGET" 2>/dev/null || echo "$TOP/out/$TARGET")"
WIPE_USERDATA=0
FORCE_UNSUPPORTED=0
DRY_RUN=0
SLOT_OVERRIDE=""

shift || true
while [[ $# -gt 0 ]]; do
  case "$1" in
    --wipe-userdata) WIPE_USERDATA=1; shift ;;
    --force-unsupported) FORCE_UNSUPPORTED=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --slot) SLOT_OVERRIDE="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

echo "========================================================="
echo "       OnuronOS Safe Fastboot Flasher (Hardware Gate)    "
echo "========================================================="

if ! command -v fastboot >/dev/null 2>&1; then
  echo "[ERROR] 'fastboot' tool not found in PATH. Install Android Platform Tools." >&2
  exit 1
fi

echo "==> Checking connected Fastboot devices..."
DEVICES=$(fastboot devices)
if [ -z "$DEVICES" ]; then
  echo "[ERROR] No device connected in fastboot mode. Connect phone in fastboot/bootloader mode." >&2
  exit 1
fi
echo "$DEVICES"

# Check for multiple devices to prevent ambiguous writes
DEVICE_COUNT=$(echo "$DEVICES" | grep -v '^[[:space:]]*$' | wc -l | tr -d ' ')
if [ "$DEVICE_COUNT" -gt 1 ]; then
  echo "[ERROR] Multiple fastboot devices detected ($DEVICE_COUNT devices)." >&2
  echo "        Disconnect other devices to avoid ambiguous flashing." >&2
  exit 1
fi

# Read device product identifier
DEVICE_PRODUCT=$(fastboot getvar product 2>&1 | grep "product:" | awk '{print $2}' || echo "unknown")
echo "[INFO] Connected device product identifier: $DEVICE_PRODUCT"

if [ -z "$DEVICE_PRODUCT" ] || [ "$DEVICE_PRODUCT" = "unknown" ]; then
  if [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
    echo "[ERROR] Connected device returned unknown or empty product identifier ('$DEVICE_PRODUCT')." >&2
    echo "        Refusing to flash unidentified device. Pass --force-unsupported to override." >&2
    exit 1
  fi
fi

# Read device active boot slot (A/B device architecture)
DEVICE_SLOT=$(fastboot getvar current-slot 2>&1 | grep "current-slot:" | awk '{print $2}' || echo "")
if [ -n "$DEVICE_SLOT" ]; then
  echo "[INFO] Connected device current boot slot: $DEVICE_SLOT"
fi

# Device matching gate
if command -v python3 >/dev/null 2>&1 && [ -f "$TOP/build/target_registry.py" ]; then
  CANONICAL_TARGET="$(python3 "$TOP/build/target_registry.py" resolve "$TARGET" 2>/dev/null || echo "$TARGET")"
else
  case "$TARGET" in
    fajita|oneplus-fajita) CANONICAL_TARGET="oneplus-fajita" ;;
    *) CANONICAL_TARGET="$TARGET" ;;
  esac
fi

# Resolve canonical output directory
if [ -d "$TOP/out/$CANONICAL_TARGET" ]; then
  OUT="$TOP/out/$CANONICAL_TARGET"
fi

if [ "$CANONICAL_TARGET" = "oneplus-fajita" ]; then
  if [ "$DEVICE_PRODUCT" != "fajita" ] && [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
    echo "[ERROR] Device product '$DEVICE_PRODUCT' does not match target 'fajita'!" >&2
    echo "        Refusing to flash incompatible boot/system images to prevent hardware bricking." >&2
    echo "        Pass --force-unsupported if you are an expert and intentional." >&2
    exit 1
  fi
else
  if [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
    echo "[SECURITY GATE] Target '$TARGET' ($CANONICAL_TARGET) is not an authorized hardware flashing target." >&2
    echo "                Flashing virtual/generic images to physical phone hardware risks permanent bricking." >&2
    echo "                Pass --force-unsupported if testing in a controlled hardware bring-up lab." >&2
    exit 1
  fi
fi

# Verify flashing authorization from target profile
FLASHING_ALLOWED=0
if command -v python3 >/dev/null 2>&1 && [ -f "$TOP/build/target_registry.py" ]; then
  if python3 "$TOP/build/target_registry.py" is-flashing-allowed "$CANONICAL_TARGET" >/dev/null 2>&1; then
    FLASHING_ALLOWED=1
  fi
fi

if [ "$FLASHING_ALLOWED" -eq 0 ] && [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
  if [ "$DRY_RUN" -eq 0 ]; then
    echo "[SECURITY GATE] Direct physical flashing is disabled for target '$CANONICAL_TARGET' (flashing_allowed=false)." >&2
    echo "                Milestone M5 (device profile, verified recovery, and bootloader trust) is not yet approved." >&2
    echo "                Flashing physical phone hardware before recovery validation risks permanent bricking." >&2
    echo "                Pass --force-unsupported if testing in a controlled hardware bring-up lab." >&2
    exit 1
  else
    echo "[WARN] Target '$CANONICAL_TARGET' has flashing_allowed=false. Preflight inspection only."
  fi
fi

# Verify image existence
if [ ! -f "$OUT/boot.img" ] && [ -f "$OUT/kernel" ] && [ -f "$OUT/initramfs.cpio.gz" ]; then
  echo "==> Packaging Android boot.img using build/mkbootimg.py with profile '$CANONICAL_TARGET'..."
  python3 "$TOP/build/mkbootimg.py" create --profile "$CANONICAL_TARGET" --kernel "$OUT/kernel" --ramdisk "$OUT/initramfs.cpio.gz" -o "$OUT/boot.img"
fi

if [ ! -f "$OUT/boot.img" ]; then
  echo "[ERROR] Boot image not found at $OUT/boot.img" >&2
  exit 1
fi

SYS_IMG=""
if [ -f "$OUT/system_a.img" ]; then
  SYS_IMG="$OUT/system_a.img"
elif [ -f "$OUT/system.img" ]; then
  SYS_IMG="$OUT/system.img"
else
  echo "[ERROR] Valid system image not found in $OUT" >&2
  exit 1
fi

VBMETA_IMG=""
if [ -f "$OUT/vbmeta_a.img" ]; then
  VBMETA_IMG="$OUT/vbmeta_a.img"
elif [ -f "$OUT/vbmeta.img" ]; then
  VBMETA_IMG="$OUT/vbmeta.img"
fi

# Fail-closed if vbmeta is missing on hardware targets without explicit override
if [ "$CANONICAL_TARGET" = "oneplus-fajita" ] && [ -z "$VBMETA_IMG" ] && [ "$FORCE_UNSUPPORTED" -eq 0 ]; then
  echo "[SECURITY GATE] Missing verified boot descriptor (vbmeta.img/vbmeta_a.img) for hardware target '$CANONICAL_TARGET'!" >&2
  echo "                Flashing without verified boot metadata on hardware devices risks bootloops/bricking." >&2
  echo "                Pass --force-unsupported if testing in a controlled bring-up lab." >&2
  exit 1
fi

# Calculate digests and sizes for preflight integrity inspection
get_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    python3 -c "import hashlib; print(hashlib.sha256(open('$1', 'rb').read()).hexdigest())"
  fi
}

BOOT_SHA="$(get_sha256 "$OUT/boot.img")"
SYS_SHA="$(get_sha256 "$SYS_IMG")"
VBMETA_SHA=""
if [ -n "$VBMETA_IMG" ]; then
  VBMETA_SHA="$(get_sha256 "$VBMETA_IMG")"
fi

# Compare image digests with trusted checksums manifest if present
CHECKSUMS_FILE="$OUT/checksums.txt"
if [ -f "$CHECKSUMS_FILE" ]; then
  echo "==> Validating image hashes against trusted checksums ($CHECKSUMS_FILE)..."
  verify_file_checksum() {
    local target_file="$1"
    local file_name
    file_name="$(basename "$target_file")"
    local expected
    expected="$(grep -E "[[:space:]]\*?${file_name}$" "$CHECKSUMS_FILE" 2>/dev/null | awk '{print $1}' || echo "")"
    if [ -n "$expected" ]; then
      local actual
      actual="$(get_sha256 "$target_file")"
      if [ "$actual" != "$expected" ]; then
        echo "[ERROR] Image digest mismatch for $file_name!" >&2
        echo "        Expected: $expected" >&2
        echo "        Actual:   $actual" >&2
        exit 1
      fi
      echo "    [OK] Hash verified: $file_name -> $actual"
    fi
  }
  verify_file_checksum "$OUT/boot.img"
  verify_file_checksum "$SYS_IMG"
  if [ -n "$VBMETA_IMG" ]; then
    verify_file_checksum "$VBMETA_IMG"
  fi
fi

# Resolve target A/B slot partition naming
TARGET_SLOT="${SLOT_OVERRIDE:-$DEVICE_SLOT}"
if [ "$CANONICAL_TARGET" = "oneplus-fajita" ] || [ -n "$TARGET_SLOT" ]; then
  if [ "$TARGET_SLOT" = "a" ] || [ "$TARGET_SLOT" = "b" ]; then
    BOOT_PART="boot_${TARGET_SLOT}"
    SYS_PART="system_${TARGET_SLOT}"
    VBMETA_PART="vbmeta_${TARGET_SLOT}"
  else
    BOOT_PART="boot"
    SYS_PART="system"
    VBMETA_PART="vbmeta"
  fi
else
  BOOT_PART="boot"
  SYS_PART="system"
  VBMETA_PART="vbmeta"
fi

echo "==> Target images validated (Preflight Integrity):"
echo "    Target:     $TARGET (canonical: $CANONICAL_TARGET)"
if [ -n "$TARGET_SLOT" ]; then
  echo "    Slot:       $TARGET_SLOT (partitions: $BOOT_PART, $SYS_PART, $VBMETA_PART)"
fi
echo "    Boot image: $OUT/boot.img (SHA-256: $BOOT_SHA)"
echo "    System:     $SYS_IMG (SHA-256: $SYS_SHA)"
if [ -n "$VBMETA_IMG" ]; then
  echo "    vbmeta:     $VBMETA_IMG (SHA-256: $VBMETA_SHA)"
else
  echo "    vbmeta:     None"
fi
if [ "$WIPE_USERDATA" -eq 1 ]; then
  echo "    Userdata:   WILL BE ERASED (--wipe-userdata active)"
else
  echo "    Userdata:   Preserved (Pass --wipe-userdata to reformat)"
fi

echo "==> Planned Fastboot Command Sequence:"
echo "    1. fastboot flash $BOOT_PART \"$OUT/boot.img\""
echo "    2. fastboot flash $SYS_PART \"$SYS_IMG\""
if [ -n "$VBMETA_IMG" ]; then
  echo "    3. fastboot flash $VBMETA_PART \"$VBMETA_IMG\""
fi
if [ "$WIPE_USERDATA" -eq 1 ]; then
  echo "    4. fastboot format userdata"
fi
echo "    5. fastboot reboot"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "========================================================="
  echo " [DRY-RUN COMPLETE] Preflight passed. Zero writes made.  "
  echo "========================================================="
  exit 0
fi

echo "==> Safety Confirmation:"
read -p "Are you sure you want to proceed with flashing? (y/N): " CONFIRM
if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
  echo "Flashing aborted."
  exit 0
fi

echo "==> Flashing boot partition ($BOOT_PART)..."
fastboot flash "$BOOT_PART" "$OUT/boot.img" || { echo "[ERROR] Failed to flash $BOOT_PART partition" >&2; exit 1; }

echo "==> Flashing system partition ($SYS_PART)..."
fastboot flash "$SYS_PART" "$SYS_IMG" || { echo "[ERROR] Failed to flash $SYS_PART partition" >&2; exit 1; }

if [ -n "$VBMETA_IMG" ]; then
  echo "==> Flashing verified boot vbmeta ($VBMETA_PART)..."
  fastboot flash "$VBMETA_PART" "$VBMETA_IMG" || { echo "[ERROR] Failed to flash $VBMETA_PART partition" >&2; exit 1; }
fi

if [ "$WIPE_USERDATA" -eq 1 ]; then
  echo "==> Formatting userdata partition..."
  (fastboot format userdata || fastboot erase userdata) || { echo "[ERROR] Failed to wipe userdata" >&2; exit 1; }
fi

echo "========================================================="
echo "   OnuronOS Flashed Successfully! Rebooting device...   "
echo "========================================================="
fastboot reboot || { echo "[ERROR] Failed to reboot device" >&2; exit 1; }
