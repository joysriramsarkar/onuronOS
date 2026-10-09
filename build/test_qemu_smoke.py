"""Unit tests for the automated QEMU boot smoke test harness."""
import importlib.util
import os
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location("qemu_smoke", os.path.join(HERE, "qemu-smoke.py"))
qemu_smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(qemu_smoke)


class QemuSmokeTests(unittest.TestCase):
    def test_build_qemu_cmd_structure(self):
        cmd = qemu_smoke.build_qemu_cmd(
            qemu_bin="qemu-system-x86_64",
            kernel_path="/boot/vmlinuz",
            initrd_path="/boot/initrd.cpio.gz",
            memory_mb=2048,
            smp=4,
        )
        self.assertEqual(cmd[0], "qemu-system-x86_64")
        self.assertIn("-m", cmd)
        self.assertIn("2048", cmd)
        self.assertIn("-smp", cmd)
        self.assertIn("4", cmd)
        self.assertIn("-kernel", cmd)
        self.assertIn("/boot/vmlinuz", cmd)
        self.assertIn("-initrd", cmd)
        self.assertIn("/boot/initrd.cpio.gz", cmd)
        self.assertIn("-display", cmd)
        self.assertIn("none", cmd)
        self.assertIn("-serial", cmd)
        self.assertIn("stdio", cmd)

    def test_check_boot_log_success(self):
        log = """
[    0.000000] Linux version 6.6.110-0-lts
[    0.113000] Freeing unused kernel memory: 2048K
[  OK  ] Early virtual filesystems mounted (/proc, /sys, /dev, /run, /tmp)
[  OK  ] Core services verified healthy (7/7 active)
[  OK  ] Onuron OS boot completed in 113.20 ms (7 services active, core services verified healthy)
"""
        status, details = qemu_smoke.check_boot_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("core services verified healthy", details)

    def test_check_boot_log_core_failure(self):
        log = """
[    0.000000] Linux version 6.6.110-0-lts
[ FAIL ] Core service 'nild' failed to start or crashed!
[ FATAL ] Onuron OS boot failed: core services not operational
"""
        status, details = qemu_smoke.check_boot_log(log)
        self.assertEqual(status, "PANIC")
        self.assertIn("core services not operational", details)

    def test_check_boot_log_kernel_panic(self):
        log = """
[    0.000000] Linux version 6.6.110-0-lts
[    0.045000] Kernel panic - not syncing: Fatal exception
[    0.046000] CPU: 0 PID: 1 Comm: swapper/0
"""
        status, details = qemu_smoke.check_boot_log(log)
        self.assertEqual(status, "PANIC")
        self.assertIn("panic", details)

    def test_check_boot_log_vfs_mount_failure(self):
        log = """
[    0.000000] Linux version 6.6.110-0-lts
[    0.052000] VFS: Unable to mount root fs on unknown-block(0,0)
"""
        status, details = qemu_smoke.check_boot_log(log)
        self.assertEqual(status, "PANIC")
        self.assertIn("Root filesystem", details)

    def test_check_boot_log_in_progress(self):
        log = """
[    0.000000] Linux version 6.6.110-0-lts
[    0.012000] Command line: console=ttyS0 init=/init
"""
        status, _ = qemu_smoke.check_boot_log(log)
        self.assertEqual(status, "IN_PROGRESS")

    def test_run_smoke_test_allow_skip_when_missing(self):
        # Should gracefully return True when allow_skip=True and files don't exist
        res = qemu_smoke.run_smoke_test(
            kernel="/nonexistent/vmlinuz",
            initrd="/nonexistent/initrd",
            allow_skip=True,
        )
        self.assertTrue(res)


if __name__ == "__main__":
    unittest.main()
