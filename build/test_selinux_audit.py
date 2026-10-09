"""Unit tests for the SELinux policy auditor."""
import os
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TOP = os.path.dirname(HERE)
POLICY_DIR = os.path.join(TOP, "security", "selinux", "policy")


def find_bash():
    for candidate in [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files\Git\usr\bin\bash.exe",
        "bash",
    ]:
        try:
            res = subprocess.run([candidate, "--version"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            if res.returncode == 0:
                return candidate
        except Exception:
            pass
    return None


class SelinuxAuditTests(unittest.TestCase):
    def test_audit_script_passes_clean_policy(self):
        bash_bin = find_bash()
        if not bash_bin:
            self.skipTest("bash not available")
        audit_sh = os.path.join(TOP, "security", "selinux", "ci", "audit.sh")
        res = subprocess.run([bash_bin, audit_sh], cwd=TOP, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.assertEqual(res.returncode, 0, f"audit.sh failed: {res.stderr}\n{res.stdout}")
        self.assertIn("0 neverallow violations detected", res.stdout)

    def test_audit_script_catches_violation(self):
        bash_bin = find_bash()
        if not bash_bin:
            self.skipTest("bash not available")
        # Create a temporary violation file in policy dir
        bad_file = os.path.join(POLICY_DIR, "99-test-violation.cil")
        try:
            with open(bad_file, "w") as f:
                f.write("(allow nilos_app_t nilinit_t (process (transition)))\n")
            audit_sh = os.path.join(TOP, "security", "selinux", "ci", "audit.sh")
            res = subprocess.run([bash_bin, audit_sh], cwd=TOP, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertNotEqual(res.returncode, 0, "audit.sh should fail when neverallow is violated")
            self.assertIn("CRITICAL VIOLATION", res.stderr)
        finally:
            if os.path.exists(bad_file):
                os.remove(bad_file)


if __name__ == "__main__":
    unittest.main()
