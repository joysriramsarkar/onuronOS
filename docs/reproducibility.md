# Image Reproducibility

> **Status:** Partial. The initramfs and the synthetic data image are
> byte-for-byte reproducible today. The Linux kernel and a `mke2fs`-built
> `nilos.img` are **not** yet reproducible. This document lists exactly what is
> and is not guaranteed so nobody has to guess.

## What "reproducible" means here

Two builds of the same commit, on the same host/toolchain, produce byte-identical
output artifacts. The gate is a SHA-256 comparison of two consecutive builds.

## Now deterministic

| Artifact | How | Check |
|---|---|---|
| `nilos-initramfs.cpio.gz` | SVR4 newc cpio with fixed metadata (uid/gid `0`, fixed `mtime=1700000000`), sorted traversal, and gzip with `mtime=0` and no embedded filename | `python3 build/mkinitramfs.py --check-reproducible` or `python3 build/check-reproducible.py` |
| synthetic `nilos.img` (pure-Python fallback) | fixed superblock fields, fixed filesystem UUID, sparse layout, no timestamps | `python3 build/check-reproducible.py` |

The initramfs determinism fix lives in `build/mkinitramfs.py`
(`gzip_compress`/`build_cpio`/`check_reproducible`). Previously the archive used
`gzip.compress(...)`, which stamps the current wall-clock time into the gzip
header, so every build differed.

## Still nondeterministic (documented honestly)

1. **The Linux kernel (`vmlinuz-lts`).** `build/mkinitramfs.py` downloads it from
   `KERNEL_URL` in `ensure_kernel()`. The URL is not pinned to a release digest
   and the bytes are fetched over the network, so:
   - a network failure silently leaves a stale/absent kernel (it only warns);
   - a republished upstream artifact would change the payload without warning.

   To make the kernel reproducible, vendor it (or fetch by immutable digest and
   verify a pinned SHA-256) and store the expected hash in the repo. Not done
   yet, so kernel bytes are **not** covered by the reproducibility gate.

2. **A real ext4 `nilos.img` built by `mke2fs`.** Used by
   `build/persistent-data-test.py` and by `build/mkdisk.py` when the e2fsprogs
   toolchain is present. `mke2fs` embeds a random filesystem UUID, a random
   directory hash seed, and the current time (`s_mkfs_time`/`s_wtime`), so two
   runs differ. The reproducibility gate only checks the *synthetic* disk image,
   which is deterministic. Making real images reproducible requires pinning the
   UUID/hash seed and normalising on-disk timestamps, and is future work.

3. **Toolchain / host metadata.** Rust binaries copied into the initramfs are
   read as opaque bytes, so the initramfs is deterministic *given the same
   binaries*. Building the Rust workspace twice is not guaranteed to be
   bit-identical (paths, compiler build ids), which is why the improvement plan
   calls for pinning the toolchain.

## Running the checks

```
# initramfs only (builds twice in memory, no QEMU/network)
python3 build/mkinitramfs.py --check-reproducible [--rootfs DIR]

# initramfs + deterministic synthetic disk image
python3 build/check-reproducible.py [--rootfs DIR] [--skip-disk] [--disk-size-mb N]

# unit tests for the determinism helpers (small temp trees)
python3 -m unittest build/test_reproducible.py
```

## Persistent-data (A3) status

`build/mkdisk.py` produces a **real, mountable** ext2/3/4 image when
`mke2fs` is available, and falls back to the deterministic synthetic image
otherwise. `build/persistent-data-test.py` builds a real image from a known
staging tree and reads the files back with `debugfs`, verifying `/data`
state (OOBE flag, PIN record, contacts, SMS threads) is present and
byte-identical. On platforms without Linux `mke2fs`/`debugfs` (e.g. Windows)
the harness prints a `[SKIP]` message and exits 0.

It does **not** validate that `nilinit` mounts `/data`, that a reboot keeps the
data, or that the shell reads it — those require a QEMU boot and remain open.