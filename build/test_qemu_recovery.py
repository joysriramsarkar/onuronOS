#!/usr/bin/env python3
"""Unit tests for the automated QEMU recovery-path test harness."""
import importlib.util
import os
import struct
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location(
    "qemu_recovery_test",
    os.path.join(HERE, "qemu-recovery-test.py"),
)
qemu_recovery = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(qemu_recovery)


class QemuRecoveryCmdTests(unittest.TestCase):
    def _cmd(self, phase, arch="x86_64"):
        return qemu_recovery.build_qemu_cmd(
            qemu_bin="qemu-system-x86_64",
            kernel_path="/boot/vmlinuz",
            initrd_path="/boot/initrd.cpio.gz",
            disk_path="/tmp/test_data.img",
            phase=phase,
            arch=arch,
        )

    def test_flag_phase_requests_recovery_on_cmdline(self):
        cmd = self._cmd("flag")
        append = cmd[cmd.index("-append") + 1]
        self.assertIn("onuron.recovery=1", append)
        self.assertIn("console=ttyS0", append)
        self.assertNotIn("onuron.readiness_ms", append)

    def test_counter_and_reset_phases_boot_normally_then_recover(self):
        for phase in ("counter", "reset"):
            cmd = self._cmd(phase)
            append = cmd[cmd.index("-append") + 1]
            self.assertNotIn("onuron.recovery", append,
                              f"{phase} must trigger recovery via the seeded counter, not the flag")
            self.assertNotIn("onuron.readiness_ms", append)

    def test_full_boot_phases_carry_the_readiness_budget(self):
        for phase in ("reboot", "corrupt"):
            cmd = self._cmd(phase)
            append = cmd[cmd.index("-append") + 1]
            self.assertIn("onuron.readiness_ms=45000", append)
            self.assertNotIn("onuron.recovery=1", append)

    def test_x86_64_pins_modern_cpu_and_attaches_the_disk(self):
        cmd = self._cmd("flag")
        self.assertIn("-cpu", cmd)
        self.assertIn("max", cmd)
        self.assertIn("-no-reboot", cmd)
        self.assertIn("-serial", cmd)
        self.assertIn("stdio", cmd)
        self.assertIn("-drive", cmd)
        self.assertIn("file=/tmp/test_data.img,format=raw,if=none,id=vda_disk,cache=writeback", cmd)
        self.assertIn("virtio-blk-pci,drive=vda_disk,id=vda", cmd)

    def test_aarch64_uses_virt_machine_and_serial_console(self):
        cmd = qemu_recovery.build_qemu_cmd(
            qemu_bin="qemu-system-aarch64",
            kernel_path="/boot/vmlinuz-arm",
            initrd_path="/boot/initrd-arm.cpio.gz",
            disk_path="/tmp/test_arm.img",
            phase="counter",
            arch="aarch64",
        )
        self.assertEqual(cmd[0], "qemu-system-aarch64")
        self.assertIn("-M", cmd)
        self.assertIn("virt", cmd)
        self.assertIn("cortex-a57", cmd)
        append = cmd[cmd.index("-append") + 1]
        self.assertIn("console=ttyAMA0", append)
        self.assertNotIn("onuron.recovery=1", append)

    def test_menu_input_payloads(self):
        self.assertEqual(qemu_recovery.STDIN_PAYLOADS["flag"], b"q\n")
        self.assertEqual(qemu_recovery.STDIN_PAYLOADS["counter"], b"q\n")
        self.assertEqual(qemu_recovery.STDIN_PAYLOADS["reset"], b"3\nYES\n")
        self.assertIsNone(qemu_recovery.STDIN_PAYLOADS["corrupt"])
        self.assertIsNone(qemu_recovery.STDIN_PAYLOADS["reboot"])
        # Rebooting phases must wait for the guest reset; full boots must not.
        self.assertTrue(qemu_recovery.EXPECT_EXIT["flag"])
        self.assertTrue(qemu_recovery.EXPECT_EXIT["counter"])
        self.assertTrue(qemu_recovery.EXPECT_EXIT["reset"])
        self.assertFalse(qemu_recovery.EXPECT_EXIT["corrupt"])
        self.assertFalse(qemu_recovery.EXPECT_EXIT["reboot"])


