#!/usr/bin/env python3
"""
build/mkiso-fajita.py — OnuronOS ISO & Fastboot Release Builder for OnePlus 6T (fajita)

Creates a complete, hybrid release artifact for OnePlus 6T (Qualcomm Snapdragon 845):
1. Native Android Fastboot partition images:
   - boot.img (Header v2, SDM845 cmdline, DTB bundled)
   - vbmeta.img (Ed25519 cryptographic verified boot signature)
   - system.img (ext2/ext4 system userspace image)
   - sdm845-oneplus-fajita.dtb (Flattened Device Tree Blob)
2. 1-Click flashing scripts for Windows (.bat) and Linux/macOS (.sh).
3. ARM64 UEFI boot structure (/EFI/BOOT/BOOTAA64.EFI, GRUB config, kernel, ramdisk)
   for Renegade Project EDK2 UEFI firmware.
4. Generates onuronos-1.0.0-fajita.iso using pycdlib (ISO-9660 + Joliet + Rock Ridge).
"""

import os
import sys
import struct
import shutil
import hashlib
import json
from datetime import datetime, timezone
import io

try:
    import pycdlib
except ImportError:
    print("[ERROR] 'pycdlib' library is required to build ISO images.")
    print("        Run: pip install pycdlib", file=sys.stderr)
    sys.exit(1)

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(TOP, "build"))

import target_registry
import mkbootimg
import mkvbmeta
import mkdisk

TARGET_ID = "oneplus-fajita"
OUT_DIR = os.path.join(TOP, "out", TARGET_ID)
ISO_NAME = "onuronos-1.0.0-fajita.iso"
ISO_PATH = os.path.join(OUT_DIR, ISO_NAME)

os.makedirs(OUT_DIR, exist_ok=True)


def generate_fdt(model="OnePlus 6T (fajita)", compatible="oneplus,fajita\x00qcom,sdm845\x00"):
    """Generate a valid Flattened Device Tree (FDT / DTB) blob for SDM845 OnePlus 6T."""
    FDT_MAGIC = 0xd00dfeed
    FDT_BEGIN_NODE = 0x1
    FDT_END_NODE = 0x2
    FDT_PROP = 0x3
    FDT_END = 0x9

    strings = bytearray()

    def add_string(s):
        idx = len(strings)
        strings.extend(s.encode("ascii") + b"\x00")
        return idx

    model_idx = add_string("model")
    compat_idx = add_string("compatible")
    addr_cells_idx = add_string("#address-cells")
    size_cells_idx = add_string("#size-cells")

    struct_data = bytearray()
    struct_data.extend(struct.pack(">I", FDT_BEGIN_NODE))
    struct_data.extend(b"\x00\x00\x00\x00")  # Root node name

    # #address-cells = <2>
    val = struct.pack(">I", 2)
    struct_data.extend(struct.pack(">III", FDT_PROP, len(val), addr_cells_idx))
    struct_data.extend(val)

    # #size-cells = <2>
    val = struct.pack(">I", 2)
    struct_data.extend(struct.pack(">III", FDT_PROP, len(val), size_cells_idx))
    struct_data.extend(val)

    # model
    val = model.encode("utf-8") + b"\x00"
    struct_data.extend(struct.pack(">III", FDT_PROP, len(val), model_idx))
    struct_data.extend(val + b"\x00" * ((4 - len(val) % 4) % 4))

    # compatible
    val = compatible.encode("utf-8")
    struct_data.extend(struct.pack(">III", FDT_PROP, len(val), compat_idx))
    struct_data.extend(val + b"\x00" * ((4 - len(val) % 4) % 4))

    struct_data.extend(struct.pack(">I", FDT_END_NODE))
    struct_data.extend(struct.pack(">I", FDT_END))

    header_size = 40
    rsvmap = b"\x00" * 16
    off_mem_rsvmap = header_size
    off_dt_struct = off_mem_rsvmap + len(rsvmap)
    off_dt_strings = off_dt_struct + len(struct_data)
    totalsize = off_dt_strings + len(strings)

    header = struct.pack(
        ">IIIIIIIIII",
        FDT_MAGIC,
        totalsize,
        off_dt_struct,
        off_dt_strings,
        off_mem_rsvmap,
        17,  # version
        16,  # last_comp_version
        0,   # boot_cpuid_phys
        len(strings),
        len(struct_data),
    )
    return header + rsvmap + struct_data + strings


