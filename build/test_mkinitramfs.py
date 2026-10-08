"""Regression tests for initramfs archive layout and unsafe input handling."""
import gzip
import importlib.util
import io
import os
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(__file__), "mkinitramfs.py")
SPEC = importlib.util.spec_from_file_location("mkinitramfs", SCRIPT)
mkinitramfs = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mkinitramfs)


def cpio_entries(raw):
    """Parse enough newc to assert entry names, modes and payloads."""
    entries = {}
    pos = 0
    while pos + 110 <= len(raw):
        header = raw[pos:pos + 110]
        if header[:6] != b"070701":
            raise AssertionError(f"invalid newc magic at {pos}")
        fields = [int(header[i:i + 8], 16) for i in range(6, 110, 8)]
        mode, filesize, namesize = fields[1], fields[6], fields[11]
        pos += 110
        name = raw[pos:pos + namesize - 1].decode()
        pos += namesize
        pos = (pos + 3) & ~3
        payload = raw[pos:pos + filesize]
        pos += filesize
        pos = (pos + 3) & ~3
        if name == "TRAILER!!!":
            break
        entries[name] = (mode, payload)
    return entries


class InitramfsTests(unittest.TestCase):
    def test_archive_has_valid_newc_entries_and_executable_mode(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = os.path.join(tmp, "root")
            os.makedirs(os.path.join(root, "usr", "bin"))
            with open(os.path.join(root, "usr", "bin", "demo"), "wb") as f:
                f.write(b"#!/bin/sh\necho ok\n")
            output = os.path.join(tmp, "images", "initramfs.cpio.gz")
            mkinitramfs.create_initramfs(root, output)
            with gzip.open(output, "rb") as f:
                entries = cpio_entries(f.read())
            self.assertEqual(entries["usr/bin/demo"][1], b"#!/bin/sh\necho ok\n")
            self.assertEqual(entries["usr/bin/demo"][0] & 0o777, 0o755)
            self.assertEqual(entries["dev/console"][0] & 0o170000, 0o020000)

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_symlink_in_rootfs_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = os.path.join(tmp, "root")
            os.mkdir(root)
            with open(os.path.join(tmp, "payload"), "wb") as f:
                f.write(b"data")
            try:
                os.symlink(os.path.join(tmp, "payload"), os.path.join(root, "escape"))
            except OSError as exc:
                if getattr(exc, "winerror", None) == 1314:
                    self.skipTest("Windows symlink privilege unavailable")
                raise
            with self.assertRaisesRegex(RuntimeError, "Symlinks are not supported"):
                mkinitramfs.create_initramfs(root, os.path.join(tmp, "initrd.gz"))

    def test_only_executable_directories_get_executable_bit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = os.path.join(tmp, "root")
            os.makedirs(os.path.join(root, "mybin"))
            os.makedirs(os.path.join(root, "usr", "bin"))
            for name in ("mybinary", "usr/bin/app"):
                path = os.path.join(root, name)
                os.makedirs(os.path.dirname(path), exist_ok=True)
                with open(path, "wb") as f:
                    f.write(b"x")
            out = io.BytesIO()
            mkinitramfs.CpioWriter(out)
            archive = os.path.join(tmp, "initrd.gz")
            mkinitramfs.create_initramfs(root, archive)
            with gzip.open(archive, "rb") as f:
                entries = cpio_entries(f.read())
            self.assertEqual(entries["mybinary"][0] & 0o777, 0o644)
            self.assertEqual(entries["usr/bin/app"][0] & 0o777, 0o755)


if __name__ == "__main__":
    unittest.main()
