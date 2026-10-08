#!/usr/bin/env python3
"""Unit tests for the pure helpers in build/persistent-data-test.py.

Only the parsing/decision helpers are tested here so the suite runs without
e2fsprogs, QEMU, or a real block device.
"""
import importlib.util
import os
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(__file__), "persistent-data-test.py")
SPEC = importlib.util.spec_from_file_location("persistent_data_test", SCRIPT)
persistent_data_test = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(persistent_data_test)


class ParseLsTests(unittest.TestCase):
    def test_parses_names_and_drops_dot_entries(self):
        output = "2  3  4\n.  ..  config  sms\n"
        self.assertEqual(
            persistent_data_test.parse_ls_names(output),
            {"config", "sms"},
        )

    def test_parses_debugfs_ls_long_style(self):
        output = (
            " 2  40755 (2) 0 0 0 1-Jan-2024 00:00 .\n"
            " 2  40755 (2) 0 0 0 1-Jan-2024 00:00 ..\n"
            " 12 100644 (1) 0 0 0 1-Jan-2024 00:00 oobe_done\n"
            " 13 40755 (2) 0 0 0 1-Jan-2024 00:00 nested dir\n"
        )
        names = persistent_data_test.parse_ls_names(output)
        self.assertIn("oobe_done", names)
        self.assertNotIn(".", names)
        self.assertNotIn("..", names)

    def test_empty_output(self):
        self.assertEqual(persistent_data_test.parse_ls_names(""), set())


class CompareEntriesTests(unittest.TestCase):
    def test_reports_missing_and_unexpected(self):
        missing, unexpected = persistent_data_test.compare_entries(
            ["a", "b"], ["b", "c"]
        )
        self.assertEqual(missing, ["a"])
        self.assertEqual(unexpected, ["c"])

    def test_identical_sets_have_no_differences(self):
        missing, unexpected = persistent_data_test.compare_entries(
            ["a", "b"], ["a", "b"]
        )
        self.assertEqual(missing, [])
        self.assertEqual(unexpected, [])


class MaterializeTests(unittest.TestCase):
    def test_materialize_writes_known_state(self):
        with tempfile.TemporaryDirectory() as tmp:
            persistent_data_test.materialize(tmp)
            for rel, expected in persistent_data_test.KNOWN_FILES.items():
                path = os.path.join(tmp, rel.replace("/", os.sep))
                self.assertTrue(os.path.isfile(path), rel)
                with open(path, "rb") as f:
                    self.assertEqual(f.read(), expected)


if __name__ == "__main__":
    unittest.main()