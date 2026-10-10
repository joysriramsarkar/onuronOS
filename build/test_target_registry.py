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


if __name__ == "__main__":
    unittest.main()
