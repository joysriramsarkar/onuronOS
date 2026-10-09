#!/usr/bin/env bash
# security/selinux/ci/audit.sh — Automated Policy & Budget CI Audit
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POLICY_DIR="$(cd "$SCRIPT_DIR/../policy" && pwd)"

echo "==> Auditing SELinux policy against Constitution in $POLICY_DIR..."

if [ ! -d "$POLICY_DIR" ]; then
  echo "[FAIL] Policy directory not found: $POLICY_DIR" >&2
  exit 1
fi

cil_files=("$POLICY_DIR"/*.cil)
if [ ${#cil_files[@]} -eq 0 ] || [ ! -f "${cil_files[0]}" ]; then
  echo "[FAIL] No .cil policy files found in $POLICY_DIR" >&2
  exit 1
fi

echo "[INFO] Found ${#cil_files[@]} policy files."

# 1. Check for neverallow violations in policy files
violations=0
neverallow_rules=()

while IFS= read -r line; do
  clean_line="$(echo "$line" | sed 's/;.*//' | tr -s '[:space:]' ' ' | sed 's/^ //;s/ $//')"
  if [[ "$clean_line" == "(neverallow "* ]]; then
    neverallow_rules+=("$clean_line")
  fi
done < <(grep -h "^\s*(neverallow" "$POLICY_DIR"/*.cil 2>/dev/null || true)

echo "[INFO] Extracted ${#neverallow_rules[@]} neverallow constraint(s)."

for na in "${neverallow_rules[@]}"; do
  src=$(echo "$na" | awk '{print $2}')
  tgt=$(echo "$na" | awk '{print $3}')
  cls=$(echo "$na" | awk '{print $4}' | tr -d '()')
  perm=$(echo "$na" | awk '{print $5}' | tr -d '()')
  
  if grep -hE "^\s*\(allow\s+$src\s+$tgt\s+\(\s*$cls\s+\([^\)]*$perm" "$POLICY_DIR"/*.cil >/dev/null 2>&1; then
    echo "[CRITICAL VIOLATION] Rule permits forbidden transition: $src -> $tgt ($cls ($perm))" >&2
    violations=$((violations + 1))
  fi
done

if [ "$violations" -gt 0 ]; then
  echo "[FAIL] $violations neverallow violation(s) detected!" >&2
  exit 1
fi

echo "[OK] 0 neverallow violations detected."

if command -v secilc >/dev/null 2>&1; then
  echo "==> Validating with secilc compiler..."
  tmp_out="$(mktemp -d)"
  secilc -o "$tmp_out/policy.bin" -f "$tmp_out/file_contexts" "$POLICY_DIR"/*.cil
  rm -rf "$tmp_out"
  echo "[OK] secilc compiler validation succeeded."
fi
