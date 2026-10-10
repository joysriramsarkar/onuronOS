#!/usr/bin/env python3
"""
build/mkbootimg.py — OnuronOS Android Boot Image Packaging Tool

Creates and unpacks Android-compliant boot images (boot.img) for flashing to
ARM64 mobile reference targets and fastboot devices.

Supports:
- Header versions 0, 1, 2, 3, and 4
- Kernel, initramfs/ramdisk, and Device Tree Blob (DTB) bundling
- Kernel command line specification
- Byte-for-byte deterministic builds (zeroed timestamps / padding)
- Inspection and unpacking of existing boot.img files
"""

import argparse
import hashlib
import os
import struct
from typing import Dict, Any, Tuple

BOOT_MAGIC = b"ANDROID!"
BOOT_MAGIC_SIZE = 8
BOOT_NAME_SIZE = 16
BOOT_ARGS_SIZE = 512
BOOT_EXTRA_ARGS_SIZE = 1024
BOOT_ARGS_SIZE_V3 = 1536

DEFAULT_PAGE_SIZE = 2048
DEFAULT_BASE = 0x80000000
DEFAULT_KERNEL_OFFSET = 0x00080000
DEFAULT_RAMDISK_OFFSET = 0x01000000
DEFAULT_SECOND_OFFSET = 0x00F00000
DEFAULT_TAGS_OFFSET = 0x00000100


def pad_size(size: int, page_size: int) -> int:
    """Calculate padded size to page_size boundary."""
    if page_size <= 0:
        return size
    rem = size % page_size
    return size if rem == 0 else size + (page_size - rem)


def get_padding(size: int, page_size: int) -> bytes:
    """Return null bytes needed to pad size to page_size."""
    padded = pad_size(size, page_size)
    return b"\x00" * (padded - size)


def encode_os_version(version_str: str, patch_level_str: str) -> int:
    """
    Encode OS version and patch level into 32-bit integer.
    Format:
      a.b.c -> (a << 14) | (b << 7) | c  (21 bits)
      yyyy-mm -> ((yyyy - 2000) << 4) | mm (11 bits)
    """
    os_ver = 0
    if version_str:
        parts = [int(p) for p in version_str.split(".") if p.isdigit()]
        a = parts[0] if len(parts) > 0 else 0
        b = parts[1] if len(parts) > 1 else 0
        c = parts[2] if len(parts) > 2 else 0
        os_ver = ((a & 0x7F) << 14) | ((b & 0x7F) << 7) | (c & 0x7F)

    patch_lvl = 0
    if patch_level_str:
        p_parts = [int(p) for p in patch_level_str.split("-") if p.isdigit()]
        if len(p_parts) >= 2:
            year = max(0, p_parts[0] - 2000)
            month = p_parts[1]
            patch_lvl = ((year & 0x7F) << 4) | (month & 0x0F)

    return ((os_ver & 0x1FFFFF) << 11) | (patch_lvl & 0x7FF)


def decode_os_version(val: int) -> Tuple[str, str]:
    """Decode 32-bit integer into OS version and patch level strings."""
    os_ver = (val >> 11) & 0x1FFFFF
    patch_lvl = val & 0x7FF

    a = (os_ver >> 14) & 0x7F
    b = (os_ver >> 7) & 0x7F
    c = os_ver & 0x7F
    version_str = f"{a}.{b}.{c}"

    year = ((patch_lvl >> 4) & 0x7F) + 2000
    month = patch_lvl & 0x0F
    patch_str = f"{year:04d}-{month:02d}"

    return version_str, patch_str