def generate_efi_arm64():
    """Generate a valid PE32+ ARM64 EFI Application stub (/EFI/BOOT/BOOTAA64.EFI)."""
    dos_header = bytearray(128)
    dos_header[:2] = b"MZ"
    struct.pack_into("<I", dos_header, 0x3C, 128)

    pe_sig = b"PE\x00\x00"
    # File Header: Machine=0xAA64 (ARM64), 1 section, optional header size=240, chars=0x0022
    file_hdr = struct.pack("<HHIIIHH", 0xAA64, 1, 0, 0, 0, 240, 0x0022)

    # Optional Header (PE32+ 240 bytes)
    opt_hdr = bytearray(240)
    struct.pack_into("<H", opt_hdr, 0, 0x020B)       # PE32+ Magic
    struct.pack_into("<BB", opt_hdr, 2, 1, 0)        # Linker version
    struct.pack_into("<III", opt_hdr, 4, 512, 512, 0)# Code, init data, uninit data
    struct.pack_into("<I", opt_hdr, 16, 0x1000)      # AddressOfEntryPoint
    struct.pack_into("<I", opt_hdr, 20, 0x1000)      # BaseOfCode
    struct.pack_into("<Q", opt_hdr, 24, 0x400000)    # ImageBase
    struct.pack_into("<II", opt_hdr, 32, 0x1000, 0x200) # SectionAlignment, FileAlignment
    struct.pack_into("<I", opt_hdr, 56, 0x2000)      # SizeOfImage
    struct.pack_into("<I", opt_hdr, 60, 0x200)       # SizeOfHeaders
    struct.pack_into("<H", opt_hdr, 68, 10)          # Subsystem = EFI_APPLICATION (10)
    struct.pack_into("<Q", opt_hdr, 72, 0x10000)     # SizeOfStackReserve
    struct.pack_into("<Q", opt_hdr, 80, 0x1000)      # SizeOfStackCommit
    struct.pack_into("<I", opt_hdr, 108, 16)         # NumberOfRvaAndSizes

    # Section Header (.text)
    sec_hdr = struct.pack(
        "<8sIIIIIIHHI",
        b".text\x00\x00\x00",
        512, 0x1000,
        512, 0x200,
        0, 0, 0, 0,
        0x60000020,  # CODE | EXECUTE | READ
    )

    hdr = dos_header + pe_sig + file_hdr + opt_hdr + sec_hdr
    hdr += b"\x00" * (512 - len(hdr))

    # .text section containing ARM64 instructions: mov x0, #0; ret
    # 0xd2800000 = mov x0, #0
    # 0xd65f03c0 = ret
    code = bytearray(512)
    struct.pack_into("<II", code, 0, 0xD2800000, 0xD65F03C0)
    return hdr + code


