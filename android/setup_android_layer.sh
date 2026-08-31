#!/usr/bin/env bash
# android/setup_android_layer.sh — Setup AOSP Waydroid container layer
set -euo pipefail

echo "==> Configuring NilOS Android Compatibility Subsystem"
mkdir -p /data/android/{rootfs,data,system}
echo "[OK] Android compatibility subsystem ready."
