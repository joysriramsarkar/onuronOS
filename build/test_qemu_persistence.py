#!/usr/bin/env python3
"""Unit tests for the automated QEMU reboot persistence test harness."""
import importlib.util
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location(
    "qemu_persistence_test",
    os.path.join(HERE, "qemu-persistence-test.py"),
)
qemu_persistence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(qemu_persistence)


class QemuPersistenceCmdTests(unittest.TestCase):
    def test_build_qemu_cmd_x86_64(self):
        cmd = qemu_persistence.build_qemu_cmd(
            qemu_bin="qemu-system-x86_64",
            kernel_path="/boot/vmlinuz",
            initrd_path="/boot/initrd.cpio.gz",
            disk_path="/tmp/test_data.img",
            phase="write",
            arch="x86_64",
        )
        self.assertEqual(cmd[0], "qemu-system-x86_64")
        self.assertIn("-kernel", cmd)
        self.assertIn("/boot/vmlinuz", cmd)
        self.assertIn("-initrd", cmd)
        self.assertIn("/boot/initrd.cpio.gz", cmd)
        self.assertIn("-drive", cmd)
        self.assertIn("file=/tmp/test_data.img,format=raw,if=none,id=vda_disk,cache=writeback", cmd)
        self.assertIn("-device", cmd)
        self.assertIn("virtio-blk-pci,drive=vda_disk,id=vda", cmd)
        append_str = cmd[cmd.index("-append") + 1]
        self.assertIn("onuron.test_persistence=write", append_str)
        self.assertIn("console=ttyS0", append_str)

    def test_build_qemu_cmd_aarch64(self):
        cmd = qemu_persistence.build_qemu_cmd(
            qemu_bin="qemu-system-aarch64",
            kernel_path="/boot/vmlinuz-arm",
            initrd_path="/boot/initrd-arm.cpio.gz",
            disk_path="/tmp/test_arm_data.img",
            phase="verify",
            arch="aarch64",
        )
        self.assertEqual(cmd[0], "qemu-system-aarch64")
        self.assertIn("-M", cmd)
        self.assertIn("virt", cmd)
        self.assertIn("-cpu", cmd)
        self.assertIn("cortex-a57", cmd)
        append_str = cmd[cmd.index("-append") + 1]
        self.assertIn("onuron.test_persistence=verify", append_str)
        self.assertIn("console=ttyAMA0", append_str)


class QemuPersistenceLogEvaluationTests(unittest.TestCase):
    def test_check_write_log_success(self):
        log = """
[  OK  ] Early virtual filesystems mounted (/proc, /sys, /dev, /run, /tmp)
[  OK  ] Persistent storage mounted: /dev/vda -> /data
[ INFO ] Executing automated persistence test: WRITE phase...
[  OK  ] Persistence marker written and synced to /data
[  OK  ] Persistence test WRITE phase completed successfully
[ INFO ] Initiating system poweroff
"""
        status, details = qemu_persistence.check_write_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("successfully written and synced", details)

    def test_check_write_log_fails_on_tmpfs(self):
        log = """
[  OK  ] Early virtual filesystems mounted (/proc, /sys, /dev, /run, /tmp)
[ WARN ] No persistent disk found — /data is tmpfs (ephemeral)
[ INFO ] Executing automated persistence test: WRITE phase...
[ FAIL ] Persistence test failed: /data is not mounted from real ext block device!
"""
        status, details = qemu_persistence.check_write_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("tmpfs fallback rejected", details)

    def test_check_write_log_fails_on_panic(self):
        log = """
[    0.045000] Kernel panic - not syncing: VFS: Unable to mount root fs
"""
        status, details = qemu_persistence.check_write_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("panic", details)

    def test_check_write_log_fails_on_write_error(self):
        log = """
[ INFO ] Executing automated persistence test: WRITE phase...
[ FAIL ] Failed writing persistence marker: Read-only file system (os error 30)
"""
        status, details = qemu_persistence.check_write_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("Write failure", details)

    def test_check_verify_log_success(self):
        log = """
[  OK  ] Persistent storage mounted: /dev/vda -> /data
[ INFO ] Executing automated persistence test: VERIFY phase...
[  OK  ] Persistence marker verified across reboot: payload matches byte-for-byte
[  OK  ] Persistence test VERIFY phase completed successfully
"""
        status, details = qemu_persistence.check_verify_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("payload matches byte-for-byte", details)

    def test_check_verify_log_fails_on_corrupt_payload(self):
        log = """
[  OK  ] Persistent storage mounted: /dev/vda -> /data
[ INFO ] Executing automated persistence test: VERIFY phase...
[ FAIL ] Persistence marker corrupt (read 12 bytes)
"""
        status, details = qemu_persistence.check_verify_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("corrupted", details)

    def test_check_verify_log_fails_on_missing_file(self):
        log = """
[  OK  ] Persistent storage mounted: /dev/vda -> /data
[ INFO ] Executing automated persistence test: VERIFY phase...
[ FAIL ] Persistence marker missing after reboot: No such file or directory (os error 2)
"""
        status, details = qemu_persistence.check_verify_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("missing after reboot", details)

    def test_check_verify_log_fails_on_tmpfs(self):
        log = """
[ WARN ] No persistent disk found — /data is tmpfs (ephemeral)
[ INFO ] Executing automated persistence test: VERIFY phase...
[ FAIL ] Persistence test verify failed: /data is tmpfs!
"""
        status, details = qemu_persistence.check_verify_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("tmpfs", details)


class QemuPersistenceSkipTests(unittest.TestCase):
    def test_allow_skip_when_images_missing(self):
        # When images are nonexistent and allow_skip=True, returns True cleanly
        result = qemu_persistence.run_persistence_test(
            kernel="/nonexistent/vmlinuz",
            initrd="/nonexistent/initrd",
            allow_skip=True,
        )
        self.assertTrue(result)


if __name__ == "__main__":
    unittest.main()
