#!/usr/bin/env python3
"""A3 — Persistent-data validation harness (QEMU-less).

Builds a *real* ext4 data image from a known staging tree and reads the files
back with ``debugfs``, asserting they are present and byte-identical. This
proves the image layer of the persistent-data story (build -> filesystem ->
read back) without booting QEMU.

What this validates:
  * ``mkdisk.build_real_image`` produces a genuinely mountable ext4 image.
  * Known files (OOBE flag, PIN record, contacts, SMS threads, ...) survive
    being written at build time and read back from the image.
  * A second build from the same staging tree still contains the data.

What this does NOT validate (needs QEMU / a real boot):
  * that ``nilinit`` mounts /data and falls back safely when it is missing;
  * that the shell actually reads the name/PIN/SMS state from the image.

Requires Linux ``mke2fs`` and ``debugfs`` (e2fsprogs). When either is absent
(e.g. on Windows) the harness prints a SKIP message and exits 0.

Usage:
    python build/persistent-data-test.py [--size-mb N] [--keep DIR]
"""
import argparse
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, filename):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, filename))
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


mkdisk = _load("mkdisk", "mkdisk.py")

# The state a first boot is expected to persist under /data. Kept deliberately
# small and pure-bytes so read-back comparison is exact.
KNOWN_FILES = {
    "config/oobe_done": b"true\n",
    "config/device.toml": b'name = "Onuron Test Device"\n',
    "config/pin.json": b'{"algo":"sha256","iterations":100000,"salt":"00112233","hash":"deadbeef"}\n',
    "contacts/contacts.json": b'[{"name":"Ada Lovelace","number":"+15550100"}]\n',
    "sms/threads.json": b'{"threads":[{"peer":"+15550100","messages":["hello","world"]}]}\n',
}


def materialize(staging_dir):
    """Write KNOWN_FILES into a staging tree and return the directory."""
    for rel, content in KNOWN_FILES.items():
        path = os.path.join(staging_dir, rel.replace("/", os.sep))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(content)
    return staging_dir


def parse_ls_names(text):
    """Extract entry names from `debugfs -R "ls DIR"` output.

    debugfs may emit a leading line like ``2  3  4`` (inode numbers) before the
    names, and always separates names with whitespace. ``.`` and ``..`` are
    dropped. Pure function so it can be unit-tested without e2fsprogs.
    """
    names = set()
    for line in text.splitlines():
        for token in line.split():
            if token in (".", ".."):
                continue
            # Skip pure-numeric inode-number tokens from the header line.
            if token.isdigit():
                continue
            names.add(token)
    return names


def compare_entries(expected, present):
    """Return (missing, unexpected) sets for a dict/set comparison.

    Pure decision helper: `expected` is an iterable of names that must exist,
    `present` is an iterable of names actually found.
    """
    expected = set(expected)
    present = set(present)
    return sorted(expected - present), sorted(present - expected)


def debugfs_cat(debugfs, image, path):
    """Return the bytes of `path` inside `image`, or None if missing."""
    proc = subprocess.run(
        [debugfs, "-R", f"cat {path}", image],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        return None
    return proc.stdout


def debugfs_ls(debugfs, image, path):
    """Return the entry-name set for a directory inside `image` (or empty)."""
    proc = subprocess.run(
        [debugfs, "-R", f"ls {path}", image],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        return set()
    return parse_ls_names(proc.stdout.decode("utf-8", errors="replace"))


def verify_image(debugfs, image):
    """Return a list of human-readable failures for the populated image."""
    failures = []
    for rel, expected in KNOWN_FILES.items():
        got = debugfs_cat(debugfs, image, "/" + rel)
        if got is None:
            failures.append(f"missing file: /{rel}")
        elif got != expected:
            failures.append(f"content mismatch: /{rel} (got {got!r})")

    # Directory listing sanity check using the pure decision helper.
    present = debugfs_ls(debugfs, image, "/config")
    missing, _ = compare_entries(["oobe_done", "device.toml", "pin.json"], present)
    for name in missing:
        failures.append(f"missing directory entry: /config/{name}")
    return failures


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size-mb", type=int, default=64,
                        help="size of the test image (default 64 MiB)")
    parser.add_argument("--keep", default=None,
                        help="write the built image to this path instead of a temp dir")
    args = parser.parse_args(argv)

    mkfs = mkdisk.mke2fs_path()
    debugfs = mkdisk.debugfs_path()
    if not mkfs or not debugfs:
        print("[SKIP] persistent-data-test requires Linux e2fsprogs (mke2fs + debugfs).")
        print(f"[SKIP]   mke2fs:  {mkfs or 'not found'}")
        print(f"[SKIP]   debugfs: {debugfs or 'not found'}")
        print("[SKIP] Install with: sudo apt-get install e2fsprogs")
        print("[SKIP] Skipping: no persistent-image validation performed on this platform.")
        return 0

    tmp = tempfile.mkdtemp(prefix="onuron-persist-")
    try:
        staging = materialize(os.path.join(tmp, "staging"))
        image = args.keep or os.path.join(tmp, "nilos-data.img")

        print(f"==> Building real ext4 image ({args.size_mb} MiB) from {staging}")
        mkdisk.build_real_image(image, size_mb=args.size_mb, populate_dir=staging)

        print("==> Reading data back with debugfs")
        failures = verify_image(debugfs, image)

        # Build a second image from the same staging tree and re-verify. This is
        # the closest QEMU-less analogue of "remount the same data".
        second = os.path.join(tmp, "nilos-data-2.img")
        mkdisk.build_real_image(second, size_mb=args.size_mb, populate_dir=staging)
        failures += [f"second build: {f}" for f in verify_image(debugfs, second)]

        if failures:
            print("[FAIL] persistent-data validation failed:", file=sys.stderr)
            for failure in failures:
                print(f"  - {failure}", file=sys.stderr)
            return 1

        print(f"[OK] {len(KNOWN_FILES)} known files present and byte-identical after read-back")
        print("[OK] Second build from the same tree contains the same state")
        return 0
    finally:
        if not args.keep:
            shutil.rmtree(tmp, ignore_errors=True)
        else:
            print(f"[OK] Kept test image at {args.keep}")


if __name__ == "__main__":
    sys.exit(main())