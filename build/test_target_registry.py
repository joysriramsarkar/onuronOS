#!/usr/bin/env python3
"""
build/test_target_registry.py — Unit Tests for Canonical Target Registry
"""

import unittest
import sys
import os

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(TOP, "build"))

import target_registry


class TestTargetRegistry(unittest.TestCase):
    def test_canonical_targets_resolve(self):
        for target in target_registry.CANONICAL_TARGETS:
            self.assertEqual(target_registry.resolve_target(target, warn=False), target)

    def test_alias_resolution(self):
        self.assertEqual(target_registry.resolve_target("x86_64-generic", warn=False), "qemu-x86_64")
        self.assertEqual(target_registry.resolve_target("aarch64-qemu", warn=False), "qemu-aarch64")
        self.assertEqual(target_registry.resolve_target("arm64-generic", warn=False), "qemu-aarch64")
        self.assertEqual(target_registry.resolve_target("fajita", warn=False), "oneplus-fajita")
        self.assertEqual(target_registry.resolve_target("s25", warn=False), "android-host-arm64")

    def test_unknown_target_raises(self):
        with self.assertRaises(ValueError):
            target_registry.resolve_target("unknown-nonexistent-device", warn=False)

    def test_load_all_canonical_targets(self):
        for target in target_registry.CANONICAL_TARGETS:
            cfg = target_registry.load_target(target, warn=False)
            self.assertIn("target", cfg)
            tinfo = cfg["target"]
            self.assertEqual(tinfo["id"], target)
            self.assertIn("architecture", tinfo)
            self.assertIn("target_triple", tinfo)
            self.assertIn("output_dir", tinfo)
            self.assertIn("flashing_allowed", tinfo)

    def test_flashing_allowed_rules(self):
        # Fail-closed invariant: all targets default to flashing_allowed=False until
        # Milestone M5 recovery validation and safety gates are formally approved.
        self.assertFalse(target_registry.is_flashing_allowed("qemu-x86_64"))
        self.assertFalse(target_registry.is_flashing_allowed("qemu-aarch64"))
        self.assertFalse(target_registry.is_flashing_allowed("android-host-arm64"))
        self.assertFalse(target_registry.is_flashing_allowed("oneplus-fajita"))

    def test_triples_and_output_dirs(self):
        self.assertEqual(
            target_registry.get_target_triple("qemu-x86_64"),
            "x86_64-unknown-linux-musl"
        )
        self.assertEqual(
            target_registry.get_target_triple("qemu-aarch64"),
            "aarch64-unknown-linux-musl"
        )
        self.assertEqual(
            target_registry.get_target_triple("android-host-arm64"),
            "aarch64-linux-android"
        )
        out_dir = target_registry.get_target_output_dir("oneplus-fajita")
        self.assertTrue(out_dir.endswith(os.path.join("out", "oneplus-fajita")))

    def test_is_flashing_allowed_cli(self):
        import subprocess
        res = subprocess.run(
            [sys.executable, os.path.join(TOP, "build", "target_registry.py"), "is-flashing-allowed", "qemu-x86_64"],
            capture_output=True,
            text=True
        )
        self.assertEqual(res.returncode, 1)
        self.assertIn("false", res.stdout)

    def test_canonical_output_dirs_match_registry(self):
        self.assertTrue(target_registry.get_target_output_dir("qemu-x86_64").endswith(os.path.join("out", "qemu-x86_64")))
        self.assertTrue(target_registry.get_target_output_dir("qemu-aarch64").endswith(os.path.join("out", "qemu-aarch64")))
        self.assertTrue(target_registry.get_target_output_dir("android-host-arm64").endswith(os.path.join("out", "android-host-arm64")))
        self.assertTrue(target_registry.get_target_output_dir("oneplus-fajita").endswith(os.path.join("out", "oneplus-fajita")))

    def test_android_host_sdk_alignment_with_gradle(self):
        cfg = target_registry.load_target("android-host-arm64", warn=False)
        sdk_info = cfg.get("jni", cfg.get("android", {}))
        self.assertEqual(sdk_info.get("min_sdk"), 29, "target.toml min_sdk must align with build.gradle.kts minSdk=29")
        self.assertEqual(sdk_info.get("target_sdk"), 35, "target.toml target_sdk must align with build.gradle.kts targetSdk=35")

    def test_build_scripts_agree_on_target_output_dirs(self):
        import importlib.util
        sys.path.insert(0, os.path.join(TOP, "build"))
        import mkinitramfs
        import mkdisk

        spec = importlib.util.spec_from_file_location("qemu_smoke", os.path.join(TOP, "build", "qemu-smoke.py"))
        qemu_smoke = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(qemu_smoke)

        # mkinitramfs configs must point to canonical target directories
        self.assertEqual(mkinitramfs.ARCH_CONFIGS["x86_64"]["out_dir"], target_registry.get_target_output_dir("qemu-x86_64"))
        self.assertEqual(mkinitramfs.ARCH_CONFIGS["aarch64"]["out_dir"], target_registry.get_target_output_dir("qemu-aarch64"))

        # qemu_smoke defaults must point to canonical target directories
        self.assertEqual(qemu_smoke.ARCH_DEFAULTS["x86_64"]["out_dir"], target_registry.get_target_output_dir("qemu-x86_64"))
        self.assertEqual(qemu_smoke.ARCH_DEFAULTS["aarch64"]["out_dir"], target_registry.get_target_output_dir("qemu-aarch64"))

        # mkdisk default must point to canonical x86_64 output directory
        self.assertEqual(mkdisk.OUT, target_registry.get_target_output_dir("qemu-x86_64"))


if __name__ == "__main__":
    unittest.main()
