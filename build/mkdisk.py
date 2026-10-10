#!/usr/bin/env python3
"""
build/mkdisk.py — OnuronOS Data Partition Image Builder

Creates a 256 MB raw disk image (nilos.img) that QEMU exposes as /dev/vda.

Two build modes are supported:

* **real** — when Linux ``mke2fs`` is available the image is a genuinely
  mountable ext2/3/4 filesystem (optionally pre-populated from a directory).
  This is what the persistent-data harness (``build/persistent-data-test.py``)
  exercises with ``debugfs``.
* **synthetic** — a pure-Python fallback that writes a minimal ext2 superblock
  and a sparse file. It needs no external tools (so it works on Windows), but it
  is **not** a mountable filesystem: ``nilinit`` is expected to format it on
  first boot. This path is byte-for-byte deterministic.

Use ``auto`` (the default) to prefer the real image when the full e2fsprogs
toolchain (``mke2fs`` + ``debugfs``) is present, and otherwise fall back to the
synthetic image. A *real* image is not yet byte-reproducible (mkfs embeds a
random hash seed and timestamps); see docs/reproducibility.md.
"""

import argparse
import os
import shutil
import struct
import subprocess
import sys

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(TOP, "out", "x86_64-generic")
DISK_PATH = os.path.join(OUT, "nilos.img")

# Disk geometry
DISK_SIZE_MB = 256
BLOCK_SIZE = 4096           # ext2 block size
BLOCKS_PER_GROUP = 8192
INODES_PER_GROUP = 2048

DISK_SIZE = DISK_SIZE_MB * 1024 * 1024
TOTAL_BLOCKS = DISK_SIZE // BLOCK_SIZE

EXT2_MAGIC = 0xEF53
EXT2_FEATURE_COMPAT_EXT_ATTR = 0x0008
EXT2_FEATURE_INCOMPAT_FILETYPE = 0x0002
EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER = 0x0001


def mke2fs_path():
    """Return the path to a mkfs.ext-family tool, or None."""
    for name in ("mke2fs", "mkfs.ext4", "mkfs.ext3", "mkfs.ext2"):
        found = shutil.which(name)
        if found:
            return found
    return None


def debugfs_path():
    """Return the path to debugfs, or None."""
    return shutil.which("debugfs")


def write_ext2_superblock(data: bytearray, offset: int = 1024, size_mb: int = DISK_SIZE_MB):
    """Write a minimal ext2 superblock at the given offset."""
    disk_size = size_mb * 1024 * 1024
    total_blocks = disk_size // BLOCK_SIZE
    blocks_per_group = BLOCKS_PER_GROUP
    inodes_per_group = INODES_PER_GROUP
    total_groups = (total_blocks + blocks_per_group - 1) // blocks_per_group
    total_inodes = total_groups * inodes_per_group

    # Reserved blocks for root
    reserved_blocks = total_blocks // 20

    sb = struct.pack(
        "<IIIIIIIIIIIIIHHHHHHIIIIHHIHHIII",
        total_inodes,                 # s_inodes_count
        total_blocks,                 # s_blocks_count
        reserved_blocks,              # s_r_blocks_count
        total_blocks - 32,            # s_free_blocks_count
        total_inodes - 11,            # s_free_inodes_count
        1,                            # s_first_data_block
        int(BLOCK_SIZE).bit_length()-1-10,  # s_log_block_size
        0,                            # s_log_cluster_size
        blocks_per_group,             # s_blocks_per_group
        blocks_per_group,             # s_clusters_per_group
        inodes_per_group,             # s_inodes_per_group
        0,                            # s_mtime
        0,                            # s_wtime
        0,                            # s_mnt_count
        0xFFFF,                       # s_max_mnt_count
        EXT2_MAGIC,                   # s_magic
        1,                            # s_state (clean)
        1,                            # s_errors (continue)
        0,                            # s_minor_rev_level
        0,                            # s_lastcheck
        0,                            # s_checkinterval
        0,                            # s_creator_os (Linux)
        1,                            # s_rev_level
        0,                            # s_def_resuid
        0,                            # s_def_resgid
        11,                           # s_first_ino
        256,                          # s_inode_size
        0,                            # s_block_group_nr
        EXT2_FEATURE_COMPAT_EXT_ATTR,           # s_feature_compat
        EXT2_FEATURE_INCOMPAT_FILETYPE,          # s_feature_incompat
        EXT2_FEATURE_RO_COMPAT_SPARSE_SUPER,     # s_feature_ro_compat
    )

    # UUID (fake but fixed, so the synthetic image is deterministic)
    uuid = b'\xde\xad\xbe\xef\xca\xfe\xba\xbe\xde\xad\xbe\xef\xca\xfe\xba\xbe'
    # Volume name "nilos-data"
    vol_name = b"nilos-data\x00\x00\x00\x00\x00\x00"

    data[offset:offset + len(sb)] = sb
    data[offset + 104:offset + 120] = uuid
    data[offset + 120:offset + 136] = vol_name


