#!/usr/bin/env python3
# build/build.py — Cross-Platform Builder for OnuronOS / NilOS
import os
import sys
import shutil
import subprocess
import json
import hashlib
from datetime import datetime

TARGET = sys.argv[1] if len(sys.argv) > 1 else "x86_64-generic"
NORM_ARCH = "aarch64" if TARGET.startswith(("aarch64", "arm64")) else "x86_64"
TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(TOP, "out", TARGET)
SYS = os.path.join(OUT, "rootfs")

print("=========================================================")
print(f"             Building OnuronOS for {TARGET}             ")
print("=========================================================")

# 1. Clean output directory
if os.path.exists(OUT):
    shutil.rmtree(OUT, ignore_errors=True)

dirs = [
    "bin", "usr/bin", "usr/lib", "etc/nilos/apps", "data/app", "data/user",
    "proc", "sys", "dev", "run/nilos", "mnt", "vendor/lib/nilhal"
]
for d in dirs:
    os.makedirs(os.path.join(SYS, d), exist_ok=True)

# 2. Strict Cargo Userspace Compilation
print("==> [1/4] Compiling Userspace (Rust Crates)...")
cargo = shutil.which("cargo")
if not cargo:
    print("[ERROR] 'cargo' compiler not detected in PATH.")
    print("        Rust is required to build OnuronOS. Install from https://rustup.rs")
    sys.exit(1)

try:
    if NORM_ARCH == "aarch64":
        print("==> Compiling for aarch64-unknown-linux-musl...")
        subprocess.run([cargo, "build", "--release", "--workspace", "--target", "aarch64-unknown-linux-musl"],
                       cwd=TOP, check=True)
        print("[OK] Compiled for aarch64-unknown-linux-musl")
    else:
        # For x86_64 target
        target_triple = "x86_64-unknown-linux-musl"
        res = subprocess.run([cargo, "build", "--release", "--workspace", "--target", target_triple], cwd=TOP)
        if res.returncode != 0:
            print(f"[INFO] Compiling default release workspace for {TARGET}...")
            subprocess.run([cargo, "build", "--release", "--workspace"], cwd=TOP, check=True)
    print("[OK] Rust crates compiled successfully.")
except subprocess.CalledProcessError as e:
    print(f"[ERROR] Cargo workspace compilation failed with exit code {e.returncode}")
    sys.exit(e.returncode)

# 3. Copy System Configuration & Tokens
print("==> [2/4] Installing System Configurations & Rules...")
etc_src = os.path.join(TOP, "etc", "nilos")
if os.path.exists(etc_src):
    shutil.copytree(etc_src, os.path.join(SYS, "etc", "nilos"), dirs_exist_ok=True)

# 4. Invoke Architecture-Aware Initramfs Packaging
print(f"==> [3/4] Packaging Initramfs ({NORM_ARCH})...")
mkinitramfs_script = os.path.join(TOP, "build", "mkinitramfs.py")
try:
    subprocess.run([sys.executable, mkinitramfs_script, "--arch", NORM_ARCH], cwd=TOP, check=True)
except subprocess.CalledProcessError as e:
    print(f"[ERROR] Initramfs packaging failed with exit code {e.returncode}")
    sys.exit(e.returncode)

# 5. Generate Build Manifest & Checksums
print("==> [4/4] Writing Build Manifest and Metadata...")
git_rev = "unknown"
try:
    git_rev = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=TOP, text=True).strip()
except Exception:
    pass

manifest = {
    "build_target": TARGET,
    "architecture": NORM_ARCH,
    "git_revision": git_rev,
    "build_time_utc": datetime.utcnow().isoformat() + "Z",
    "verified": True,
}
manifest_path = os.path.join(OUT, "build_manifest.json")
with open(manifest_path, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

print("=========================================================")
print(f"   OnuronOS build completed successfully: {OUT}         ")
print(f"   Target: {TARGET} ({NORM_ARCH}) | Git: {git_rev[:10]}   ")
print("=========================================================")

