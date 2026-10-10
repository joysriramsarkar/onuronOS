#!/usr/bin/env python3
"""
build/test_flasher_preflight.py — Unit tests for Fastboot Flasher Preflight & Safety Gates

Tests safety guarantees from Roadmap Section 47.9:
- No device connected -> fails closed
- Multiple devices connected -> fails closed
- Product mismatch -> fails closed
- Unknown/empty product -> fails closed
- Missing boot/system/vbmeta images -> fails closed
- Flashing not allowed (flashing_allowed=false) -> blocks physical flashing
- Target alias resolution (fajita -> oneplus-fajita) -> works seamlessly
- Dry-run validation -> preflight passes without writing
- Asserts that NO destructive commands (flash/erase/format/reboot) are executed
  on any safety failure or in dry-run mode.
"""

import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FLASH_SH = os.path.join(TOP, "build", "flash-device.sh")
FLASH_PS1 = os.path.join(TOP, "build", "flash-device.ps1")


class TestFlasherPreflight(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()
        self.fake_bin = os.path.join(self.tmpdir, "bin")
        os.makedirs(self.fake_bin, exist_ok=True)
        self.log_file = os.path.join(self.tmpdir, "fastboot_calls.log")
        self.config_file = os.path.join(self.tmpdir, "mock_config.json")
        self.out_fajita = os.path.join(TOP, "out", "oneplus-fajita")
        os.makedirs(self.out_fajita, exist_ok=True)
        self.created_images = []

    def tearDown(self):
        for f in self.created_images:
            try:
                if os.path.exists(f):
                    os.remove(f)
            except OSError:
                pass
        shutil.rmtree(self.tmpdir, ignore_errors=True)

    def _setup_mock_fastboot(self, devices="12345678 fastboot", product="fajita", slot="a"):
        """Configure mock fastboot CLI dispatcher."""
        config = {
            "devices": devices,
            "product": product,
            "slot": slot,
            "log_path": self.log_file,
        }
        with open(self.config_file, "w", encoding="utf-8") as f:
            json.dump(config, f)

        mock_py = os.path.join(self.fake_bin, "mock_fastboot.py")
        with open(mock_py, "w", encoding="utf-8") as f:
            f.write(r'''import json, os, sys
config_path = os.environ.get("MOCK_FASTBOOT_CONFIG", "")
if not config_path or not os.path.exists(config_path):
    sys.exit(1)

with open(config_path, "r", encoding="utf-8") as f:
    cfg = json.load(f)

log_path = cfg.get("log_path")
if log_path:
    with open(log_path, "a", encoding="utf-8") as f:
        f.write(" ".join(sys.argv[1:]) + "\n")

args = sys.argv[1:]
if not args:
    sys.exit(0)

cmd = args[0]
if cmd == "devices":
    devs = cfg.get("devices", "")
    if devs:
        print(devs)
    sys.exit(0)

if cmd == "getvar":
    var = args[1] if len(args) > 1 else ""
    if var == "product":
        print(f"product: {cfg.get('product', 'unknown')}")
        sys.exit(0)
    elif var == "current-slot":
        slot = cfg.get("slot", "")
        if slot:
            print(f"current-slot: {slot}")
        sys.exit(0)
    else:
        print(f"{var}: unknown")
        sys.exit(0)

print(f"fastboot mock: {' '.join(args)}")
sys.exit(0)
''')

        if sys.platform == "win32":
            mock_bat = os.path.join(self.fake_bin, "fastboot.cmd")
            with open(mock_bat, "w", encoding="utf-8") as f:
                f.write(f'@echo off\n"{sys.executable}" "{mock_py}" %*\n')
        else:
            mock_sh = os.path.join(self.fake_bin, "fastboot")
            with open(mock_sh, "w", encoding="utf-8") as f:
                f.write(f'#!/bin/sh\nexec "{sys.executable}" "{mock_py}" "$@"\n')
            st = os.stat(mock_sh)
            os.chmod(mock_sh, st.st_mode | stat.S_IEXEC | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    def _create_dummy_images(self, include_vbmeta=True):
        boot_img = os.path.join(self.out_fajita, "boot.img")
        sys_img = os.path.join(self.out_fajita, "system_a.img")
        vbmeta_img = os.path.join(self.out_fajita, "vbmeta_a.img")

        if not os.path.exists(boot_img):
            with open(boot_img, "wb") as f:
                f.write(b"ANDROID!" + b"\x00" * 4088)
            self.created_images.append(boot_img)
        if not os.path.exists(sys_img):
            with open(sys_img, "wb") as f:
                f.write(b"DUMMY_SYSTEM_IMAGE_PAYLOAD" * 32)
            self.created_images.append(sys_img)
        if include_vbmeta and not os.path.exists(vbmeta_img):
            with open(vbmeta_img, "wb") as f:
                f.write(b"AVB0" + b"\x00" * 252)
            self.created_images.append(vbmeta_img)

    def _run_flasher(self, target="oneplus-fajita", extra_args=None):
        extra = extra_args or []
        env = os.environ.copy()
        env["PATH"] = self.fake_bin + os.pathsep + env.get("PATH", "")
        env["MOCK_FASTBOOT_CONFIG"] = self.config_file

        if sys.platform == "win32":
            cmd = [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy", "Bypass",
                "-File", FLASH_PS1,
                "-Target", target,
            ] + extra
        else:
            cmd = ["bash", FLASH_SH, target] + extra

        return subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            env=env,
            cwd=TOP,
        )

    def _assert_no_destructive_commands(self):
        """Verify that fastboot was NEVER called with flash, erase, format, or reboot."""
        if not os.path.exists(self.log_file):
            return
        with open(self.log_file, "r", encoding="utf-8") as f:
            lines = [line.strip() for line in f if line.strip()]

        for line in lines:
            parts = line.split()
            if not parts:
                continue
            first = parts[0].lower()
            self.assertNotIn(
                first,
                ["flash", "erase", "format", "reboot"],
                f"Destructive fastboot command was executed: '{line}'"
            )

    def test_dry_run_preflight(self):
        """Dry-run passes preflight with full digest calculation and zero device writes."""
        self._setup_mock_fastboot(product="fajita", slot="a")
        self._create_dummy_images(include_vbmeta=True)

        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 0, f"Stdout: {res.stdout}\nStderr: {res.stderr}")
        self.assertIn("[DRY-RUN COMPLETE]", res.stdout)
        self.assertIn("Boot image:", res.stdout)
        self.assertIn("SHA-256:", res.stdout)
        self.assertIn("Planned Fastboot Command Sequence:", res.stdout)
        self._assert_no_destructive_commands()

    def test_target_alias_resolution(self):
        """Target alias 'fajita' resolves to canonical 'oneplus-fajita'."""
        self._setup_mock_fastboot(product="fajita", slot="a")
        self._create_dummy_images(include_vbmeta=True)

        res = self._run_flasher("fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 0, f"Stdout: {res.stdout}\nStderr: {res.stderr}")
        self.assertIn("oneplus-fajita", res.stdout)
        self._assert_no_destructive_commands()

    def test_flash_blocked_when_flashing_not_allowed(self):
        """Real flashing without --dry-run or -ForceUnsupported is blocked by default."""
        self._setup_mock_fastboot(product="fajita")
        self._create_dummy_images(include_vbmeta=True)

        res = self._run_flasher("oneplus-fajita")
        self.assertEqual(res.returncode, 1)
        self.assertIn("flashing_allowed=false", res.stdout + res.stderr)
        self._assert_no_destructive_commands()

    def test_no_device_connected(self):
        """Fails closed when fastboot devices reports no connected device."""
        self._setup_mock_fastboot(devices="")
        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 1)
        self.assertIn("No device connected", res.stdout + res.stderr)
        self._assert_no_destructive_commands()

    def test_multiple_devices_connected(self):
        """Fails closed when multiple fastboot devices are detected to prevent ambiguous writes."""
        self._setup_mock_fastboot(devices="device1 fastboot\ndevice2 fastboot")
        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 1)
        self.assertIn("Multiple fastboot devices detected", res.stdout + res.stderr)
        self._assert_no_destructive_commands()

    def test_product_mismatch(self):
        """Fails closed when device product does not match fajita."""
        self._setup_mock_fastboot(product="blueline")
        self._create_dummy_images(include_vbmeta=True)

        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 1)
        self.assertIn("does not match target", res.stdout + res.stderr)
        self._assert_no_destructive_commands()

    def test_unknown_product(self):
        """Fails closed when device reports unknown product."""
        self._setup_mock_fastboot(product="unknown")
        self._create_dummy_images(include_vbmeta=True)

        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 1)
        self.assertIn("unknown or empty product identifier", res.stdout + res.stderr)
        self._assert_no_destructive_commands()

    def test_missing_vbmeta_on_hardware_target(self):
        """Fails closed when verified boot vbmeta descriptor is missing for hardware target."""
        self._setup_mock_fastboot(product="fajita")
        self._create_dummy_images(include_vbmeta=False)

        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 1)
        self.assertIn("Missing verified boot descriptor", res.stdout + res.stderr)
        self._assert_no_destructive_commands()

    def test_ab_slot_partition_resolution(self):
        """Resolves target A/B slot partitions (boot_b, system_b, vbmeta_b) when slot is b."""
        self._setup_mock_fastboot(product="fajita", slot="b")
        self._create_dummy_images(include_vbmeta=True)

        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 0)
        output = res.stdout + res.stderr
        self.assertIn("boot_b", output)
        self.assertIn("system_b", output)
        self.assertIn("vbmeta_b", output)
        self._assert_no_destructive_commands()

    def test_checksum_mismatch_fails_closed(self):
        """Aborts immediately when image hash fails verification against checksums.txt."""
        self._setup_mock_fastboot(product="fajita", slot="a")
        self._create_dummy_images(include_vbmeta=True)
        checksum_path = os.path.join(self.out_fajita, "checksums.txt")
        self.created_images.append(checksum_path)
        with open(checksum_path, "w", encoding="utf-8") as f:
            f.write("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  boot.img\n")

        res = self._run_flasher("oneplus-fajita", ["-DryRun"] if sys.platform == "win32" else ["--dry-run"])
        self.assertEqual(res.returncode, 1)
        self.assertIn("Image digest mismatch", res.stdout + res.stderr)
        self._assert_no_destructive_commands()


if __name__ == "__main__":
    unittest.main()

