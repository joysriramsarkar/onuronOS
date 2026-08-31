#!/usr/bin/env bash
# tools/mkdocs.sh — Build NilOS Developer Documentation
set -euo pipefail
echo "==> Building mdBook & Rust API Documentation"
if command -v mdbook >/dev/null 2>&1; then
  mdbook build docs/
fi
cargo doc --workspace --no-deps
echo "[OK] Documentation generated in target/doc and docs/book"
