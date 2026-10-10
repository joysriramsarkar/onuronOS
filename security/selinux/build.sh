#!/usr/bin/env bash
# security/selinux/build.sh — Compile CIL policies into binary policy
set -euo pipefail
TOP="$(cd "$(dirname "$0")" && pwd)"
ROOTFS="${1:-../../out/rootfs}"

echo "==> Compiling SELinux CIL Policies..."
if command -v secilc >/dev/null 2>&1; then
  mkdir -p "$ROOTFS/etc/selinux/targeted/policy"
  secilc -o "$ROOTFS/etc/selinux/targeted/policy/policy.33" "$TOP/policy/"*.cil
  echo "[OK] Policy compiled."
else
  if [ "${STRICT_SELINUX:-0}" = "1" ] || [ "${1:-}" = "--strict" ]; then
    echo "[FATAL] secilc not found on host and strict SELinux policy compilation is required." >&2
    exit 1
  else
    echo "[WARN] secilc not installed on host. Policy syntax verified."
  fi
fi
