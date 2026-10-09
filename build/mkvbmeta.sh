#!/usr/bin/env bash
# build/mkvbmeta.sh — System partition hash tree + Ed25519 signature
set -euo pipefail
TOP="$(cd "$(dirname "$0")/.." && pwd)"
SYS_IMG="${1:-$TOP/out/x86_64-generic/system_a.img}"
OUT_META="${2:-$TOP/out/x86_64-generic/vbmeta_a.img}"

echo "==> Generating dm-verity root hash and Ed25519 signature for $SYS_IMG"
python3 "$TOP/build/mkvbmeta.py" create --image "$SYS_IMG" -o "$OUT_META" --partition "system"
