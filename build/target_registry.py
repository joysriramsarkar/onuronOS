#!/usr/bin/env python3
"""
build/target_registry.py — Canonical Target Registry for OnuronOS / NilOS

Loads target definitions from targets/<target>/target.toml adhering to ADR-0002.
Translates legacy target aliases with deprecation warnings and enforces
fail-closed target matching across build, package, test, and flash tools.
"""

import os
import sys

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGETS_DIR = os.path.join(TOP, "targets")

CANONICAL_TARGETS = [
    "qemu-x86_64",
    "qemu-aarch64",
    "android-host-arm64",
    "oneplus-fajita",
]

ALIASES = {
    "x86_64-generic": "qemu-x86_64",
    "x86_64": "qemu-x86_64",
    "amd64": "qemu-x86_64",
    "aarch64-generic": "qemu-aarch64",
    "aarch64-qemu": "qemu-aarch64",
    "arm64-generic": "qemu-aarch64",
    "aarch64": "qemu-aarch64",
    "arm64": "qemu-aarch64",
    "fajita": "oneplus-fajita",
    "s25": "android-host-arm64",
    "android-host": "android-host-arm64",
}


def _parse_toml(filepath):
    """Parse a basic TOML file into nested dict without external dependencies."""
    if sys.version_info >= (3, 11):
        import tomllib
        with open(filepath, "rb") as f:
            return tomllib.load(f)

    # Fallback basic TOML parser for sections and key=value strings/numbers/bools/lists
    data = {}
    current_section = data
    with open(filepath, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("[") and line.endswith("]"):
                sec_name = line[1:-1].strip()
                parts = sec_name.split(".")
                curr = data
                for p in parts:
                    if p not in curr:
                        curr[p] = {}
                    curr = curr[p]
                current_section = curr
            elif "=" in line:
                k, v = line.split("=", 1)
                k = k.strip()
                v = v.strip()
                # parse value
                if v.startswith('"') and v.endswith('"'):
                    val = v[1:-1]
                elif v in ("true", "True"):
                    val = True
                elif v in ("false", "False"):
                    val = False
                elif v.startswith("[") and v.endswith("]"):
                    # simple list of strings
                    items = [item.strip().strip('"').strip("'") for item in v[1:-1].split(",") if item.strip()]
                    val = items
                else:
                    try:
                        val = int(v)
                    except ValueError:
                        val = v
                current_section[k] = val
    return data


def resolve_target(name, warn=True):
    """
    Resolve a target identifier or alias to its canonical target ID.
    Logs a deprecation warning if a legacy alias is used.
    Raises ValueError if unknown target.
    """
    raw = (name or "").strip().lower()
    if raw in CANONICAL_TARGETS:
        return raw

    if raw in ALIASES:
        canonical = ALIASES[raw]
        if warn:
            print(f"[DEPRECATION] Target alias '{raw}' is deprecated. Using canonical target '{canonical}'.",
                  file=sys.stderr)
        return canonical

    raise ValueError(
        f"Unknown target '{name}'. Available canonical targets: {', '.join(CANONICAL_TARGETS)}"
    )


def load_target(name, warn=True):
    """
    Load target configuration dictionary from targets/<canonical>/target.toml.
    """
    canonical = resolve_target(name, warn=warn)
    target_toml = os.path.join(TARGETS_DIR, canonical, "target.toml")
    if not os.path.isfile(target_toml):
        raise FileNotFoundError(f"Target specification file not found: {target_toml}")

    return _parse_toml(target_toml)


def get_target_output_dir(name):
    """Get absolute path to target output directory."""
    cfg = load_target(name, warn=False)
    rel_out = cfg.get("target", {}).get("output_dir", f"out/{resolve_target(name, warn=False)}")
    return os.path.normpath(os.path.join(TOP, rel_out))


def get_target_triple(name):
    """Get Rust target triple for target."""
    cfg = load_target(name, warn=False)
    return cfg.get("target", {}).get("target_triple")


def is_flashing_allowed(name):
    """Check if direct physical flashing is permitted for this target profile."""
    cfg = load_target(name, warn=False)
    return cfg.get("target", {}).get("flashing_allowed", False)


if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] == "list":
        print("Available OnuronOS Canonical Targets:")
        for t in CANONICAL_TARGETS:
            cfg = load_target(t, warn=False)
            tinfo = cfg.get("target", {})
            print(f"  - {t}: {tinfo.get('display_name', '')} ({tinfo.get('architecture', '')})")
    elif sys.argv[1] == "resolve" and len(sys.argv) > 2:
        try:
            print(resolve_target(sys.argv[2]))
        except ValueError as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)
    elif sys.argv[1] == "info" and len(sys.argv) > 2:
        try:
            cfg = load_target(sys.argv[2])
            import json
            print(json.dumps(cfg, indent=2))
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)
    elif sys.argv[1] == "output-dir" and len(sys.argv) > 2:
        try:
            print(get_target_output_dir(sys.argv[2]))
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)
    elif sys.argv[1] == "is-flashing-allowed" and len(sys.argv) > 2:
        try:
            allowed = is_flashing_allowed(sys.argv[2])
            print("true" if allowed else "false")
            sys.exit(0 if allowed else 1)
        except Exception as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(2)
    else:
        print("Usage: target_registry.py [list | resolve <name> | output-dir <name> | info <name> | is-flashing-allowed <name>]")
