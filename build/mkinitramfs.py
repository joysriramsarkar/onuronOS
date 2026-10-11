#!/usr/bin/env python3
"""
build/mkinitramfs.py — NilOS Standalone Initramfs & Image Generator
Creates a Linux-compatible SVR4 (070701) cpio.gz initramfs for QEMU x86_64 boot.
Runs completely cross-platform without requiring external 'cpio' or 'mknod'.
"""

import os
import sys
import gzip
import hashlib
import io
import shutil
import struct
import subprocess
import tempfile
import urllib.request
import argparse
import json

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

try:
    import target_registry
except ImportError:
    try:
        from . import target_registry
    except (ImportError, ValueError):
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import target_registry


def _load_arch_config(target_name, default_initrd):
    canonical = target_registry.resolve_target(target_name, warn=False)
    cfg = target_registry.load_target(canonical, warn=False)
    out_dir = target_registry.get_target_output_dir(canonical)
    kernel_sec = cfg.get("kernel", {})
    target_sec = cfg.get("target", {})
    return {
        "canonical_target": canonical,
        "out_dir": out_dir,
        "kernel_name": kernel_sec.get("name", "vmlinuz-lts"),
        "kernel_url": kernel_sec.get("url", ""),
        "kernel_sha256": kernel_sec.get("sha256", ""),
        "target_triple": target_sec.get("target_triple", ""),
        "initrd_name": default_initrd,
    }


ARCH_CONFIGS = {
    "x86_64": _load_arch_config("qemu-x86_64", "nilos-initramfs.cpio.gz"),
    "aarch64": _load_arch_config("qemu-aarch64", "initramfs.cpio.gz"),
}

# Default authoritative canonical globals for tests
OUT = ARCH_CONFIGS["x86_64"]["out_dir"]
ROOTFS = os.path.join(OUT, "rootfs")
KERNEL_PATH = os.path.join(OUT, ARCH_CONFIGS["x86_64"]["kernel_name"])
INITRD_PATH = os.path.join(OUT, "nilos-initramfs.cpio.gz")
KERNEL_URL = ARCH_CONFIGS["x86_64"]["kernel_url"]

STORAGE_MODULE_NAMES = [
    "virtio_blk.ko",
    "crc32c_generic.ko",
    "libcrc32c.ko",
    "crc16.ko",
    "mbcache.ko",
    "jbd2.ko",
    "ext4.ko",
]


def provision_storage_modules(arch, rootfs_dir, out_dir):
    """Ensure kernel storage modules (virtio_blk and ext4 stack) are present in rootfs."""
    norm_arch = "aarch64" if arch in ("aarch64", "arm64") else "x86_64"
    dest_dir = os.path.join(rootfs_dir, "lib", "modules", "storage")
    os.makedirs(dest_dir, exist_ok=True)

    missing = [m for m in STORAGE_MODULE_NAMES if not os.path.exists(os.path.join(dest_dir, m))]
    if not missing:
        return

    modloop_path = os.path.join(out_dir, "modloop-virt")
    if not os.path.exists(modloop_path) or os.path.getsize(modloop_path) < 1000000:
        url = f"https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/{norm_arch}/netboot/modloop-virt"
        print(f"==> Fetching Alpine kernel storage modules ({norm_arch}) from:\n    {url}")
        try:
            urllib.request.urlretrieve(url, modloop_path)
        except Exception as exc:
            print(f"[WARN] Failed downloading modloop-virt ({exc}); checking existing modules...")
            return

    try:
        unsquashfs_bin = shutil.which("unsquashfs")
        if unsquashfs_bin:
            with tempfile.TemporaryDirectory() as tmp_sq:
                subprocess.run(
                    [unsquashfs_bin, "-f", "-d", tmp_sq, modloop_path],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    check=False,
                )
                for root, _, files in os.walk(tmp_sq):
                    for f in files:
                        if f in STORAGE_MODULE_NAMES:
                            shutil.copy(os.path.join(root, f), os.path.join(dest_dir, f))
        else:
            try:
                from PySquashfsImage import SquashFsImage
                def _collect_entries(entry):
                    if hasattr(entry, "iter"):
                        for child in entry.iter():
                            yield from _collect_entries(child)
                    else:
                        yield entry

                with open(modloop_path, "rb") as f:
                    img = SquashFsImage(f)
                    for item in _collect_entries(img.root):
                        if getattr(item, "name", "") in STORAGE_MODULE_NAMES:
                            target = os.path.join(dest_dir, item.name)
                            with open(target, "wb") as out:
                                out.write(item.read_bytes())
            except ImportError:
                print("[WARN] Neither unsquashfs nor PySquashfsImage available; cannot unpack modloop-virt.")
    except Exception as exc:
        print(f"[WARN] Could not extract storage modules from {modloop_path}: {exc}")

    extracted = [m for m in STORAGE_MODULE_NAMES if os.path.exists(os.path.join(dest_dir, m))]
    if len(extracted) == len(STORAGE_MODULE_NAMES):
        print(f"[OK] Storage kernel modules provisioned in /lib/modules/storage ({', '.join(extracted)})")
    else:
        print(f"[WARN] Partial storage modules provisioned: {extracted}")


