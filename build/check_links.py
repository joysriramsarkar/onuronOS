#!/usr/bin/env python3
"""
build/check_links.py — Markdown Link Checker for OnuronOS Documentation

Complies with Roadmap Section 40.3 & Section 40.7:
- Rejects absolute machine paths (e.g., file:///c:/... or c:\\...) in documentation.
- Validates all local relative markdown links across docs/ and repository root.
- Ensures all ADR cross-references point to real files.
"""

import os
import re
import sys

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Regex to capture markdown links: [text](target)
LINK_PATTERN = re.compile(r'\[([^\]]+)\]\(([^)]+)\)')


def check_markdown_file(filepath):
    """
    Check a single markdown file for broken or invalid links.
    Returns list of errors (empty if all links valid).
    """
    errors = []
    base_dir = os.path.dirname(os.path.abspath(filepath))

    try:
        with open(filepath, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()
    except Exception as e:
        return [f"Could not read {filepath}: {e}"]

    # Strip fenced code blocks (``` ... ```) and inline code (`...`) to avoid scanning illustrative snippets
    clean_content = re.sub(r'```.*?```', '', content, flags=re.DOTALL)
    clean_content = re.sub(r'`[^`\n]+`', '', clean_content)

    for match in LINK_PATTERN.finditer(clean_content):
        label, target = match.group(1), match.group(2).strip()

        # Ignore web URLs, mailto, and in-page anchor-only links
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue

        # Check for forbidden absolute host-specific links
        if target.startswith("file://") or re.match(r'^[a-zA-Z]:[\\/]', target):
            errors.append(
                f"{filepath}: Forbidden absolute host path in link '[{label}]({target})'. Use relative path."
            )
            continue

        # Strip fragment identifier (e.g., #section-header)
        file_part = target.split("#", 1)[0].strip()
        if not file_part:
            continue

        # Resolve relative target against the directory of the markdown file
        resolved = os.path.normpath(os.path.join(base_dir, file_part))

        if not os.path.exists(resolved):
            errors.append(
                f"{filepath}: Broken relative link '[{label}]({target})' -> '{resolved}' does not exist."
            )

    return errors


def check_directory(dir_path):
    """
    Recursively check all .md files in a directory.
    Returns (total_files_checked, all_errors).
    """
    total_files = 0
    all_errors = []

    for root, _, files in os.walk(dir_path):
        for f in files:
            if f.endswith(".md"):
                full_path = os.path.join(root, f)
                total_files += 1
                errors = check_markdown_file(full_path)
                all_errors.extend(errors)

    return total_files, all_errors


def main():
    paths_to_check = [
        os.path.join(TOP, "docs"),
        os.path.join(TOP, "README.md"),
    ]

    total_files = 0
    all_errors = []

    for p in paths_to_check:
        if os.path.isdir(p):
            n, errs = check_directory(p)
            total_files += n
            all_errors.extend(errs)
        elif os.path.isfile(p):
            total_files += 1
            errs = check_markdown_file(p)
            all_errors.extend(errs)

    print(f"[check-links] Checked {total_files} Markdown files.")

    if all_errors:
        print(f"\x1b[1;31m[ERROR] Found {len(all_errors)} link issue(s):\x1b[0m", file=sys.stderr)
        for err in all_errors:
            print(f"  - {err}", file=sys.stderr)
        return 1
    else:
        print("\x1b[1;32m[OK] All documentation links are relative and verified.\x1b[0m")
        return 0


if __name__ == "__main__":
    sys.exit(main())
