#!/usr/bin/env python3
"""Regenerate the README "Subsystem Maturity & Reality" table from docs/maturity.toml.

The generated table lives between two markers in README.md:

    <!-- BEGIN GENERATED: maturity -->
    ...
    <!-- END GENERATED: maturity -->

Checks performed (both in normal and --check mode):
  * every `evidence` path in docs/maturity.toml exists;
  * the `ui_screens` keys in docs/maturity.toml match the SIMULATED_SCREENS
    registry in shell/src/simulated.rs in both directions (no orphan screens and
    no phantom claims);
  * every `tier` is one of the recognised values.

--check exits non-zero (with a unified diff) when README.md is stale.

Usage:
    python build/gen-maturity.py            # rewrite README.md
    python build/gen-maturity.py --check    # verify only (CI)
"""
import argparse
import difflib
import os
import re
import sys

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
    import tomli as tomllib  # type: ignore

BEGIN = "<!-- BEGIN GENERATED: maturity -->"
END = "<!-- END GENERATED: maturity -->"
TABLE_HEADER = "| Subsystem / Feature | Maturity | Details & Reality |"

# tier -> (emoji, human label) matching the README legend.
TIERS = {
    "production": ("🟢", "Production-ready"),
    "functional-prototype": ("🔵", "Functional prototype"),
    "experimental": ("🟡", "Experimental"),
    "stub": ("🟠", "Stub / simulated"),
    "not-implemented": ("🔴", "Not implemented"),
}

SIMULATED_MARKER = "**[SIMULATED]**"


def repo_root():
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def default_paths(root=None):
    root = root or repo_root()
    return {
        "readme": os.path.join(root, "README.md"),
        "toml": os.path.join(root, "docs", "maturity.toml"),
        "sim": os.path.join(root, "shell", "src", "simulated.rs"),
    }


def load_entries(toml_path):
    with open(toml_path, "rb") as f:
        data = tomllib.load(f)
    return data.get("entries", [])


def load_registry(sim_path):
    """Return the set of screen keys declared in SIMULATED_SCREENS.

    Only the body of the `SIMULATED_SCREENS` constant is scanned so unrelated
    string literals elsewhere in the file cannot leak in.
    """
    keys = set()
    if not os.path.exists(sim_path):
        return keys
    with open(sim_path, "r", encoding="utf-8") as f:
        content = f.read()
    start = content.find("SIMULATED_SCREENS")
    if start == -1:
        return keys
    end = content.find("];", start)
    body = content[start:] if end == -1 else content[start:end]
    for match in re.finditer(r'\("([A-Za-z0-9_]+)",\s*"', body):
        keys.add(match.group(1))
    return keys


def validate(entries, registry, root):
    errors = []
    for entry in entries:
        name = entry.get("name", "<unnamed>")
        tier = entry.get("tier")
        if tier not in TIERS:
            errors.append(f"{name}: invalid tier `{tier}` (expected one of {sorted(TIERS)})")
        for rel in entry.get("evidence", []):
            full = os.path.normpath(os.path.join(root, rel))
            if not os.path.exists(full):
                errors.append(f"{name}: evidence path does not exist: {rel}")
        if not entry.get("evidence"):
            errors.append(f"{name}: at least one evidence path is required")
        for screen in entry.get("ui_screens", []):
            if screen not in registry:
                errors.append(
                    f"{name}: ui_screens key `{screen}` is not in shell/src/simulated.rs"
                )
        if entry.get("ui_screens") and not entry.get("simulated"):
            errors.append(f"{name}: has ui_screens but simulated is not true")

    claimed = set()
    for entry in entries:
        if entry.get("simulated"):
            claimed.update(entry.get("ui_screens", []))
    for screen in sorted(registry - claimed):
        errors.append(f"shell/src/simulated.rs key `{screen}` is not claimed by any maturity.toml entry")
    return errors


def render_table(entries):
    lines = [TABLE_HEADER, "|---|---|---|"]
    for entry in entries:
        tier = entry["tier"]
        emoji, label = TIERS[tier]
        details = entry.get("details", "")
        if entry.get("simulated") and SIMULATED_MARKER not in details:
            details = f"{details} {SIMULATED_MARKER}" if details else SIMULATED_MARKER
        lines.append(f"| **{entry['name']}** | {emoji} {label} | {details} |")
    return "\n".join(lines)


def render_block(entries):
    return f"{BEGIN}\n{render_table(entries)}\n{END}"


def apply_to_readme(content, block):
    marker = re.compile(re.escape(BEGIN) + r".*?" + re.escape(END), re.DOTALL)
    if BEGIN in content and END in content:
        return marker.sub(lambda _: block, content, count=1)

    # No markers: replace the existing hand-written table so the block can be
    # inserted without losing the surrounding legend prose.
    table = re.compile(
        re.escape(TABLE_HEADER) + r".*?(?=\n(?:##|\Z))",
        re.DOTALL,
    )
    if table.search(content):
        return table.sub(block + "\n", content, count=1)

    raise ValueError("could not find the maturity table or generation markers in README.md")


def generate(paths):
    entries = load_entries(paths["toml"])
    registry = load_registry(paths["sim"])
    # Evidence paths are repo-relative, i.e. relative to the directory that
    # contains README.md / docs/ — not relative to docs/ itself.
    errors = validate(entries, registry, os.path.dirname(os.path.abspath(paths["readme"])))
    if errors:
        raise ValueError("maturity data is invalid:\n  - " + "\n  - ".join(errors))
    with open(paths["readme"], "r", encoding="utf-8") as f:
        content = f.read()
    return apply_to_readme(content, render_block(entries))


def check(paths):
    """Return (ok, message). `message` is a diff when stale."""
    expected = generate(paths)
    with open(paths["readme"], "r", encoding="utf-8") as f:
        actual = f.read()
    if actual == expected:
        return True, "README is up to date."
    diff = "".join(
        difflib.unified_diff(
            actual.splitlines(keepends=True),
            expected.splitlines(keepends=True),
            fromfile="README.md",
            tofile="README.md.generated",
        )
    )
    return False, "README.md is stale; run `python build/gen-maturity.py`.\n" + diff


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify README is up to date")
    parser.add_argument("--repo", default=None, help="repository root (defaults to this checkout)")
    args = parser.parse_args(argv)
    paths = default_paths(args.repo)

    try:
        expected = generate(paths)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if args.check:
        ok, message = check(paths)
        if not ok:
            print(message, file=sys.stderr)
            return 1
        print(message)
        return 0

    with open(paths["readme"], "w", encoding="utf-8", newline="\n") as f:
        f.write(expected)
    print("README.md regenerated.")
    return 0


if __name__ == "__main__":
    sys.exit(main())