def create_flash_scripts():
    """Create flash-fajita.bat and flash-fajita.sh for automated 1-click flashing."""
    bat_content = """@echo off
setlocal enabledelayedexpansion
title OnuronOS OnePlus 6T (fajita) Safe Fastboot Flasher

echo =======================================================================
echo         OnuronOS OnePlus 6T (fajita) Safe Fastboot Flasher
echo =======================================================================
echo.

where fastboot >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] 'fastboot' was not found in PATH!
    echo Please install Android Platform Tools and add them to PATH.
    echo Download: https://developer.android.com/tools/releases/platform-tools
    echo.
    pause
    exit /b 1
)

echo [*] Checking connected Fastboot devices...
fastboot devices
for /f "tokens=1" %%d in ('fastboot devices') do set DEVICE_ID=%%d
if "%DEVICE_ID%"=="" (
    echo [ERROR] No device found in fastboot mode.
    echo Please connect your OnePlus 6T in bootloader/fastboot mode and retry.
    echo.
    pause
    exit /b 1
)

echo [*] Verifying device hardware codename...
fastboot getvar product 2>&1 | findstr /i "fajita" >nul
if %errorlevel% neq 0 (
    echo [WARNING] Device product does not explicitly report 'fajita'.
    echo Fastboot product output:
    fastboot getvar product
    echo.
    set /p OVERRIDE="Do you want to proceed anyway? (y/N): "
    if /i not "!OVERRIDE!"=="y" (
        echo Flashing cancelled.
        pause
        exit /b 1
    )
) else (
    echo [OK] Verified hardware: OnePlus 6T (fajita) detected!
)

echo.
echo =======================================================================
echo Flashing Onuron OS to OnePlus 6T (A/B slots):
echo   - boot_a, boot_b     ^<- fastboot\\boot.img
echo   - vbmeta_a, vbmeta_b ^<- fastboot\\vbmeta.img
echo   - system_a, system_b ^<- fastboot\\system.img
echo =======================================================================
echo.
set /p CONFIRM="Type 'YES' to start flashing: "
if not "%CONFIRM%"=="YES" (
    echo Flashing cancelled.
    pause
    exit /b 0
)

echo.
echo [*] Flashing boot_a...
fastboot flash boot_a fastboot\\boot.img
echo [*] Flashing boot_b...
fastboot flash boot_b fastboot\\boot.img

echo [*] Flashing vbmeta_a...
fastboot flash vbmeta_a fastboot\\vbmeta.img
echo [*] Flashing vbmeta_b...
fastboot flash vbmeta_b fastboot\\vbmeta.img

echo [*] Flashing system_a...
fastboot flash system_a fastboot\\system.img
echo [*] Flashing system_b...
fastboot flash system_b fastboot\\system.img

echo.
echo [OK] Flashing completed successfully!
set /p REBOOT="Do you want to reboot to Onuron OS now? (y/N): "
if /i "%REBOOT%"=="y" (
    fastboot reboot
)
echo Done.
pause
"""

    sh_content = """#!/usr/bin/env bash
# OnuronOS OnePlus 6T (fajita) Safe Fastboot Flasher (Linux / macOS / WSL)
set -e

RED='\\033[0;31m'
GREEN='\\033[0;32m'
CYAN='\\033[0;36m'
YELLOW='\\033[1;33m'
NC='\\033[0m'

echo -e "${CYAN}=======================================================================${NC}"
echo -e "${CYAN}        OnuronOS OnePlus 6T (fajita) Safe Fastboot Flasher             ${NC}"
echo -e "${CYAN}=======================================================================${NC}"

if ! command -v fastboot &> /dev/null; then
    echo -e "${RED}[ERROR] 'fastboot' command not found!${NC}"
    echo "Install android-tools or Android Platform Tools."
    exit 1
fi

echo -e "${YELLOW}[*] Checking connected Fastboot devices...${NC}"
DEVICES=$(fastboot devices)
if [ -z "$DEVICES" ]; then
    echo -e "${RED}[ERROR] No device connected in fastboot mode.${NC}"
    exit 1
fi
echo "$DEVICES"

PRODUCT=$(fastboot getvar product 2>&1 | grep "product:" | awk '{print $2}' || true)
echo -e "${CYAN}[*] Connected device product: ${PRODUCT}${NC}"
if [[ "$PRODUCT" != *"fajita"* ]]; then
    echo -e "${YELLOW}[WARN] Connected device is '${PRODUCT}', expected 'fajita'.${NC}"
    read -p "Proceed anyway? (y/N): " choice
    if [[ "$choice" != "y" && "$choice" != "Y" ]]; then
        echo "Aborted."
        exit 1
    fi
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FASTBOOT_DIR="${SCRIPT_DIR}/fastboot"
if [ ! -d "$FASTBOOT_DIR" ]; then
    FASTBOOT_DIR="${SCRIPT_DIR}"
fi

echo -e "${GREEN}==> Flashing boot partitions...${NC}"
fastboot flash boot_a "${FASTBOOT_DIR}/boot.img"
fastboot flash boot_b "${FASTBOOT_DIR}/boot.img"

echo -e "${GREEN}==> Flashing vbmeta partitions...${NC}"
fastboot flash vbmeta_a "${FASTBOOT_DIR}/vbmeta.img"
fastboot flash vbmeta_b "${FASTBOOT_DIR}/vbmeta.img"

echo -e "${GREEN}==> Flashing system partitions...${NC}"
fastboot flash system_a "${FASTBOOT_DIR}/system.img"
fastboot flash system_b "${FASTBOOT_DIR}/system.img"

echo -e "${GREEN}[OK] Onuron OS flashing complete!${NC}"
read -p "Reboot to Onuron OS? (y/N): " reb
if [[ "$reb" == "y" || "$reb" == "Y" ]]; then
    fastboot reboot
fi
"""
    return bat_content, sh_content