def build_boot_img_v0_2(
    kernel_data: bytes,
    ramdisk_data: bytes,
    second_data: bytes = b"",
    dtb_data: bytes = b"",
    recovery_dtbo_data: bytes = b"",
    cmdline: str = "",
    base: int = DEFAULT_BASE,
    kernel_offset: int = DEFAULT_KERNEL_OFFSET,
    ramdisk_offset: int = DEFAULT_RAMDISK_OFFSET,
    second_offset: int = DEFAULT_SECOND_OFFSET,
    tags_offset: int = DEFAULT_TAGS_OFFSET,
    page_size: int = DEFAULT_PAGE_SIZE,
    header_version: int = 2,
    os_version_str: str = "14.0.0",
    os_patch_level_str: str = "2026-10",
    board_name: str = "onuron-arm64",
) -> bytes:
    """Build Android boot image format v0, v1, or v2."""
    kernel_size = len(kernel_data)
    ramdisk_size = len(ramdisk_data)
    second_size = len(second_data)
    dtb_size = len(dtb_data)
    recovery_dtbo_size = len(recovery_dtbo_data)

    kernel_addr = base + kernel_offset
    ramdisk_addr = base + ramdisk_offset
    second_addr = base + second_offset
    tags_addr = base + tags_offset
    dtb_addr = base + 0x01F00000

    os_version = encode_os_version(os_version_str, os_patch_level_str)

    board_bytes = board_name.encode("utf-8")[:BOOT_NAME_SIZE].ljust(BOOT_NAME_SIZE, b"\x00")

    cmdline_bytes = cmdline.encode("utf-8")
    main_cmdline = cmdline_bytes[:BOOT_ARGS_SIZE].ljust(BOOT_ARGS_SIZE, b"\x00")
    extra_cmdline = cmdline_bytes[BOOT_ARGS_SIZE:BOOT_ARGS_SIZE + BOOT_EXTRA_ARGS_SIZE].ljust(
        BOOT_EXTRA_ARGS_SIZE, b"\x00"
    )

    # Compute image SHA-1 digest for ID field
    sha = hashlib.sha1()
    sha.update(kernel_data)
    sha.update(struct.pack("<I", kernel_size))
    sha.update(ramdisk_data)
    sha.update(struct.pack("<I", ramdisk_size))
    sha.update(second_data)
    sha.update(struct.pack("<I", second_size))
    if header_version >= 1:
        sha.update(recovery_dtbo_data)
        sha.update(struct.pack("<I", recovery_dtbo_size))
    if header_version >= 2:
        sha.update(dtb_data)
        sha.update(struct.pack("<I", dtb_size))
    img_id = sha.digest().ljust(32, b"\x00")

    # Pack base v0 header (1632 bytes)
    header = bytearray(
        struct.pack(
            "<8s10I16s512s32s1024s",
            BOOT_MAGIC,
            kernel_size,
            kernel_addr,
            ramdisk_size,
            ramdisk_addr,
            second_size,
            second_addr,
            tags_addr,
            page_size,
            header_version,
            os_version,
            board_bytes,
            main_cmdline,
            img_id,
            extra_cmdline,
        )
    )

    recovery_dtbo_offset = 0
    if header_version >= 1:
        header_size = 1632 + 16  # v1 header size (1648)
        if header_version >= 2:
            header_size += 12   # v2 header size (1660)

        # offset calculation:
        curr_offset = pad_size(header_size, page_size)
        curr_offset += pad_size(kernel_size, page_size)
        curr_offset += pad_size(ramdisk_size, page_size)
        curr_offset += pad_size(second_size, page_size)
        recovery_dtbo_offset = curr_offset if recovery_dtbo_size > 0 else 0

        header.extend(
            struct.pack(
                "<IQI",
                recovery_dtbo_size,
                recovery_dtbo_offset,
                header_size,
            )
        )

    if header_version >= 2:
        header.extend(
            struct.pack(
                "<IQ",
                dtb_size,
                dtb_addr,
            )
        )

    out = bytearray()
    out.extend(header)
    out.extend(get_padding(len(header), page_size))

    out.extend(kernel_data)
    out.extend(get_padding(kernel_size, page_size))

    out.extend(ramdisk_data)
    out.extend(get_padding(ramdisk_size, page_size))

    if second_size > 0:
        out.extend(second_data)
        out.extend(get_padding(second_size, page_size))

    if header_version >= 1 and recovery_dtbo_size > 0:
        out.extend(recovery_dtbo_data)
        out.extend(get_padding(recovery_dtbo_size, page_size))

    if header_version >= 2 and dtb_size > 0:
        out.extend(dtb_data)
        out.extend(get_padding(dtb_size, page_size))

    return bytes(out)


