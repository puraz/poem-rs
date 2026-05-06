#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required" >&2
  exit 1
fi

if ! cargo appimage --version >/dev/null 2>&1; then
  echo "Installing cargo-appimage..."
  cargo install cargo-appimage
fi

if ! cargo deb --version >/dev/null 2>&1; then
  echo "Installing cargo-deb..."
  cargo install cargo-deb
fi

cargo build --release
cargo appimage
cargo deb --no-build --output target/debian/

echo "AppImage artifacts:"
find target -type f -name "*.AppImage" -print

echo "DEB artifacts:"
find target/debian -type f -name "*.deb" -print