def compute_file_sha256(filepath):
    """Compute SHA-256 hash of a file."""
    if not os.path.exists(filepath):
        return None
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def get_elf_machine(filepath):
    """Return ELF machine string ('x86_64', 'aarch64', 'arm') or None if not an ELF file."""
    if not os.path.isfile(filepath):
        return None
    try:
        with open(filepath, "rb") as f:
            header = f.read(20)
            if len(header) < 20 or header[:4] != b"\x7fELF":
                return None
            endian = "<" if header[5] == 1 else ">"
            e_machine = struct.unpack(f"{endian}H", header[18:20])[0]
            if e_machine == 0x3E:
                return "x86_64"
            elif e_machine == 0xB7:
                return "aarch64"
            elif e_machine == 0x28:
                return "arm"
            return f"unknown({hex(e_machine)})"
    except Exception:
        return None


def validate_elf_architecture(filepath, expected_arch):
    """Validate that an ELF binary matches expected architecture. Raises RuntimeError on mismatch."""
    elf_arch = get_elf_machine(filepath)
    if elf_arch is None:
        return None
    norm_expected = "aarch64" if expected_arch in ("aarch64", "arm64") else "x86_64"
    if elf_arch != norm_expected:
        raise RuntimeError(
            f"ELF architecture mismatch for {filepath}: "
            f"expected {norm_expected}, got {elf_arch}"
        )
    return elf_arch


def ensure_kernel(skip_download=False, arch="x86_64", out_dir=None):
    norm_arch = "aarch64" if arch in ("aarch64", "arm64") else ("x86_64" if arch in ("x86_64", "amd64") else None)
    if not norm_arch or norm_arch not in ARCH_CONFIGS:
        raise ValueError(f"Unknown or unsupported architecture for kernel: {arch}")
    cfg = ARCH_CONFIGS[norm_arch]
    target_out = out_dir or cfg["out_dir"]
    os.makedirs(target_out, exist_ok=True)
    kernel_path = os.path.join(target_out, cfg["kernel_name"])
    pinned_sha = cfg.get("kernel_sha256")

    if os.path.exists(kernel_path) and os.path.getsize(kernel_path) > 1000000:
        digest = compute_file_sha256(kernel_path)
        if pinned_sha:
            if digest.lower() != pinned_sha.lower():
                raise RuntimeError(
                    f"Kernel integrity verification failed for {kernel_path}!\n"
                    f"  Expected SHA-256: {pinned_sha}\n"
                    f"  Actual SHA-256:   {digest}\n"
                    f"Refusing to boot with unverified or corrupted kernel."
                )
            print(f"[OK] Kernel present: {kernel_path} ({os.path.getsize(kernel_path)} bytes, sha256: {digest[:16]}...)")
            print("[OK] Kernel SHA-256 integrity verified against pinned release.")
        else:
            print(f"[OK] Kernel present: {kernel_path} ({os.path.getsize(kernel_path)} bytes, sha256: {digest[:16]}...)")
        return kernel_path

    if skip_download:
        raise RuntimeError(f"Kernel missing at {kernel_path}; provide a pre-fetched kernel with matching checksum")

    k_url = cfg["kernel_url"]
    print(f"==> Downloading Linux LTS kernel ({norm_arch}) for QEMU from:\n    {k_url}")
    try:
        urllib.request.urlretrieve(k_url, kernel_path)
    except Exception as e:
        raise RuntimeError(f"Failed to download kernel automatically from {k_url}: {e}")

    digest = compute_file_sha256(kernel_path)
    print(f"[OK] Downloaded kernel: {kernel_path} ({os.path.getsize(kernel_path)} bytes, sha256: {digest[:16]}...)")
    if pinned_sha:
        if digest.lower() != pinned_sha.lower():
            try:
                os.remove(kernel_path)
            except OSError:
                pass
            raise RuntimeError(
                f"Downloaded kernel SHA-256 verification failed for {kernel_path}!\n"
                f"  Expected: {pinned_sha}\n"
                f"  Got:      {digest}\n"
                f"Corrupted download removed."
            )
        print("[OK] Kernel SHA-256 integrity verified.")
    return kernel_path



