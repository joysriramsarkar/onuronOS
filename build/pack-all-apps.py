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
        "perms": "display,input",
    },
    {
        "pkg": "settings",
        "app_id": "org.onuron.settings",
        "name": "Settings",
        "desc": "System Settings and Permissions",
        "perms": "display,input,bluetooth,wifi,cellular",
    },
    {
        "pkg": "lockscreen",
        "app_id": "org.onuron.lockscreen",
        "name": "Lockscreen",
        "desc": "Native Lockscreen and Keyguard",
        "perms": "display,input",
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
        "perms": "display,input",
    },
    {
        "pkg": "busdemo",
        "app_id": "org.onuron.busdemo",
        "name": "SoftBus Demo",
        "desc": "SoftBus Cross-Device Demo",
        "perms": "display,input,network",
    },
    {
        "pkg": "camdemo",
        "app_id": "org.onuron.camdemo",
        "name": "Camera Demo",
        "desc": "Camera Preview and Capture Demo",
        "perms": "display,input,camera",
    },
    {
        "pkg": "phone",
        "app_id": "org.onuron.phone",
        "name": "Phone",
        "desc": "Native Cellular & VoLTE Dialer",
        "perms": "display,input,phone,audio",
    },
    {
        "pkg": "messages",
        "app_id": "org.onuron.messages",
        "name": "Messages",
        "desc": "Encrypted SMS and Messaging",
        "perms": "display,input,sms,contacts",
    },
    {
        "pkg": "camera",
        "app_id": "org.onuron.camera",
        "name": "Camera",
        "desc": "Camera Viewfinder and Capture",
        "perms": "display,input,camera,storage.write",
    },
    {
        "pkg": "music",
        "app_id": "org.onuron.music",
        "name": "Music",
        "desc": "Native Audio Player",
        "perms": "display,input,audio,storage.read",
    },
    {
        "pkg": "files",
        "app_id": "org.onuron.files",
        "name": "Files",
        "desc": "OnuronOS File Manager",
        "perms": "display,input,storage.read,storage.write",
    },
    {
        "pkg": "notes",
        "app_id": "org.onuron.notes",
        "name": "Notes",
        "desc": "NilLang Persistent Notes",
        "perms": "display,input,storage.read,storage.write",
    },
    {
        "pkg": "calculator",
        "app_id": "org.onuron.calculator",
        "name": "Calculator",
        "desc": "Alap Calculator",
        "perms": "display,input",
    },
    {
        "pkg": "browser",
        "app_id": "org.onuron.browser",
        "name": "Browser",
        "desc": "NilZar WebKit Browser",
        "perms": "display,input,network,storage.read",
    },
    {
        "pkg": "wifi",
        "app_id": "org.onuron.wifi",
        "name": "Wi-Fi",
        "desc": "Wi-Fi Network Manager",
        "perms": "display,input,wifi,network",
    },
    {
        "pkg": "bluetooth",
        "app_id": "org.onuron.bluetooth",
        "name": "Bluetooth",
        "desc": "Bluetooth Device Manager",
        "perms": "display,input,bluetooth",
    },
    {
        "pkg": "sound",
        "app_id": "org.onuron.sound",
        "name": "Sound",
        "desc": "Sound & Audio Profile Manager",
        "perms": "display,input,audio",
    },
    {
        "pkg": "display",
        "app_id": "org.onuron.display",
        "name": "Display",
        "desc": "DRM/KMS Display Settings",
        "perms": "display,input",
    },
    {
        "pkg": "security",
        "app_id": "org.onuron.security",
        "name": "Security",
        "desc": "Security & Sandbox Architecture",
        "perms": "display,input",
    },
    {
        "pkg": "sysupdate",
        "app_id": "org.onuron.sysupdate",
        "name": "System Update",
        "desc": "A/B Dual-Slot Transactional Updater",
        "perms": "display,input,network,storage.read",
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