def build_boot_img_v3_4(
    kernel_data: bytes,
    ramdisk_data: bytes,
    cmdline: str = "",
    header_version: int = 3,
    os_version_str: str = "14.0.0",
    os_patch_level_str: str = "2026-10",
    signature_data: bytes = b"",
) -> bytes:
    """Build Android boot image format v3 or v4 (fixed 4096-byte pages)."""
    page_size = 4096
    kernel_size = len(kernel_data)
    ramdisk_size = len(ramdisk_data)
    os_version = encode_os_version(os_version_str, os_patch_level_str)

    cmdline_bytes = cmdline.encode("utf-8")[:BOOT_ARGS_SIZE_V3].ljust(BOOT_ARGS_SIZE_V3, b"\x00")
    header_size = 4096

    header = bytearray(
        struct.pack(
            "<8s4I16sI1536s",
            BOOT_MAGIC,
            kernel_size,
            ramdisk_size,
            os_version,
            header_size,
            b"\x00" * 16,  # reserved
            header_version,
            cmdline_bytes,
        )
    )

    if header_version >= 4:
        header.extend(struct.pack("<I", len(signature_data)))

    out = bytearray()
    out.extend(header)
    out.extend(get_padding(len(header), page_size))

    out.extend(kernel_data)
    out.extend(get_padding(kernel_size, page_size))

    out.extend(ramdisk_data)
    out.extend(get_padding(ramdisk_size, page_size))

    if header_version >= 4 and len(signature_data) > 0:
        out.extend(signature_data)
        out.extend(get_padding(len(signature_data), page_size))

    return bytes(out)


def parse_boot_img(data: bytes) -> Dict[str, Any]:
    """Parse an existing Android boot image and return header and component offsets."""
    if len(data) < BOOT_MAGIC_SIZE or data[:BOOT_MAGIC_SIZE] != BOOT_MAGIC:
        raise ValueError("Invalid boot.img magic: not an Android boot image")

    # Peek version at offset 40
    ver_cand = struct.unpack_from("<I", data, 40)[0]
    is_v3_or_higher = (ver_cand in (3, 4))

    if is_v3_or_higher:
        # Header v3/v4: <8s 4I 16s I 1536s
        magic, kernel_size, ramdisk_size, os_ver_raw, header_sz, _, header_version, cmdline_b = (
            struct.unpack_from("<8s4I16sI1536s", data, 0)
        )
        page_size = 4096
        cmdline = cmdline_b.split(b"\x00", 1)[0].decode("utf-8", errors="replace")
        os_ver, patch_lvl = decode_os_version(os_ver_raw)

        curr = page_size
        k_data = data[curr:curr + kernel_size]
        curr += pad_size(kernel_size, page_size)
        rd_data = data[curr:curr + ramdisk_size]

        return {
            "header_version": header_version,
            "page_size": page_size,
            "kernel_size": kernel_size,
            "ramdisk_size": ramdisk_size,
            "os_version": os_ver,
            "os_patch_level": patch_lvl,
            "cmdline": cmdline,
            "kernel_data": k_data,
            "ramdisk_data": rd_data,
            "second_data": b"",
            "dtb_data": b"",
        }

    # Version 0/1/2
    (
        magic,
        kernel_size,
        kernel_addr,
        ramdisk_size,
        ramdisk_addr,
        second_size,
        second_addr,
        tags_addr,
        page_size,
        header_version,
        os_ver_raw,
        board_name_b,
        cmdline_b,
        img_id,
        extra_cmdline_b,
    ) = struct.unpack_from("<8s10I16s512s32s1024s", data, 0)

    board_name = board_name_b.split(b"\x00", 1)[0].decode("utf-8", errors="replace")
    cmd1 = cmdline_b.split(b"\x00", 1)[0].decode("utf-8", errors="replace")
    cmd2 = extra_cmdline_b.split(b"\x00", 1)[0].decode("utf-8", errors="replace")
    full_cmdline = f"{cmd1} {cmd2}".strip()
    os_ver, patch_lvl = decode_os_version(os_ver_raw)

    recovery_dtbo_size = 0
    dtb_size = 0

    if header_version >= 1 and len(data) >= 1648:
        recovery_dtbo_size, _, _ = struct.unpack_from("<IQI", data, 1632)
    if header_version >= 2 and len(data) >= 1660:
        dtb_size, _ = struct.unpack_from("<IQ", data, 1648)

    curr = pad_size(1632 + (16 if header_version >= 1 else 0) + (12 if header_version >= 2 else 0), page_size)

    k_data = data[curr:curr + kernel_size]
    curr += pad_size(kernel_size, page_size)

    rd_data = data[curr:curr + ramdisk_size]
    curr += pad_size(ramdisk_size, page_size)

    second_data = b""
    if second_size > 0:
        second_data = data[curr:curr + second_size]
        curr += pad_size(second_size, page_size)

    if header_version >= 1 and recovery_dtbo_size > 0:
        curr += pad_size(recovery_dtbo_size, page_size)

    dtb_data = b""
    if header_version >= 2 and dtb_size > 0:
        dtb_data = data[curr:curr + dtb_size]

    return {
        "header_version": header_version,
        "page_size": page_size,
        "kernel_size": kernel_size,
        "kernel_addr": kernel_addr,
        "ramdisk_size": ramdisk_size,
        "ramdisk_addr": ramdisk_addr,
        "second_size": second_size,
        "dtb_size": dtb_size,
        "board_name": board_name,
        "os_version": os_ver,
        "os_patch_level": patch_lvl,
        "cmdline": full_cmdline,
        "kernel_data": k_data,
        "ramdisk_data": rd_data,
        "second_data": second_data,
        "dtb_data": dtb_data,
    }