class CpioWriter:
    """Writes SVR4 portable cpio (070701) archives."""
    def __init__(self, f):
        self.f = f
        self.ino = 1

    def add_entry(self, name, mode, content=b"", rdevmajor=0, rdevminor=0):
        name_bytes = name.encode('utf-8') + b'\x00'
        namesize = len(name_bytes)
        filesize = len(content)

        header = (
            f"070701"
            f"{self.ino:08X}"
            f"{mode:08X}"
            f"{0:08X}"         # uid
            f"{0:08X}"         # gid
            f"{1:08X}"         # nlink
            f"{1700000000:08X}" # mtime
            f"{filesize:08X}"
            f"{0:08X}"         # devmajor
            f"{0:08X}"         # devminor
            f"{rdevmajor:08X}" # rdevmajor
            f"{rdevminor:08X}" # rdevminor
            f"{namesize:08X}"
            f"{0:08X}"         # check
        ).encode('ascii')

        self.ino += 1
        self.f.write(header)
        self.f.write(name_bytes)
        name_pad = (4 - ((110 + namesize) % 4)) % 4
        if name_pad > 0:
            self.f.write(b'\x00' * name_pad)

        if filesize > 0:
            self.f.write(content)
            data_pad = (4 - (filesize % 4)) % 4
            if data_pad > 0:
                self.f.write(b'\x00' * data_pad)

    def close(self):
        self.add_entry("TRAILER!!!", 0)
        curr_pos = self.f.tell()
        pad = (512 - (curr_pos % 512)) % 512
        if pad > 0:
            self.f.write(b'\x00' * pad)


def gzip_compress(data, compresslevel=6):
    """Deterministically gzip `data` (mtime pinned to 0, no embedded filename).

    ``gzip.compress`` defaults to stamping the current wall-clock time into the
    gzip header, which makes byte-for-byte reproducibility impossible. We write
    through a ``BytesIO`` (so no ``FNAME`` field is emitted) and force
    ``mtime=0``. Both are required for two runs on the same inputs to produce
    an identical archive.
    """
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb", compresslevel=compresslevel, mtime=0) as gz:
        gz.write(data)
    return buf.getvalue()


