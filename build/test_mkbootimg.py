#!/usr/bin/env python3
"""
build/test_mkbootimg.py — Unit tests for OnuronOS Boot Image Packager
"""

import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(__file__), "mkbootimg.py")
SPEC = importlib.util.spec_from_file_location("mkbootimg", SCRIPT)
mkbootimg = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mkbootimg)


class TestMkBootImg(unittest.TestCase):
    def test_pad_size(self):
        self.assertEqual(mkbootimg.pad_size(100, 2048), 2048)
        self.assertEqual(mkbootimg.pad_size(2048, 2048), 2048)
        self.assertEqual(mkbootimg.pad_size(2049, 2048), 4096)
        self.assertEqual(mkbootimg.pad_size(0, 4096), 0)

    def test_os_version_encoding(self):
        ver_str = "14.2.1"
        patch_str = "2026-10"
        encoded = mkbootimg.encode_os_version(ver_str, patch_str)
        dec_ver, dec_patch = mkbootimg.decode_os_version(encoded)
        self.assertEqual(dec_ver, ver_str)
        self.assertEqual(dec_patch, patch_str)

    def test_boot_img_v2_roundtrip(self):
        kernel = b"FAKEMOBILEKERNEL" * 32
        ramdisk = b"FAKERAMDISKINITRAMFS" * 64
        dtb = b"FAKEDEVICETREEBLOB" * 16
        cmdline = "console=ttyMSM0,115200 root=/dev/ram0 rw quiet"

        img = mkbootimg.build_boot_img_v0_2(
            kernel_data=kernel,
            ramdisk_data=ramdisk,
            dtb_data=dtb,
            cmdline=cmdline,
            header_version=2,
            page_size=2048,
            os_version_str="14.0.0",
            os_patch_level_str="2026-10",
            board_name="onuron-arm64",
        )

        self.assertTrue(img.startswith(mkbootimg.BOOT_MAGIC))
        parsed = mkbootimg.parse_boot_img(img)
        self.assertEqual(parsed["header_version"], 2)
        self.assertEqual(parsed["page_size"], 2048)
        self.assertEqual(parsed["kernel_size"], len(kernel))
        self.assertEqual(parsed["ramdisk_size"], len(ramdisk))
        self.assertEqual(parsed["dtb_size"], len(dtb))
        self.assertEqual(parsed["kernel_data"], kernel)
        self.assertEqual(parsed["ramdisk_data"], ramdisk)
        self.assertEqual(parsed["dtb_data"], dtb)
        self.assertIn("console=ttyMSM0", parsed["cmdline"])
        self.assertEqual(parsed["board_name"], "onuron-arm64")
        self.assertEqual(parsed["os_version"], "14.0.0")
        self.assertEqual(parsed["os_patch_level"], "2026-10")

    def test_boot_img_v3_roundtrip(self):
        kernel = b"V3KERNELPAYLOAD" * 50
        ramdisk = b"V3RAMDISKPAYLOAD" * 100
        cmdline = "bootopt=64S3,32N2,64N2 androidboot.hardware=qcom"

        img = mkbootimg.build_boot_img_v3_4(
            kernel_data=kernel,
            ramdisk_data=ramdisk,
            cmdline=cmdline,
            header_version=3,
            os_version_str="14.1.0",
            os_patch_level_str="2026-10",
        )

        self.assertTrue(img.startswith(mkbootimg.BOOT_MAGIC))
        parsed = mkbootimg.parse_boot_img(img)
        self.assertEqual(parsed["header_version"], 3)
        self.assertEqual(parsed["page_size"], 4096)
        self.assertEqual(parsed["kernel_size"], len(kernel))
        self.assertEqual(parsed["ramdisk_size"], len(ramdisk))
        self.assertEqual(parsed["kernel_data"], kernel)
        self.assertEqual(parsed["ramdisk_data"], ramdisk)
        self.assertEqual(parsed["cmdline"], cmdline)
        self.assertEqual(parsed["os_version"], "14.1.0")

    def test_cli_create_and_unpack(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            k_path = os.path.join(tmpdir, "Image.gz")
            rd_path = os.path.join(tmpdir, "initramfs.cpio.gz")
            out_img = os.path.join(tmpdir, "boot.img")
            unpack_dir = os.path.join(tmpdir, "unpacked")

            with open(k_path, "wb") as f:
                f.write(b"SAMPLE_KERNEL_STREAM" * 40)
            with open(rd_path, "wb") as f:
                f.write(b"SAMPLE_RAMDISK_STREAM" * 80)

            # Test CLI create
            res = subprocess.run(
                [
                    sys.executable,
                    SCRIPT,
                    "create",
                    "--kernel", k_path,
                    "--ramdisk", rd_path,
                    "--cmdline", "console=tty0 root=/dev/vda",
                    "-o", out_img,
                ],
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertIn("[OK] Boot image generated", res.stdout)
            self.assertTrue(os.path.exists(out_img))

            # Test CLI info
            res_info = subprocess.run(
                [sys.executable, SCRIPT, "info", out_img],
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertIn("Android Boot Image Metadata:", res_info.stdout)

            # Test CLI unpack
            res_unpack = subprocess.run(
                [sys.executable, SCRIPT, "unpack", out_img, "-o", unpack_dir],
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertIn("[OK] Unpacked", res_unpack.stdout)
            self.assertTrue(os.path.exists(os.path.join(unpack_dir, "kernel")))
            self.assertTrue(os.path.exists(os.path.join(unpack_dir, "ramdisk.cpio.gz")))

    def test_invalid_magic_raises(self):
        with self.assertRaises(ValueError):
            mkbootimg.parse_boot_img(b"NOT_BOOT_IMAGE_HEADER_DATA")

    def test_cli_fajita_profile(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            k_path = os.path.join(tmpdir, "Image.gz")
            rd_path = os.path.join(tmpdir, "initramfs.cpio.gz")
            out_img = os.path.join(tmpdir, "fajita_boot.img")

            with open(k_path, "wb") as f:
                f.write(b"SAMPLE_FAJITA_KERNEL" * 32)
            with open(rd_path, "wb") as f:
                f.write(b"SAMPLE_FAJITA_RAMDISK" * 64)

            res = subprocess.run(
                [
                    sys.executable,
                    SCRIPT,
                    "create",
                    "--profile", "fajita",
                    "--kernel", k_path,
                    "--ramdisk", rd_path,
                    "-o", out_img,
                ],
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertIn("[OK] Boot image generated", res.stdout)
            self.assertTrue(os.path.exists(out_img))

            with open(out_img, "rb") as f:
                parsed = mkbootimg.parse_boot_img(f.read())
            self.assertEqual(parsed["board_name"], "fajita")
            self.assertEqual(parsed["page_size"], 4096)
            self.assertIn("androidboot.hardware=qcom", parsed["cmdline"])

    def test_cli_profile_conflicting_arg_fails(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            k_path = os.path.join(tmpdir, "Image.gz")
            rd_path = os.path.join(tmpdir, "initramfs.cpio.gz")
            out_img = os.path.join(tmpdir, "conflict_boot.img")

            with open(k_path, "wb") as f:
                f.write(b"SAMPLE_KERNEL" * 32)
            with open(rd_path, "wb") as f:
                f.write(b"SAMPLE_RAMDISK" * 64)

            res = subprocess.run(
                [
                    sys.executable,
                    SCRIPT,
                    "create",
                    "--profile", "fajita",
                    "--pagesize", "2048",  # Fajita profile specifies 4096
                    "--kernel", k_path,
                    "--ramdisk", rd_path,
                    "-o", out_img,
                ],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(res.returncode, 0)
            self.assertIn("Conflicting argument --pagesize", res.stderr)

    def test_cli_canonical_target_profile(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            k_path = os.path.join(tmpdir, "Image.gz")
            rd_path = os.path.join(tmpdir, "initramfs.cpio.gz")
            out_img = os.path.join(tmpdir, "canonical_boot.img")

            with open(k_path, "wb") as f:
                f.write(b"SAMPLE_KERNEL" * 32)
            with open(rd_path, "wb") as f:
                f.write(b"SAMPLE_RAMDISK" * 64)

            res = subprocess.run(
                [
                    sys.executable,
                    SCRIPT,
                    "create",
                    "--profile", "oneplus-fajita",
                    "--kernel", k_path,
                    "--ramdisk", rd_path,
                    "-o", out_img,
                ],
                capture_output=True,
                text=True,
                check=True,
            )
            self.assertIn("[OK] Boot image generated", res.stdout)
            self.assertTrue(os.path.exists(out_img))


if __name__ == "__main__":
    unittest.main()