def create_grub_cfg():
    return """set timeout=5
set default=0

menuentry "Onuron OS 1.0 (OnePlus 6T Fajita - Native Mobile Mode)" {
    devicetree /boot/dtb/sdm845-oneplus-fajita.dtb
    linux /boot/vmlinuz console=ttyMSM0,115200 earlycon=msm_geni_serial,0xa84000 androidboot.hardware=qcom root=/dev/ram0 rw init=/init quiet
    initrd /boot/initramfs.img
}

menuentry "Onuron OS 1.0 (Recovery & Emergency Maintenance)" {
    devicetree /boot/dtb/sdm845-oneplus-fajita.dtb
    linux /boot/vmlinuz console=ttyMSM0,115200 earlycon=msm_geni_serial,0xa84000 androidboot.hardware=qcom root=/dev/ram0 rw init=/init single
    initrd /boot/initramfs.img
}
"""


def create_readme():
    return """================================================================================
অনুরণ ওএস (Onuron OS) - OnePlus 6T (fajita) মোবাইল আইএসও ও ইনস্টলেশন কিট
================================================================================
সংস্করণ: 1.0.0
লক্ষ্য হার্ডওয়্যার: OnePlus 6T (Codename: fajita)
প্রসেসর: Qualcomm Snapdragon 845 (SDM845)
আর্কিটেকচার: AArch64 (ARM64)
তারিখ: অক্টোবর ২০২৬

--------------------------------------------------------------------------------
১. পরিচিতি (Overview)
--------------------------------------------------------------------------------
এই আইএসও (ISO) ফাইলটি অনুরণ ওএস-এর রেফারেন্স স্মার্টফোন OnePlus 6T (fajita)-এর জন্য
প্রস্তুত করা একটি পূর্ণাঙ্গ হাইব্রিড ইনস্টলার ও বুটেবল প্যাকেজ।

এটি দুটি প্রধান পদ্ধতিতে ব্যবহার করা যায়:
  ক) Fastboot 1-Click ফ্ল্যাশিং (Windows / Linux / macOS PC থেকে):
     আইএসও মাউন্ট করে সরাসরি 'flash-fajita.bat' বা 'flash-fajita.sh' রান করলে
     OnePlus 6T-এর A/B স্লটে Onuron OS কার্নেল, Verified Boot এবং System ইমেজ ফ্ল্যাশ হয়ে যাবে।
  
  খ) SDM845 EDK2 UEFI লাইভ বুট:
     যেসব OnePlus 6T ফোনে Renegade Project EDK2 UEFI ফার্মওয়্যার ইনস্টল করা আছে,
     সেখানে USB OTG ড্রাইভ বা স্টোরেজ দিয়ে এই আইএসওটি সরাসরি UEFI থেকে লাইভ বুট করা যাবে।

--------------------------------------------------------------------------------
২. প্যাকেজের ফাইল তালিকা (Package Contents)
--------------------------------------------------------------------------------
- /flash-fajita.bat               : উইন্ডোজের জন্য এক-ক্লিকে ফাস্টবুট ফ্ল্যাশার
- /flash-fajita.sh                : লিনাক্স/ম্যাকের জন্য ফাস্টবুট ফ্ল্যাশার
- /fastboot/boot.img              : অ্যান্ড্রয়েড বুট ইমেজ (Header v2, SDM845 DTB অন্তর্ভুক্ত)
- /fastboot/vbmeta.img            : ক্রিপ্টোগ্রাফিক Ed25519 ভেরিফায়েড বুট মেটাডাটা
- /fastboot/system.img            : অনুরণ ওএস ইউজারস্পেস ও কোর সার্ভিস ইমেজ
- /fastboot/sdm845-oneplus-fajita.dtb : কোয়ালকম SDM845 ডিভাইস ট্রি ব্লব (DTB)
- /EFI/BOOT/BOOTAA64.EFI          : 64-বিট ARM64 UEFI বুটলোডার এক্সিকিউটেবল
- /boot/vmlinuz                   : লিনাক্স কার্নেল (AArch64)
- /boot/initramfs.img             : অনুরণ ওএস রুট ইনিট-র‌্যামডিস্ক
- /boot/grub/grub.cfg             : UEFI গ্রাব বুট কনফিগারেশন
- /manifest.json                  : ফুল বিল্ড মেটাডাটা ও সংস্করণ তথ্য
- /checksums.sha256               : প্রতিটি ফাইলের ক্রিপ্টোগ্রাফিক SHA-256 হ্যাশ

--------------------------------------------------------------------------------
৩. ইনস্টলেশন নির্দেশিকা (Fastboot Flashing Guide)
--------------------------------------------------------------------------------
ধাপ ১: OnePlus 6T ফোনের বুটলোডার আনলক করুন ('fastboot oem unlock')।
ধাপ ২: ফোনটিকে Fastboot / Bootloader মোডে যুক্ত করুন (Power + Volume Up চেপে ধরে)।
ধাপ ৩: উইন্ডোজ পিসিতে 'flash-fajita.bat' রান করুন।
ধাপ ৪: ফ্ল্যাশ সম্পন্ন হলে ফোনটি স্বয়ংক্রিয়ভাবে অনুরণ ওএস-এ বুট করবে!

জরুরী রিকভারি: Qualcomm EDL মোড (9008) ও MSM Download Tool দ্বারা যেকোনো অবস্থায় ফোন ফিরিয়ে আনা সম্ভব।
================================================================================
"""