def build_cpio(root_dir):
    """Return the raw (uncompressed) SVR4 newc cpio archive for `root_dir`.

    All metadata is pinned (uid/gid 0, fixed mtime) and traversal order is
    sorted, so the output depends only on the file tree contents.
    """
    bio = io.BytesIO()
    writer = CpioWriter(bio)

    # Add initial essential device nodes
    writer.add_entry("dev", 0o040755, b"")
    writer.add_entry("dev/console", 0o020600, b"", rdevmajor=5, rdevminor=1)
    writer.add_entry("dev/null", 0o020666, b"", rdevmajor=1, rdevminor=3)
    writer.add_entry("dev/ttyS0", 0o020660, b"", rdevmajor=4, rdevminor=64)
    writer.add_entry("dev/tty", 0o020666, b"", rdevmajor=5, rdevminor=0)
    writer.add_entry("dev/tty0", 0o020660, b"", rdevmajor=4, rdevminor=0)
    writer.add_entry("dev/tty1", 0o020660, b"", rdevmajor=4, rdevminor=1)
    writer.add_entry("dev/kmsg", 0o020666, b"", rdevmajor=1, rdevminor=11)
    writer.add_entry("dev/fb0", 0o020660, b"", rdevmajor=29, rdevminor=0)

    # Collect files in deterministic order
    entries = []
    for root, dirs, files in os.walk(root_dir):
        dirs.sort()
        files.sort()
        rel_root = os.path.relpath(root, root_dir).replace("\\", "/")
        if rel_root != "." and rel_root != "dev":
            entries.append((rel_root, 0o040755, b""))
        for f in files:
            file_path = os.path.join(root, f)
            if os.path.islink(file_path):
                raise RuntimeError(f"Symlinks are not supported in initramfs rootfs: {file_path}")
            rel_file = (f if rel_root == "." else f"{rel_root}/{f}").replace("\\", "/")
            with open(file_path, "rb") as fp:
                data = fp.read()
            mode = 0o100755 if (rel_file == "init" or rel_file.startswith(("bin/", "sbin/", "usr/bin/"))) else 0o100644
            entries.append((rel_file, mode, data))

    for rel_path, mode, content in entries:
        writer.add_entry(rel_path, mode, content)

    writer.close()
    return bio.getvalue()


def serialize_initramfs(root_dir):
    """Return the complete gzip-compressed initramfs bytes for `root_dir`.

    Pure function: no I/O outside reading `root_dir`, so callers (including the
    reproducibility check and its unit tests) can build the image in memory.
    """
    return gzip_compress(build_cpio(root_dir), compresslevel=6)


def create_initramfs(root_dir, output_gz):
    print(f"==> Packaging rootfs ({root_dir}) into initramfs ({output_gz})...")
    raw_data = build_cpio(root_dir)
    gz_data = gzip_compress(raw_data, compresslevel=6)
    os.makedirs(os.path.dirname(os.path.abspath(output_gz)), exist_ok=True)
    with open(output_gz, "wb") as fp:
        fp.write(gz_data)
    print(f"[OK] Initramfs created: {output_gz} ({len(gz_data)} bytes, uncompressed {len(raw_data)} bytes)")


def check_reproducible(root_dir):
    """Build the initramfs twice from `root_dir` and compare SHA-256 digests.

    Returns ``(ok, first_hash, second_hash)``. Only in-memory work is done, so
    this needs neither QEMU nor the network.
    """
    first = gzip_compress(build_cpio(root_dir), compresslevel=6)
    second = gzip_compress(build_cpio(root_dir), compresslevel=6)
    first_hash = hashlib.sha256(first).hexdigest()
    second_hash = hashlib.sha256(second).hexdigest()
    return (first_hash == second_hash, first_hash, second_hash)


