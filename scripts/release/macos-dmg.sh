#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
cd "$REPO_ROOT"

APP_NAME="poem-rs"
APP_PATH="target/release/bundle/osx/${APP_NAME}.app"
STAGE_DIR="target/release/bundle/dmg-stage"
STAGE_APP_PATH="${STAGE_DIR}/${APP_NAME}.app"

warn() {
  echo "warning: $*" >&2
}

cleanup_signing_artifacts() {
  if [[ -n "${MACOS_KEYCHAIN_PATH:-}" && -f "${MACOS_KEYCHAIN_PATH}" ]]; then
    security delete-keychain "${MACOS_KEYCHAIN_PATH}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${MACOS_CERT_PATH:-}" && -f "${MACOS_CERT_PATH}" ]]; then
    rm -f "${MACOS_CERT_PATH}"
  fi
}

is_set() {
  local name="$1"
  [[ -n "${!name:-}" ]]
}

all_set() {
  local name
  for name in "$@"; do
    if ! is_set "$name"; then
      return 1
    fi
  done

  return 0
}

any_set() {
  local name
  for name in "$@"; do
    if is_set "$name"; then
      return 0
    fi
  done

  return 1
}

setup_codesign() {
  local keychain_password
  keychain_password="$(uuidgen)"

  MACOS_CERT_PATH="${RUNNER_TEMP:-/tmp}/poem-rs-signing-cert.p12"
  MACOS_KEYCHAIN_PATH="${RUNNER_TEMP:-/tmp}/poem-rs-signing.keychain-db"
  trap cleanup_signing_artifacts EXIT

  echo "$MACOS_CERT_P12_BASE64" | base64 --decode >"$MACOS_CERT_PATH"

  security create-keychain -p "$keychain_password" "$MACOS_KEYCHAIN_PATH"
  security set-keychain-settings -lut 21600 "$MACOS_KEYCHAIN_PATH"
  security unlock-keychain -p "$keychain_password" "$MACOS_KEYCHAIN_PATH"
  security import "$MACOS_CERT_PATH" -k "$MACOS_KEYCHAIN_PATH" -P "$MACOS_CERT_PASSWORD" -T /usr/bin/codesign -T /usr/bin/security
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$keychain_password" "$MACOS_KEYCHAIN_PATH"
  security list-keychains -d user -s "$MACOS_KEYCHAIN_PATH" /Library/Keychains/System.keychain
}

sign_app() {
  echo "Signing app bundle with Developer ID identity..."
  codesign \
    --force \
    --deep \
    --options runtime \
    --timestamp \
    --keychain "$MACOS_KEYCHAIN_PATH" \
    --sign "$MACOS_SIGNING_IDENTITY" \
    "$STAGE_APP_PATH"

  codesign --verify --deep --strict --verbose=2 "$STAGE_APP_PATH"
}

sign_dmg() {
  local dmg_path="$1"

  echo "Signing DMG container..."
  codesign \
    --force \
    --timestamp \
    --keychain "$MACOS_KEYCHAIN_PATH" \
    --sign "$MACOS_SIGNING_IDENTITY" \
    "$dmg_path"

  codesign --verify --verbose=2 "$dmg_path"
}

notarize_dmg() {
  local dmg_path="$1"

  echo "Submitting DMG for notarization..."
  xcrun notarytool submit \
    "$dmg_path" \
    --apple-id "$MACOS_NOTARY_APPLE_ID" \
    --password "$MACOS_NOTARY_APP_PASSWORD" \
    --team-id "$MACOS_NOTARY_TEAM_ID" \
    --wait

  echo "Stapling notarization ticket to DMG..."
  xcrun stapler staple "$dmg_path"
  xcrun stapler validate "$dmg_path"
  spctl -a -t open -vv "$dmg_path"
}

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

if [[ ! -d "$APP_PATH" ]]; then
  echo "expected app bundle not found at $APP_PATH" >&2
  exit 1
fi

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp -R "$APP_PATH" "$STAGE_DIR/"

SIGNING_VARS=(
  MACOS_CERT_P12_BASE64
  MACOS_CERT_PASSWORD
  MACOS_SIGNING_IDENTITY
)
NOTARY_VARS=(
  MACOS_NOTARY_APPLE_ID
  MACOS_NOTARY_APP_PASSWORD
  MACOS_NOTARY_TEAM_ID
)

if any_set "${SIGNING_VARS[@]}" && ! all_set "${SIGNING_VARS[@]}"; then
  echo "macOS signing was partially configured. Set MACOS_CERT_P12_BASE64, MACOS_CERT_PASSWORD, and MACOS_SIGNING_IDENTITY together." >&2
  exit 1
fi

if any_set "${NOTARY_VARS[@]}" && ! all_set "${NOTARY_VARS[@]}"; then
  echo "macOS notarization was partially configured. Set MACOS_NOTARY_APPLE_ID, MACOS_NOTARY_APP_PASSWORD, and MACOS_NOTARY_TEAM_ID together." >&2
  exit 1
fi

if all_set "${SIGNING_VARS[@]}"; then
  setup_codesign
  sign_app
else
  warn "building unsigned macOS app bundle; downloaded builds may be blocked by Gatekeeper"
fi

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

if all_set "${SIGNING_VARS[@]}"; then
  sign_dmg "$DMG_PATH"
fi

if all_set "${SIGNING_VARS[@]}" && all_set "${NOTARY_VARS[@]}"; then
  notarize_dmg "$DMG_PATH"
elif all_set "${SIGNING_VARS[@]}"; then
  warn "DMG was signed but not notarized; browsers and Gatekeeper may still reject it on other Macs"
fi

echo "DMG artifact: ${DMG_PATH}"