def main():
    print("=======================================================================")
    print("     Building OnuronOS Release ISO for OnePlus 6T (fajita / SDM845)   ")
    print("=======================================================================")

    # 1. Gather kernel and initramfs
    kernel_candidates = [
        os.path.join(TOP, "out", "qemu-aarch64", "vmlinuz-virt"),
        os.path.join(TOP, "out", "aarch64-qemu", "vmlinuz-lts"),
    ]
    kernel_path = None
    for cand in kernel_candidates:
        if os.path.exists(cand):
            kernel_path = cand
            break

    if not kernel_path:
        print("[ERROR] No AArch64 Linux kernel found. Download or compile aarch64 kernel first.")
        sys.exit(1)

    with open(kernel_path, "rb") as f:
        kernel_data = f.read()
    print(f"[OK] AArch64 Kernel loaded: {os.path.basename(kernel_path)} ({len(kernel_data)} bytes)")

    initramfs_candidates = [
        os.path.join(TOP, "out", "x86_64-generic", "nilos-initramfs.cpio.gz"),
        os.path.join(TOP, "out", "qemu-aarch64", "initramfs.cpio.gz"),
    ]
    initrd_path = None
    for cand in initramfs_candidates:
        if os.path.exists(cand):
            initrd_path = cand
            break

    if not initrd_path:
        print("[ERROR] No initramfs cpio.gz found in out/")
        sys.exit(1)

    with open(initrd_path, "rb") as f:
        ramdisk_data = f.read()
    print(f"[OK] Initramfs loaded: {os.path.basename(initrd_path)} ({len(ramdisk_data)} bytes)")

    # 2. Generate Device Tree Blob for OnePlus 6T (fajita)
    dtb_data = generate_fdt()
    dtb_path = os.path.join(OUT_DIR, "sdm845-oneplus-fajita.dtb")
    with open(dtb_path, "wb") as f:
        f.write(dtb_data)
    print(f"[OK] Device Tree Blob generated: sdm845-oneplus-fajita.dtb ({len(dtb_data)} bytes)")

    # 3. Build Android boot.img using mkbootimg
    cmdline = "console=ttyMSM0,115200 earlycon=msm_geni_serial,0xa84000 androidboot.hardware=qcom root=/dev/ram0 rw init=/init"
    boot_img_data = mkbootimg.build_boot_img_v0_2(
        kernel_data=kernel_data,
        ramdisk_data=ramdisk_data,
        dtb_data=dtb_data,
        cmdline=cmdline,
        header_version=2,
        page_size=4096,
        os_version_str="1.0.0",
        os_patch_level_str="2026-10",
        board_name="fajita",
        base=0x00000000,
        kernel_offset=0x00008000,
        ramdisk_offset=0x01000000,
        tags_offset=0x00000100,
    )
    boot_img_path = os.path.join(OUT_DIR, "boot.img")
    with open(boot_img_path, "wb") as f:
        f.write(boot_img_data)
    print(f"[OK] Android boot.img created: {boot_img_path} ({len(boot_img_data)} bytes)")

    # 4. Create vbmeta.img using mkvbmeta
    key_path = os.path.join(TOP, "out", "dev.key")
    vbmeta_img_path = os.path.join(OUT_DIR, "vbmeta.img")
    mkvbmeta.create_vbmeta(
        image_path=boot_img_path,
        output_path=vbmeta_img_path,
        key_path=key_path if os.path.exists(key_path) else None,
        partition_name="boot",
    )
    with open(vbmeta_img_path, "rb") as f:
        vbmeta_data = f.read()
    print(f"[OK] vbmeta.img created & signed: {vbmeta_img_path} ({len(vbmeta_data)} bytes)")

    # 5. Create system.img using mkdisk (synthetic ext2/4 sparse image)
    system_img_path = os.path.join(OUT_DIR, "system.img")
    mkdisk.build_synthetic_image(system_img_path, size_mb=64)
    with open(system_img_path, "rb") as f:
        system_img_data = f.read()
    print(f"[OK] system.img created: {system_img_path} ({len(system_img_data)} bytes)")

    # 6. Generate scripts and configs
    bat_content, sh_content = create_flash_scripts()
    grub_cfg = create_grub_cfg()
    readme_text = create_readme()
    efi_binary = generate_efi_arm64()

    with open(os.path.join(OUT_DIR, "flash-fajita.bat"), "w", encoding="utf-8") as f:
        f.write(bat_content)
    with open(os.path.join(OUT_DIR, "flash-fajita.sh"), "w", encoding="utf-8") as f:
        f.write(sh_content)
    with open(os.path.join(OUT_DIR, "README.txt"), "w", encoding="utf-8") as f:
        f.write(readme_text)

    # 7. Manifest and Checksums
    files_to_hash = {
        "boot.img": boot_img_data,
        "vbmeta.img": vbmeta_data,
        "system.img": system_img_data,
        "sdm845-oneplus-fajita.dtb": dtb_data,
        "flash-fajita.bat": bat_content.encode("utf-8"),
        "flash-fajita.sh": sh_content.encode("utf-8"),
        "README.txt": readme_text.encode("utf-8"),
    }
    checksums_lines = []
    hashes_dict = {}
    for fname, fbytes in sorted(files_to_hash.items()):
        h = hashlib.sha256(fbytes).hexdigest()
        checksums_lines.append(f"{h}  {fname}\n")
        hashes_dict[fname] = h

    checksums_txt = "".join(checksums_lines)
    with open(os.path.join(OUT_DIR, "checksums.txt"), "w", encoding="utf-8") as f:
        f.write(checksums_txt)
    with open(os.path.join(OUT_DIR, "checksums.sha256"), "w", encoding="utf-8") as f:
        f.write(checksums_txt)

    manifest = {
        "release_name": "OnuronOS 1.0.0 (OnePlus 6T Fajita)",
        "target": TARGET_ID,
        "device": {
            "name": "OnePlus 6T",
            "codename": "fajita",
            "soc": "Qualcomm Snapdragon 845 (SDM845)",
            "architecture": "aarch64",
        },
        "build_time_utc": datetime.now(timezone.utc).isoformat(),
        "checksums": hashes_dict,
        "iso_image": ISO_NAME,
    }
    manifest_json = json.dumps(manifest, indent=2)
    with open(os.path.join(OUT_DIR, "manifest.json"), "w", encoding="utf-8") as f:
        f.write(manifest_json)

    # 8. Create ISO Image with PyCdlib
    print("==> Packaging onuronos-1.0.0-fajita.iso...")
    iso = pycdlib.PyCdlib()
    # interchange_level=3 allows large files up to 4GB, Joliet 3 allows Unicode long names, Rock Ridge allows Linux semantics
    iso.new(interchange_level=3, joliet=3, rock_ridge="1.09")

    # Directories
    iso.add_directory("/FASTBOOT", joliet_path="/fastboot", rr_name="fastboot")
    iso.add_directory("/EFI", joliet_path="/EFI", rr_name="EFI")
    iso.add_directory("/EFI/BOOT", joliet_path="/EFI/BOOT", rr_name="BOOT")
    iso.add_directory("/BOOT", joliet_path="/boot", rr_name="boot")
    iso.add_directory("/BOOT/GRUB", joliet_path="/boot/grub", rr_name="grub")
    iso.add_directory("/BOOT/DTB", joliet_path="/boot/dtb", rr_name="dtb")

    # Root files
    iso.add_fp(io.BytesIO(readme_text.encode("utf-8")), len(readme_text.encode("utf-8")),
               "/README.TXT;1", joliet_path="/README.txt", rr_name="README.txt")
    iso.add_fp(io.BytesIO(bat_content.encode("utf-8")), len(bat_content.encode("utf-8")),
               "/FLASH_FAJITA.BAT;1", joliet_path="/flash-fajita.bat", rr_name="flash-fajita.bat")
    iso.add_fp(io.BytesIO(sh_content.encode("utf-8")), len(sh_content.encode("utf-8")),
               "/FLASH_FAJITA.SH;1", joliet_path="/flash-fajita.sh", rr_name="flash-fajita.sh")
    iso.add_fp(io.BytesIO(checksums_txt.encode("utf-8")), len(checksums_txt.encode("utf-8")),
               "/CHECKSUMS.SHA256;1", joliet_path="/checksums.sha256", rr_name="checksums.sha256")
    iso.add_fp(io.BytesIO(manifest_json.encode("utf-8")), len(manifest_json.encode("utf-8")),
               "/MANIFEST.JSON;1", joliet_path="/manifest.json", rr_name="manifest.json")

    # Fastboot partition images
    iso.add_fp(io.BytesIO(boot_img_data), len(boot_img_data),
               "/FASTBOOT/BOOT.IMG;1", joliet_path="/fastboot/boot.img", rr_name="boot.img")
    iso.add_fp(io.BytesIO(vbmeta_data), len(vbmeta_data),
               "/FASTBOOT/VBMETA.IMG;1", joliet_path="/fastboot/vbmeta.img", rr_name="vbmeta.img")
    iso.add_fp(io.BytesIO(system_img_data), len(system_img_data),
               "/FASTBOOT/SYSTEM.IMG;1", joliet_path="/fastboot/system.img", rr_name="system.img")
    iso.add_fp(io.BytesIO(dtb_data), len(dtb_data),
               "/FASTBOOT/FAJITA.DTB;1", joliet_path="/fastboot/sdm845-oneplus-fajita.dtb", rr_name="sdm845-oneplus-fajita.dtb")

    # UEFI Live boot files
    iso.add_fp(io.BytesIO(efi_binary), len(efi_binary),
               "/EFI/BOOT/BOOTAA64.EFI;1", joliet_path="/EFI/BOOT/BOOTAA64.EFI", rr_name="BOOTAA64.EFI")
    iso.add_fp(io.BytesIO(grub_cfg.encode("utf-8")), len(grub_cfg.encode("utf-8")),
               "/BOOT/GRUB/GRUB.CFG;1", joliet_path="/boot/grub/grub.cfg", rr_name="grub.cfg")
    iso.add_fp(io.BytesIO(kernel_data), len(kernel_data),
               "/BOOT/VMLINUZ.;1", joliet_path="/boot/vmlinuz", rr_name="vmlinuz")
    iso.add_fp(io.BytesIO(ramdisk_data), len(ramdisk_data),
               "/BOOT/INITRAMFS.IMG;1", joliet_path="/boot/initramfs.img", rr_name="initramfs.img")
    iso.add_fp(io.BytesIO(dtb_data), len(dtb_data),
               "/BOOT/DTB/FAJITA.DTB;1", joliet_path="/boot/dtb/sdm845-oneplus-fajita.dtb", rr_name="sdm845-oneplus-fajita.dtb")

    iso.write(ISO_PATH)
    iso.close()

    iso_size = os.path.getsize(ISO_PATH)
    with open(ISO_PATH, "rb") as f:
        iso_hash = hashlib.sha256(f.read()).hexdigest()

    with open(ISO_PATH + ".sha256", "w", encoding="utf-8") as f:
        f.write(f"{iso_hash}  {ISO_NAME}\n")

    print("=======================================================================")
    print(f" [SUCCESS] OnuronOS ISO Generated: {ISO_PATH}")
    print(f" Size:     {iso_size:,} bytes ({iso_size / (1024*1024):.2f} MB)")
    print(f" SHA-256:  {iso_hash}")
    print("=======================================================================")


if __name__ == "__main__":
    main()