def prepare_rootfs(arch="x86_64", rootfs_dir=None, allow_host_fallback=False):
    norm_arch = "aarch64" if arch in ("aarch64", "arm64") else ("x86_64" if arch in ("x86_64", "amd64") else None)
    if not norm_arch or norm_arch not in ARCH_CONFIGS:
        raise ValueError(f"Unknown or unsupported architecture: {arch}")

    target_rootfs = rootfs_dir or ROOTFS
    os.makedirs(target_rootfs, exist_ok=True)
    dirs = [
        "bin", "sbin", "usr/bin", "usr/lib", "etc/nilos",
        "proc", "sys", "dev", "run/nilos", "run/onuron", "tmp", "mnt", "data",
        "lib/modules/storage"
    ]
    for d in dirs:
        os.makedirs(os.path.join(target_rootfs, d), exist_ok=True)

    provision_storage_modules(norm_arch, target_rootfs, ARCH_CONFIGS[norm_arch]["out_dir"])

    # Copy etc/nilos configs
    etc_src = os.path.join(TOP, "etc", "nilos")
    if os.path.exists(etc_src):
        shutil.copytree(etc_src, os.path.join(target_rootfs, "etc", "nilos"), dirs_exist_ok=True)

    # Target-specific release directories
    triple = ARCH_CONFIGS[norm_arch]["target_triple"]
    release_dir = os.path.join(TOP, "target", triple, "release")
    fallback_release = os.path.join(TOP, "target", "release")

    bins = [
        "nilinit", "nild", "nilkeyd", "nilbus", "nilshell",
        "inputd", "netd", "audiod", "powerd", "notifyd", "nilpkg",
        "settings", "oobe", "hello", "launcher", "nilimed", "nilttsd",
        "logd", "clipd", "btd", "vpnd", "thermald", "alarmd",
        "userd", "crashd", "nilandroidd", "nilinstall", "nilup", "nilperf",
        "nilc", "nilrt-launch", "nilrt"
    ]
    installed_bins = {}

    # Cross-architecture check: if target is aarch64, fallback to host target/release is prohibited unless explicitly allowed
    allow_fallback = (norm_arch == "x86_64") or allow_host_fallback

    for b in bins:
        target_path = os.path.join(target_rootfs, "usr", "bin", b)
        src_musl = os.path.join(release_dir, b)
        src_fb = os.path.join(fallback_release, b)

        chosen_src = None
        if os.path.exists(src_musl):
            chosen_src = src_musl
        elif allow_fallback and os.path.exists(src_fb):
            chosen_src = src_fb

        if chosen_src:
            # Validate ELF machine type if it's an ELF file
            elf_type = validate_elf_architecture(chosen_src, norm_arch)
            shutil.copy2(chosen_src, target_path)
            shutil.copy2(chosen_src, os.path.join(target_rootfs, "bin", b))
            b_hash = compute_file_sha256(chosen_src)
            installed_bins[b] = {
                "path": f"usr/bin/{b}",
                "sha256": b_hash,
                "elf_machine": elf_type or "script/native",
                "source": "musl" if chosen_src == src_musl else "host-fallback",
            }
            src_label = f"musl ({norm_arch})" if chosen_src == src_musl else "host-fallback"
            print(f"[+] Installed {src_label} binary: {b}")

    if not os.path.isfile(os.path.join(target_rootfs, "usr", "bin", "nilinit")):
        raise RuntimeError(
            f"nilinit binary is missing for {norm_arch} in {release_dir}!\n"
            f"Run 'cargo build --release --target {triple}' before packaging initramfs."
        )

    # Link /init and /sbin/init to nilinit
    nilinit_bin = os.path.join(target_rootfs, "usr", "bin", "nilinit")
    shutil.copy2(nilinit_bin, os.path.join(target_rootfs, "init"))
    shutil.copy2(nilinit_bin, os.path.join(target_rootfs, "bin", "nilinit"))
    shutil.copy2(nilinit_bin, os.path.join(target_rootfs, "sbin", "init"))
    print("[+] /init and /sbin/init linked to nilinit")
    return installed_bins