class QemuRecoveryDiskTests(unittest.TestCase):
    def test_counter_seed_dir_layout(self):
        with tempfile.TemporaryDirectory(prefix="onuron_seed_") as tmp:
            root = qemu_recovery.make_counter_seed_dir(os.path.join(tmp, "seed"), failed_boots=2)
            boot_count = os.path.join(root, "system", "boot_count")
            self.assertTrue(os.path.isfile(boot_count))
            with open(boot_count) as f:
                self.assertEqual(f.read(), "2")

    def test_counter_seed_dir_overwrites_previous_value(self):
        with tempfile.TemporaryDirectory(prefix="onuron_seed_") as tmp:
            root = os.path.join(tmp, "seed")
            qemu_recovery.make_counter_seed_dir(root, failed_boots=1)
            qemu_recovery.make_counter_seed_dir(root, failed_boots=2)
            with open(os.path.join(root, "system", "boot_count")) as f:
                self.assertEqual(f.read(), "2")

    def test_corrupt_image_has_no_ext_superblock_magic(self):
        with tempfile.TemporaryDirectory(prefix="onuron_corrupt_") as tmp:
            path = os.path.join(tmp, "corrupt.img")
            qemu_recovery.build_corrupt_image(path, size_mb=8)
            self.assertEqual(os.path.getsize(path), 8 * 1024 * 1024)
            with open(path, "rb") as f:
                # ext superblock magic sits at offset 1024 + 56.
                f.seek(1024 + 56)
                magic = struct.unpack("<H", f.read(2))[0]
            self.assertNotEqual(magic, 0xEF53)
            # The same validity check mkdisk uses must reject this image.
            try:
                import mkdisk
            except ImportError:
                sys_path = os.path.join(HERE)
                import sys
                if sys_path not in sys.path:
                    sys.path.insert(0, sys_path)
                import mkdisk
            self.assertFalse(mkdisk.check_existing_image_validity(path, expected_size_mb=8))


