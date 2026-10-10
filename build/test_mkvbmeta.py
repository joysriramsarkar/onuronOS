"""Unit tests for cryptographic Verified Boot VBMeta tool (Ed25519)."""
import importlib.util
import os
import sys
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(__file__), "mkvbmeta.py")
SPEC = importlib.util.spec_from_file_location("mkvbmeta", SCRIPT)
mkvbmeta = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mkvbmeta)
create_vbmeta = mkvbmeta.create_vbmeta
verify_vbmeta = mkvbmeta.verify_vbmeta


class VBMetaTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.TemporaryDirectory()
        self.img = os.path.join(self.tmpdir.name, "system.img")
        with open(self.img, "wb") as f:
            f.write(b"SAMPLE_IMAGE_DATA_1234567890" * 100)
        self.meta = os.path.join(self.tmpdir.name, "vbmeta.img")

    def tearDown(self):
        self.tmpdir.cleanup()

    def test_create_and_verify_success(self):
        self.assertTrue(create_vbmeta(self.img, self.meta, partition_name="system"))
        self.assertTrue(os.path.isfile(self.meta))
        self.assertTrue(verify_vbmeta(self.img, self.meta))

    def test_tamper_detection(self):
        self.assertTrue(create_vbmeta(self.img, self.meta, partition_name="system"))
        with open(self.img, "r+b") as f:
            f.seek(5)
            f.write(b"X")
        self.assertFalse(verify_vbmeta(self.img, self.meta))

    def test_missing_image_fails_closed(self):
        missing = os.path.join(self.tmpdir.name, "nonexistent.img")
        self.assertFalse(create_vbmeta(missing, self.meta))


if __name__ == "__main__":
    unittest.main()