def write_manifest_and_checksums(out_dir, arch, kernel_file, initrd_file, installed_bins, non_release=False):
    """Write reproducible manifest.json and checksums.txt into out directory."""
    k_hash = compute_file_sha256(kernel_file) if kernel_file and os.path.exists(kernel_file) else None
    i_hash = compute_file_sha256(initrd_file) if initrd_file and os.path.exists(initrd_file) else None

    checksums_path = os.path.join(out_dir, "checksums.txt")
    with open(checksums_path, "w", encoding="utf-8") as f:
        if k_hash:
            f.write(f"{k_hash}  {os.path.basename(kernel_file)}\n")
        if i_hash:
            f.write(f"{i_hash}  {os.path.basename(initrd_file)}\n")

    git_rev = "unknown"
    try:
        git_rev = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=TOP, text=True).strip()
    except Exception:
        pass

    triple = ARCH_CONFIGS.get(arch, {}).get("target_triple", f"{arch}-unknown-linux-musl")

    manifest = {
        "target": f"qemu-{arch}",
        "architecture": arch,
        "userspace_triple": triple,
        "kernel_arch": arch,
        "build_revision": git_rev,
        "non_release": non_release,
        "kernel": {
            "file": os.path.basename(kernel_file) if kernel_file else None,
            "sha256": k_hash,
            "size_bytes": os.path.getsize(kernel_file) if kernel_file and os.path.exists(kernel_file) else 0,
        },
        "initramfs": {
            "file": os.path.basename(initrd_file) if initrd_file else None,
            "sha256": i_hash,
            "size_bytes": os.path.getsize(initrd_file) if initrd_file and os.path.exists(initrd_file) else 0,
        },
        "installed_daemons": list(installed_bins.keys()) if isinstance(installed_bins, dict) else installed_bins,
        "binaries": installed_bins if isinstance(installed_bins, dict) else {},
        "verified": True,
    }
    manifest_path = os.path.join(out_dir, "manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
    print(f"[OK] Generated manifest: {manifest_path}")
    print(f"[OK] Generated checksums: {checksums_path}")


def main(argv=None):
    parser = argparse.ArgumentParser(description="NilOS Initramfs & Image Builder")
    parser.add_argument("--target", default=None,
                        help="Target profile name or alias (e.g. qemu-x86_64, qemu-aarch64)")
    parser.add_argument("--arch", choices=["x86_64", "aarch64", "arm64"], default="x86_64",
                        help="Target CPU architecture (x86_64 or aarch64, default: x86_64)")
    parser.add_argument("--skip-kernel-download", action="store_true",
                        help="fail instead of downloading a missing kernel")
    parser.add_argument("--check-reproducible", action="store_true",
                        help="build the initramfs twice and verify identical SHA-256")
    parser.add_argument("--allow-host-binaries-for-tests", action="store_true",
                        help="allow host binaries fallback for cross-targets (marks manifest non_release=true)")
    parser.add_argument("--rootfs", default=None,
                        help="rootfs directory to package (default: the generated one); "
                             "with --check-reproducible this can be any pre-populated tree")
    args = parser.parse_args(argv)

    target_name = args.target or args.arch
    canonical = target_registry.resolve_target(target_name)
    norm_arch = "aarch64" if canonical == "qemu-aarch64" else "x86_64"
    cfg = ARCH_CONFIGS[norm_arch]
    out_dir = target_registry.get_target_output_dir(canonical)
    target_rootfs = args.rootfs or os.path.join(out_dir, "rootfs")
    initrd_path = os.path.join(out_dir, cfg["initrd_name"])

    print("=========================================================")
    print(f"      NilOS Initramfs & Image Builder ({norm_arch})      ")
    print("=========================================================")

    if args.check_reproducible:
        if args.rootfs is None:
            ensure_kernel(skip_download=args.skip_kernel_download, arch=norm_arch, out_dir=out_dir)
            prepare_rootfs(arch=norm_arch, rootfs_dir=target_rootfs, allow_host_fallback=args.allow_host_binaries_for_tests)
        if not os.path.isdir(target_rootfs):
            raise RuntimeError(f"rootfs directory not found: {target_rootfs}")
        ok, first_hash, second_hash = check_reproducible(target_rootfs)
        print(f"    build #1 sha256: {first_hash}")
        print(f"    build #2 sha256: {second_hash}")
        if not ok:
            raise RuntimeError("initramfs is NOT reproducible (digests differ)")
        print("[OK] Initramfs is byte-for-byte reproducible.")
        return 0

    kernel_path = ensure_kernel(skip_download=args.skip_kernel_download, arch=norm_arch, out_dir=out_dir)
    installed_bins = prepare_rootfs(arch=norm_arch, rootfs_dir=target_rootfs, allow_host_fallback=args.allow_host_binaries_for_tests)
    create_initramfs(target_rootfs, initrd_path)
    write_manifest_and_checksums(out_dir, norm_arch, kernel_path, initrd_path, installed_bins, non_release=args.allow_host_binaries_for_tests)
    return 0



if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"[ERROR] Initramfs build failed: {exc}", file=sys.stderr)
        sys.exit(1)
