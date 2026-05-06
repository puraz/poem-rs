#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required" >&2
  exit 1
fi

if ! cargo bundle --version >/dev/null 2>&1; then
  echo "Installing cargo-bundle..."
  cargo install cargo-bundle
fi

if ! command -v create-dmg >/dev/null 2>&1; then
  echo "create-dmg is required (brew install create-dmg)" >&2
  exit 1
fi

cargo build --release
cargo bundle --release --format osx

APP_PATH="target/release/bundle/osx/poem-rs.app"
if [[ ! -d "$APP_PATH" ]]; then
  echo "expected app bundle not found at $APP_PATH" >&2
  exit 1
fi

STAGE_DIR="target/release/bundle/dmg-stage"
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp -R "$APP_PATH" "$STAGE_DIR/"

mkdir -p target/release/bundle/dmg
DMG_NAME="poem-rs-$(grep '^version = ' Cargo.toml | head -n1 | cut -d '"' -f2)-macos.dmg"
DMG_PATH="target/release/bundle/dmg/${DMG_NAME}"

DMG_LABEL="poem-rs"

if create-dmg --help 2>&1 | grep -q -- "--dmg-title"; then
  DMG_TITLE_FLAG=(--dmg-title "$DMG_LABEL")
else
  DMG_TITLE_FLAG=(--volname "$DMG_LABEL")
fi

if create-dmg --help 2>&1 | grep -q -- "--skip-jenkins"; then
  DMG_SAFE_FLAG=(--skip-jenkins)
else
  DMG_SAFE_FLAG=()
fi

if create-dmg --help 2>&1 | grep -q -- "--app-drop-link"; then
  DMG_LAYOUT_FLAG=(
    --window-size 640 360
    --icon-size 128
    --icon "poem-rs.app" 180 170
    --app-drop-link 460 170
  )
else
  ln -s /Applications "$STAGE_DIR/Applications"
  DMG_LAYOUT_FLAG=()
fi

if create-dmg --help 2>&1 | grep -q -- "--overwrite"; then
  create-dmg \
    --overwrite \
    "${DMG_SAFE_FLAG[@]}" \
    "${DMG_TITLE_FLAG[@]}" \
    "${DMG_LAYOUT_FLAG[@]}" \
    "$DMG_PATH" \
    "$STAGE_DIR"
else
  rm -f "$DMG_PATH"
  create-dmg \
    "${DMG_SAFE_FLAG[@]}" \
    "${DMG_TITLE_FLAG[@]}" \
    "${DMG_LAYOUT_FLAG[@]}" \
    "$DMG_PATH" \
    "$STAGE_DIR"
fi

echo "DMG artifact: ${DMG_PATH}"