def build_synthetic_image(path, size_mb: int = DISK_SIZE_MB):
    """Write a deterministic synthetic disk image (no external tools).

    The result is a sparse file carrying a minimal ext2 superblock. It is not
    mountable; it exists so the QEMU boot path works without e2fsprogs.
    """
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    disk_size = size_mb * 1024 * 1024

    # Create sparse file
    with open(path, "wb") as f:
        f.seek(disk_size - 1)
        f.write(b'\x00')

    # Write ext2 superblock
    data = bytearray(2048)  # Only need to write first 2 KB for superblock
    write_ext2_superblock(data, offset=1024, size_mb=size_mb)

    with open(path, "r+b") as f:
        f.seek(0)
        f.write(data)
    return path


def build_real_image(path, size_mb: int = DISK_SIZE_MB, populate_dir=None):
    """Build a real, mountable ext2/3/4 image using mke2fs.

    Raises RuntimeError when mke2fs is unavailable. ``populate_dir`` (if given)
    is copied onto the filesystem with ``mke2fs -d``.
    """
    mkfs = mke2fs_path()
    if not mkfs:
        raise RuntimeError("mke2fs is not available; cannot build a real filesystem")

    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    # Create the backing sparse file at the exact requested size.
    with open(path, "wb") as f:
        f.seek(size_mb * 1024 * 1024 - 1)
        f.write(b'\x00')

    command = [mkfs, "-q", "-F", "-t", "ext4", "-L", "nilos-data"]
    if populate_dir:
        command += ["-d", populate_dir]
    command.append(path)
    subprocess.run(command, check=True)
    return path


def check_existing_image_validity(path: str, expected_size_mb: int = DISK_SIZE_MB) -> bool:
    """Validate that existing file has correct size and ext superblock magic (0xEF53)."""
    expected_bytes = expected_size_mb * 1024 * 1024
    if not os.path.exists(path) or os.path.getsize(path) != expected_bytes:
        return False
    try:
        with open(path, "rb") as f:
            # ext2 superblock magic is at offset 1024 + 56
            f.seek(1024 + 56)
            magic_bytes = f.read(2)
            if len(magic_bytes) == 2:
                magic = struct.unpack("<H", magic_bytes)[0]
                return magic == EXT2_MAGIC
    except Exception:
        return False
    return False


def create_disk_image(path: str = DISK_PATH, mode: str = "auto",
                      size_mb: int = DISK_SIZE_MB,
                      populate_dir=None, force: bool = False):
    """Create the data partition image.

    mode:
        "auto"     — real if mke2fs + debugfs exist, else synthetic;
        "real"     — require mke2fs (raises otherwise);
        "synthetic"— always use the pure-Python image.
    """
    expected_bytes = size_mb * 1024 * 1024
    if not force and os.path.exists(path):
        if check_existing_image_validity(path, size_mb):
            print(f"[OK] Valid ext filesystem image already exists: {path} ({size_mb} MB)")
            return path
        elif os.path.getsize(path) != expected_bytes:
            print(f"[WARN] Existing disk image {path} size mismatch (expected {size_mb} MB). Recreating with force.")
        else:
            print(f"[WARN] Existing disk image {path} lacks valid ext superblock magic. Recreating with force.")

    use_real = mode == "real" or (
        mode == "auto" and mke2fs_path() is not None and debugfs_path() is not None
    )

    if use_real:
        print(f"==> Creating real ext4 data partition: {path} ({size_mb} MB)...")
        build_real_image(path, size_mb=size_mb, populate_dir=populate_dir)
        if populate_dir:
            print(f"[OK] Real ext4 image created and populated from {populate_dir}")
        else:
            print("[OK] Real ext4 image created (mountable; nilinit can mount /data directly)")
    else:
        if mode == "real":
            raise RuntimeError("mode=real requested but mke2fs/debugfs are unavailable")
        print(f"==> Creating synthetic data partition: {path} ({size_mb} MB)...")
        if mode == "auto":
            print("[WARN] mke2fs + debugfs not both found; writing a synthetic (non-mountable) image.")
            print("       Install Linux e2fsprogs for a real, mountable filesystem.")
        build_synthetic_image(path, size_mb=size_mb)
        print("[OK] Synthetic disk image created (nilinit formats + mounts it at /data on first boot)")

    size = os.path.getsize(path)
    print(f"[OK] Disk image ready: {path} ({size // (1024 * 1024)} MB)")
    return path


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default=DISK_PATH, help="output image path")
    parser.add_argument("--size-mb", type=int, default=DISK_SIZE_MB, help="image size in MiB")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--real", dest="mode", action="store_const", const="real",
                      help="require mke2fs and build a mountable filesystem")
    mode.add_argument("--synthetic", dest="mode", action="store_const", const="synthetic",
                      help="force the pure-Python non-mountable image")
    parser.set_defaults(mode="auto")
    parser.add_argument("--populate", default=None,
                        help="directory whose contents are copied into the real image")
    parser.add_argument("--force", action="store_true", help="rebuild even if the image exists")
    args = parser.parse_args(argv)

    print("=========================================================")
    print("         OnuronOS Data Partition Image Builder           ")
    print("=========================================================")
    create_disk_image(args.output, mode=args.mode, size_mb=args.size_mb,
                      populate_dir=args.populate, force=args.force)
    return 0


if __name__ == "__main__":
    sys.exit(main())