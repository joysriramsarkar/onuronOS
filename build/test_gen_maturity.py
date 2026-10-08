#!/usr/bin/env python3
"""Tests for build/gen-maturity.py.

Covers generation, README drift detection, evidence/tier validation, and the
bidirectional registry <-> maturity.toml agreement that keeps the README table
from claiming a simulated screen is real (or vice versa).
"""
import importlib.util
import contextlib
import io
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(HERE)
SCRIPT = os.path.join(HERE, "gen-maturity.py")

SPEC = importlib.util.spec_from_file_location("gen_maturity", SCRIPT)
gen_maturity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gen_maturity)


SIM_RS = """// fixture
pub const BADGE: &str = "[SIMULATED]";
pub const SIMULATED_SCREENS: &[(&str, &str)] = &[
{rows}];
"""

README_SEED = """# Title

<!-- BEGIN GENERATED: maturity -->
stale table
<!-- END GENERATED: maturity -->

## After the table
"""

KERNEL_ENTRY = """[[entries]]
name = "Kernel"
tier = "functional-prototype"
details = "real kernel"
evidence = ["kernel.rs"]
simulated = false
"""

HOME_ENTRY = """[[entries]]
name = "Home"
tier = "functional-prototype"
details = "app launcher"
evidence = ["kernel.rs"]
simulated = true
ui_screens = ["home"]
"""


def sim_rs(keys=("home",)):
    rows = "".join(f'    ("{k}", "demo"),\n' for k in keys)
    return SIM_RS.format(rows=rows)


def toml(entries):
    return "\n".join(entries)


class GenMaturityTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = self._tmp.name
        os.makedirs(os.path.join(self.root, "docs"))
        os.makedirs(os.path.join(self.root, "shell", "src"))
        self.write("kernel.rs", "// real evidence\n")
        self.write("README.md", README_SEED)
        self.write("docs/maturity.toml", toml([KERNEL_ENTRY, HOME_ENTRY]))
        self.write("shell/src/simulated.rs", sim_rs(["home"]))
        self.paths = gen_maturity.default_paths(self.root)

    def tearDown(self):
        self._tmp.cleanup()

    # -- helpers ---------------------------------------------------------
    def write(self, rel, content):
        path = os.path.join(self.root, rel.replace("/", os.sep))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(content)

    def _read_readme(self):
        with open(os.path.join(self.root, "README.md"), encoding="utf-8") as f:
            return f.read()

    def regenerate(self):
        content = gen_maturity.generate(self.paths)
        self.write("README.md", content)
        return content

    # -- tests -----------------------------------------------------------
    def test_generate_marks_only_simulated_screens(self):
        content = self.regenerate()
        self.assertIn(gen_maturity.BEGIN, content)
        self.assertIn(gen_maturity.END, content)
        self.assertIn("| **Home** |", content)
        self.assertIn("**[SIMULATED]**", content)
        home_line = [l for l in content.splitlines() if "**Home**" in l][0]
        kernel_line = [l for l in content.splitlines() if "**Kernel**" in l][0]
        self.assertIn("**[SIMULATED]**", home_line)
        self.assertNotIn("**[SIMULATED]**", kernel_line)

    def test_check_passes_after_regeneration(self):
        self.regenerate()
        ok, message = gen_maturity.check(self.paths)
        self.assertTrue(ok, message)

    def test_check_detects_readme_drift(self):
        self.regenerate()
        self.write(
            "README.md",
            self._read_readme().replace("**[SIMULATED]**", "_tampered_"),
        )
        ok, message = gen_maturity.check(self.paths)
        self.assertFalse(ok)
        self.assertIn("stale", message)
        # A unified diff should point at the tampered line.
        self.assertIn("tampered", message)

    def test_missing_evidence_is_rejected(self):
        self.write(
            "docs/maturity.toml",
            toml([
                '[[entries]]\nname = "Ghost"\ntier = "stub"\n'
                'details = "x"\nevidence = ["does/not/exist.rs"]\nsimulated = false\n',
            ]),
        )
        with self.assertRaises(ValueError) as ctx:
            gen_maturity.generate(self.paths)
        self.assertIn("does not exist", str(ctx.exception))

    def test_empty_evidence_is_rejected(self):
        self.write(
            "docs/maturity.toml",
            toml(['[[entries]]\nname = "NoEvidence"\ntier = "stub"\n'
                  'details = "x"\nevidence = []\nsimulated = false\n']),
        )
        with self.assertRaises(ValueError) as ctx:
            gen_maturity.generate(self.paths)
        self.assertIn("evidence", str(ctx.exception).lower())

    def test_invalid_tier_is_rejected(self):
        self.write(
            "docs/maturity.toml",
            toml(['[[entries]]\nname = "Bad"\ntier = "totally-made-up"\n'
                  'details = "x"\nevidence = ["kernel.rs"]\nsimulated = false\n']),
        )
        with self.assertRaises(ValueError) as ctx:
            gen_maturity.generate(self.paths)
        self.assertIn("tier", str(ctx.exception))

    def test_phantom_screen_claim_is_rejected(self):
        # toml claims a screen the Rust registry does not contain.
        self.write(
            "docs/maturity.toml",
            toml([KERNEL_ENTRY,
                  '[[entries]]\nname = "Phantom"\ntier = "stub"\n'
                  'details = "x"\nevidence = ["kernel.rs"]\nsimulated = true\n'
                  'ui_screens = ["phantom"]\n']),
        )
        with self.assertRaises(ValueError) as ctx:
            gen_maturity.generate(self.paths)
        self.assertIn("phantom", str(ctx.exception))

    def test_orphan_registry_screen_is_rejected(self):
        # Registry contains a screen no toml entry claims.
        self.write("shell/src/simulated.rs", sim_rs(["home", "orphan"]))
        with self.assertRaises(ValueError) as ctx:
            gen_maturity.generate(self.paths)
        self.assertIn("orphan", str(ctx.exception))

    def test_ui_screens_without_simulated_flag_is_rejected(self):
        self.write(
            "docs/maturity.toml",
            toml([KERNEL_ENTRY,
                  '[[entries]]\nname = "Home"\ntier = "stub"\n'
                  'details = "x"\nevidence = ["kernel.rs"]\nsimulated = false\n'
                  'ui_screens = ["home"]\n']),
        )
        with self.assertRaises(ValueError) as ctx:
            gen_maturity.generate(self.paths)
        self.assertIn("simulated", str(ctx.exception))

    def test_main_check_exit_codes(self):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(gen_maturity.main(["--check", "--repo", self.root]), 1)
            self.assertEqual(gen_maturity.main(["--repo", self.root]), 0)
            self.assertEqual(gen_maturity.main(["--check", "--repo", self.root]), 0)

    def test_apply_to_readme_inserts_markers_when_absent(self):
        body = "# Title\n\n" + gen_maturity.TABLE_HEADER + "\n|---|---|---|\n| x | y | z |\n\n## Tail\n"
        block = gen_maturity.BEGIN + "\nGENERATED\n" + gen_maturity.END
        out = gen_maturity.apply_to_readme(body, block)
        self.assertIn(gen_maturity.BEGIN, out)
        self.assertIn(gen_maturity.END, out)
        self.assertIn("GENERATED", out)
        self.assertIn("## Tail", out)
        self.assertNotIn("| x | y | z |", out)

    def test_real_repo_check_is_fresh(self):
        """The checked-in README must match docs/maturity.toml."""
        res = subprocess.run(
            [sys.executable, SCRIPT, "--check"],
            cwd=REPO_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)


if __name__ == "__main__":
    unittest.main()