DEVICE_PROFILES = {
    "fajita": {
        "board": "fajita",
        "header_version": 2,
        "pagesize": 4096,
        "base": 0x00000000,
        "cmdline": "console=ttyMSM0,115200,n8 androidboot.hardware=qcom root=/dev/ram0 rw init=/init",
    },
    "enchilada": {
        "board": "enchilada",
        "header_version": 2,
        "pagesize": 4096,
        "base": 0x00000000,
        "cmdline": "console=ttyMSM0,115200,n8 androidboot.hardware=qcom root=/dev/ram0 rw init=/init",
    },
    "generic-arm64": {
        "board": "onuron-arm64",
        "header_version": 2,
        "pagesize": 2048,
        "base": DEFAULT_BASE,
        "cmdline": "console=ttyMSM0,115200 root=/dev/ram0 rw init=/init",
    },
}


def main():
    parser = argparse.ArgumentParser(description="OnuronOS Android Boot Image Utility")
    subparsers = parser.add_subparsers(dest="command", required=True)

    create_p = subparsers.add_parser("create", help="Create a boot.img")
    create_p.add_argument("--profile", choices=list(DEVICE_PROFILES.keys()), help="Target device hardware profile (e.g. fajita, enchilada, generic-arm64)")
    create_p.add_argument("--kernel", required=True, help="Path to kernel zImage/Image.gz")
    create_p.add_argument("--ramdisk", required=True, help="Path to ramdisk/initramfs cpio.gz")
    create_p.add_argument("--dtb", help="Path to device tree blob (.dtb)")
    create_p.add_argument("--cmdline", default="console=ttyMSM0,115200 root=/dev/ram0 rw init=/init", help="Kernel cmdline")
    create_p.add_argument("--base", type=lambda x: int(x, 0), default=DEFAULT_BASE, help="Base memory address (hex/dec)")
    create_p.add_argument("--pagesize", type=int, default=DEFAULT_PAGE_SIZE, help="Page size (e.g. 2048, 4096)")
    create_p.add_argument("--header-version", type=int, default=2, choices=[0, 1, 2, 3, 4], help="Boot header version")
    create_p.add_argument("--os-version", default="14.0.0", help="OS version string (e.g. 14.0.0)")
    create_p.add_argument("--os-patch-level", default="2026-10", help="Security patch level (YYYY-MM)")
    create_p.add_argument("--board", default="onuron-arm64", help="Board product name")
    create_p.add_argument("-o", "--output", required=True, help="Output boot.img path")

    unpack_p = subparsers.add_parser("unpack", help="Unpack boot.img components")
    unpack_p.add_argument("image", help="Path to boot.img")
    unpack_p.add_argument("-o", "--out-dir", default=".", help="Output directory")

    info_p = subparsers.add_parser("info", help="Display boot.img header metadata")
    info_p.add_argument("image", help="Path to boot.img")

    args = parser.parse_args()

    if args.command == "create":
        if args.profile:
            prof = DEVICE_PROFILES[args.profile]
            if args.board == "onuron-arm64":
                args.board = prof["board"]
            if args.pagesize == DEFAULT_PAGE_SIZE:
                args.pagesize = prof["pagesize"]
            if args.base == DEFAULT_BASE:
                args.base = prof["base"]
            if args.cmdline == "console=ttyMSM0,115200 root=/dev/ram0 rw init=/init":
                args.cmdline = prof["cmdline"]
            if args.header_version == 2:
                args.header_version = prof["header_version"]
        with open(args.kernel, "rb") as f:
            k_bytes = f.read()
        with open(args.ramdisk, "rb") as f:
            rd_bytes = f.read()
        dtb_bytes = b""
        if args.dtb and os.path.exists(args.dtb):
            with open(args.dtb, "rb") as f:
                dtb_bytes = f.read()

        if args.header_version in (3, 4):
            img_bytes = build_boot_img_v3_4(
                kernel_data=k_bytes,
                ramdisk_data=rd_bytes,
                cmdline=args.cmdline,
                header_version=args.header_version,
                os_version_str=args.os_version,
                os_patch_level_str=args.os_patch_level,
            )
        else:
            img_bytes = build_boot_img_v0_2(
                kernel_data=k_bytes,
                ramdisk_data=rd_bytes,
                dtb_data=dtb_bytes,
                cmdline=args.cmdline,
                base=args.base,
                page_size=args.pagesize,
                header_version=args.header_version,
                os_version_str=args.os_version,
                os_patch_level_str=args.os_patch_level,
                board_name=args.board,
            )

        os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
        with open(args.output, "wb") as f:
            f.write(img_bytes)
        print(f"[OK] Boot image generated: {args.output} ({len(img_bytes)} bytes, v{args.header_version})")

    elif args.command == "info":
        with open(args.image, "rb") as f:
            info = parse_boot_img(f.read())
        print("Android Boot Image Metadata:")
        print(f"  Header Version: {info.get('header_version')}")
        print(f"  Page Size:      {info.get('page_size')}")
        print(f"  Kernel Size:    {info.get('kernel_size')} bytes")
        print(f"  Ramdisk Size:   {info.get('ramdisk_size')} bytes")
        if "dtb_size" in info:
            print(f"  DTB Size:       {info.get('dtb_size')} bytes")
        print(f"  OS Version:     {info.get('os_version')}")
        print(f"  Patch Level:    {info.get('os_patch_level')}")
        print(f"  Cmdline:        {info.get('cmdline')}")

    elif args.command == "unpack":
        os.makedirs(args.out_dir, exist_ok=True)
        with open(args.image, "rb") as f:
            info = parse_boot_img(f.read())
        k_path = os.path.join(args.out_dir, "kernel")
        rd_path = os.path.join(args.out_dir, "ramdisk.cpio.gz")
        with open(k_path, "wb") as f:
            f.write(info["kernel_data"])
        with open(rd_path, "wb") as f:
            f.write(info["ramdisk_data"])
        if info.get("dtb_data"):
            dtb_path = os.path.join(args.out_dir, "dtb.img")
            with open(dtb_path, "wb") as f:
                f.write(info["dtb_data"])
        print(f"[OK] Unpacked to {args.out_dir}")


if __name__ == "__main__":
    main()
