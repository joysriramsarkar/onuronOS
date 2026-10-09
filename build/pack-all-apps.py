#!/usr/bin/env python3
"""
build/pack-all-apps.py — Package all native OnuronOS apps into signed .nilax packages
Usage: python build/pack-all-apps.py [--release]
"""

import os
import sys
import subprocess
import shutil

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(TOP, "out", "packages")

APPS = [
    {
        "pkg": "hello",
        "app_id": "org.onuron.hello",
        "name": "Hello Onuron",
        "desc": "OnuronOS Native Hello World Application",
        "perms": "display,input",
    },
    {
        "pkg": "launcher",
        "app_id": "org.onuron.launcher",
        "name": "Launcher",
        "desc": "OnuronOS Native Application Launcher",
        "perms": "display,input,app_launch",
    },
    {
        "pkg": "settings",
        "app_id": "org.onuron.settings",
        "name": "Settings",
        "desc": "System Settings and Permissions",
        "perms": "display,input,system_config",
    },
    {
        "pkg": "lockscreen",
        "app_id": "org.onuron.lockscreen",
        "name": "Lockscreen",
        "desc": "Native Lockscreen and Keyguard",
        "perms": "display,input,auth",
    },
    {
        "pkg": "oobe",
        "app_id": "org.onuron.oobe",
        "name": "Out of Box Experience",
        "desc": "First boot wizard and device setup",
        "perms": "display,input,network",
    },
    {
        "pkg": "clockwidget",
        "app_id": "org.onuron.clockwidget",
        "name": "Clock Widget",
        "desc": "Desktop Clock Widget",
        "perms": "display",
    },
    {
        "pkg": "animdemo",
        "app_id": "org.onuron.animdemo",
        "name": "Animation Physics Demo",
        "desc": "120Hz Animation Physics Demo",
        "perms": "display,input",
    },
    {
        "pkg": "bridgedemo",
        "app_id": "org.onuron.bridgedemo",
        "name": "Android Intent Bridge Demo",
        "desc": "Android Intent Bridge Demo",
        "perms": "display,input,bridge",
    },
    {
        "pkg": "busdemo",
        "app_id": "org.onuron.busdemo",
        "name": "SoftBus Demo",
        "desc": "SoftBus Cross-Device Demo",
        "perms": "display,input,softbus",
    },
    {
        "pkg": "camdemo",
        "app_id": "org.onuron.camdemo",
        "name": "Camera Demo",
        "desc": "Camera Preview and Capture Demo",
        "perms": "display,input,camera",
    },
]

def main():
    is_release = "--release" in sys.argv
    build_mode = "release" if is_release else "debug"

    os.makedirs(OUT_DIR, exist_ok=True)

    cargo = shutil.which("cargo")
    if not cargo:
        print("[ERROR] 'cargo' compiler not found in PATH.")
        sys.exit(1)

    print(f"==> Packaging OnuronOS native apps ({build_mode})...")

    # Build nilpkg tool first
    nilpkg_args = [cargo, "build", "-p", "nilpkg"]
    if is_release:
        nilpkg_args.append("--release")
    subprocess.check_call(nilpkg_args, cwd=TOP)

    exe_suffix = ".exe" if sys.platform == "win32" else ""
    nilpkg_bin = os.path.join(TOP, "target", build_mode, f"nilpkg{exe_suffix}")

    success_count = 0
    for app in APPS:
        pkg = app["pkg"]
        app_id = app["app_id"]
        out_nilax = os.path.join(OUT_DIR, f"{pkg}.nilax")

        print(f"--> Building {pkg}...")
        build_cmd = [cargo, "build", "-p", pkg]
        if is_release:
            build_cmd.append("--release")
        subprocess.check_call(build_cmd, cwd=TOP)

        bin_path = os.path.join(TOP, "target", build_mode, f"{pkg}{exe_suffix}")
        if not os.path.exists(bin_path):
            print(f"[WARN] Binary not found at {bin_path}, skipping.")
            continue

        print(f"--> Packing {app_id} into {out_nilax}...")
        pack_cmd = [
            nilpkg_bin,
            "pack",
            bin_path,
            out_nilax,
            "--app-id", app_id,
            "--name", app["name"],
            "--version", "1.0.0",
            "--desc", app["desc"],
            "--perm", app["perms"],
        ]
        res = subprocess.run(pack_cmd, cwd=TOP)
        if res.returncode == 0:
            success_count += 1
        else:
            print(f"[ERROR] Failed to pack {pkg}")

    print(f"\n==> Done! Successfully packed {success_count}/{len(APPS)} applications to {OUT_DIR}.")

if __name__ == "__main__":
    main()
