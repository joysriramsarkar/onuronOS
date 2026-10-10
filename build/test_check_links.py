#!/usr/bin/env python3
"""
build/test_check_links.py — Unit Tests for Markdown Link Checker
"""

import os
import shutil
import sys
import tempfile
import unittest

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(TOP, "build"))

import check_links


class TestCheckLinks(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.tmpdir, ignore_errors=True)

    def test_valid_relative_links_pass(self):
        doc1 = os.path.join(self.tmpdir, "doc1.md")
        doc2 = os.path.join(self.tmpdir, "doc2.md")
        with open(doc2, "w") as f:
            f.write("# Target Document")
        with open(doc1, "w") as f:
            f.write("Link to [doc2](./doc2.md) and [web](https://example.com).")

        errors = check_links.check_markdown_file(doc1)
        self.assertEqual(errors, [])

    def test_broken_relative_link_detected(self):
        doc = os.path.join(self.tmpdir, "broken.md")
        with open(doc, "w") as f:
            f.write("Broken link to [missing](./nonexistent.md).")

        errors = check_links.check_markdown_file(doc)
        self.assertEqual(len(errors), 1)
        self.assertIn("does not exist", errors[0])

    def test_forbidden_absolute_host_path_detected(self):
        doc = os.path.join(self.tmpdir, "absolute.md")
        with open(doc, "w") as f:
            f.write("Bad link [file](file:///c:/secret/path.md).")

        errors = check_links.check_markdown_file(doc)
        self.assertEqual(len(errors), 1)
        self.assertIn("Forbidden absolute host path", errors[0])

    def test_code_blocks_are_ignored(self):
        doc = os.path.join(self.tmpdir, "snippet.md")
        with open(doc, "w") as f:
            f.write("""# Snippet
```markdown
[illustrative link](nonexistent_example.md)
```
""")

        errors = check_links.check_markdown_file(doc)
        self.assertEqual(errors, [])

    def test_full_repository_docs_links_valid(self):
        """Run checker on official docs/ directory to ensure zero broken links."""
        docs_dir = os.path.join(TOP, "docs")
        total, errors = check_links.check_directory(docs_dir)
        self.assertGreater(total, 0)
        self.assertEqual(errors, [], f"Documentation has broken links: {errors}")


if __name__ == "__main__":
    unittest.main()