class QemuRecoveryLogEvaluationTests(unittest.TestCase):
    def test_flag_log_success(self):
        log = f"""
[  OK  ] Persistent storage mounted: /dev/vda → /data
[ WARN ] Entering recovery mode: recovery requested on the kernel command line
=========================================================
                 Onuron Recovery System
=========================================================
1) Reboot System Normal
q) Quit
Select option: Leaving recovery. The device will reboot.
"""
        status, details = qemu_recovery.check_flag_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("explicit recovery flag", details)

    def test_flag_log_fails_when_binary_is_missing(self):
        log = f"""
[ WARN ] Entering recovery mode: recovery requested on the kernel command line
[ WARN ] Could not start /usr/bin/nilrecovery: No such file or directory
"""
        status, details = qemu_recovery.check_flag_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("mkinitramfs bins list", details)

    def test_flag_log_fails_when_boot_ignores_the_flag(self):
        log = """
[  OK  ] Core service sockets verified ready
[  OK  ] Onuron OS boot completed in 1234.00 ms
"""
        status, details = qemu_recovery.check_flag_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("ignored onuron.recovery=1", details)

    def test_counter_log_success(self):
        log = f"""
[ WARN ] Entering recovery mode: 3 consecutive failed boots (limit 3)
                 Onuron Recovery System
Select option: Leaving recovery. The device will reboot.
"""
        status, details = qemu_recovery.check_counter_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("failed-boot counter triggered recovery", details)

    def test_counter_log_fails_when_seed_is_ignored(self):
        log = "[  OK  ] Onuron OS boot completed in 2000.00 ms\n"
        status, details = qemu_recovery.check_counter_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("did not trigger recovery", details)

    def test_reset_log_success(self):
        log = f"""
[ WARN ] Entering recovery mode: 3 consecutive failed boots (limit 3)
This permanently erases all user data. Type YES to confirm:
Wiping user data...
Could not run mkfs.ext4: No such file or directory — wiping in place instead
Factory reset complete.
"""
        status, details = qemu_recovery.check_reset_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("in-place wipe", details)

    def test_reset_log_fails_when_confirmation_is_refused(self):
        log = f"""
[ WARN ] Entering recovery mode: 3 consecutive failed boots (limit 3)
This permanently erases all user data. Type YES to confirm:
Factory reset cancelled (no confirmation).
"""
        status, details = qemu_recovery.check_reset_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("'YES' did not reach", details)

    def test_reset_log_fails_when_reset_reports_failure(self):
        log = f"""
[ WARN ] Entering recovery mode: 3 consecutive failed boots (limit 3)
Wiping user data...
Factory reset failed: could not remount /dev/vda after unmount
"""
        status, details = qemu_recovery.check_reset_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("reported failure", details)

    def test_reboot_log_success(self):
        log = """
[  OK  ] Persistent storage mounted: /dev/vda → /data
[  OK  ] Core service sockets verified ready
[  OK  ] Onuron OS boot completed in 3210.00 ms
"""
        status, details = qemu_recovery.check_reboot_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("post-reset boot mounted persistent storage", details)

    def test_reboot_log_fails_when_recovery_loops(self):
        log = "[ WARN ] Entering recovery mode: 3 consecutive failed boots (limit 3)\n"
        status, details = qemu_recovery.check_reboot_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("counter not cleared", details)

    def test_reboot_log_fails_on_ipc_denial(self):
        log = """
[  OK  ] Persistent storage mounted: /dev/vda → /data
[nilsd:auth] denied: uid 0 gid 0 pid 42 is not permitted to use 'notifyd'
[  OK  ] Onuron OS boot completed in 3000.00 ms
"""
        status, details = qemu_recovery.check_reboot_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("IPC policy", details)

    def test_corrupt_log_success(self):
        log = f"""
[ WARN ] CORRUPT DISK DETECTED: persistent block device(s) present but unmountable (/dev/vda: EINVAL) — mounting emergency tmpfs on /data
[  OK  ] Core service sockets verified ready
[  OK  ] Onuron OS boot completed in 2500.00 ms
"""
        status, details = qemu_recovery.check_corrupt_log(log)
        self.assertEqual(status, "SUCCESS")
        self.assertIn("emergency tmpfs", details)

    def test_corrupt_log_fails_on_silent_no_disk_fallback(self):
        log = f"""
[ WARN ] No persistent disk found — /data is tmpfs (ephemeral)
[  OK  ] Onuron OS boot completed in 2500.00 ms
"""
        status, details = qemu_recovery.check_corrupt_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("silent tmpfs fallback", details)

    def test_corrupt_log_fails_on_degraded_boot(self):
        log = f"""
[ WARN ] CORRUPT DISK DETECTED: persistent block device(s) present but unmountable — mounting emergency tmpfs on /data
[ WARN ] Onuron OS boot degraded: core readiness incomplete
"""
        status, details = qemu_recovery.check_corrupt_log(log)
        self.assertEqual(status, "FAIL")
        self.assertIn("degraded", details)

    def test_all_log_evaluators_fail_on_kernel_panic(self):
        panic = "[    0.045000] Kernel panic - not syncing: attempted to kill init!\n"
        for phase, evaluator in qemu_recovery.PHASE_EVALUATORS.items():
            status, details = evaluator(panic)
            self.assertEqual(status, "FAIL", phase)
            self.assertIn("panic", details)


if __name__ == "__main__":
    unittest.main()
