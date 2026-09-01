#!/usr/bin/env python3
"""
build/mkdisk.py — NilOS Data Partition Image Builder
Creates a 256 MB sparse raw disk image (nilos.img) that QEMU mounts as /dev/vda.
The image is pre-formatted with a minimal ext2 superblock so Linux can mount it.
Works entirely without external tools — pure Python struct-based image creation.
"""

import os
import struct
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


def write_ext2_superblock(data: bytearray, offset: int = 1024):
    """Write a minimal ext2 superblock at the given offset."""
    total_blocks = DISK_SIZE // BLOCK_SIZE
    blocks_per_group = BLOCKS_PER_GROUP
    inodes_per_group = INODES_PER_GROUP
    total_groups = (total_blocks + blocks_per_group - 1) // blocks_per_group
    total_inodes = total_groups * inodes_per_group

    # Reserved blocks for root
    reserved_blocks = total_blocks // 20

    sb = struct.pack(
        "<IIIIIIIIIIIHHHHHHHIIIBBHIIIBBHIII",
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

    # UUID (fake but unique-ish)
    uuid = b'\xde\xad\xbe\xef\xca\xfe\xba\xbe\xde\xad\xbe\xef\xca\xfe\xba\xbe'
    # Volume name "nilos-data"
    vol_name = b"nilos-data\x00\x00\x00\x00\x00\x00"

    data[offset:offset + len(sb)] = sb
    data[offset + 104:offset + 120] = uuid
    data[offset + 120:offset + 136] = vol_name


def create_disk_image():
    os.makedirs(OUT, exist_ok=True)

    if os.path.exists(DISK_PATH):
        size = os.path.getsize(DISK_PATH)
        if size == DISK_SIZE:
            print(f"[OK] Disk image already exists: {DISK_PATH} ({DISK_SIZE_MB} MB)")
            return
        else:
            print(f"[!] Existing disk image has wrong size ({size} bytes), recreating...")

    print(f"==> Creating NilOS data partition: {DISK_PATH} ({DISK_SIZE_MB} MB)...")

    # Create sparse file
    with open(DISK_PATH, "wb") as f:
        f.seek(DISK_SIZE - 1)
        f.write(b'\x00')

    # Write ext2 superblock
    data = bytearray(2048)  # Only need to write first 2 KB for superblock
    write_ext2_superblock(data, offset=1024)

    with open(DISK_PATH, "r+b") as f:
        f.seek(0)
        f.write(data)

    size = os.path.getsize(DISK_PATH)
    print(f"[OK] Disk image created: {DISK_PATH} ({size // (1024*1024)} MB)")
    print(f"     QEMU will detect this as an ext2 volume 'nilos-data'.")
    print(f"     nilinit will format + mount it at /data on first boot.")


if __name__ == "__main__":
    print("=========================================================")
    print("         NilOS Data Partition Image Builder              ")
    print("=========================================================")
    create_disk_image()
