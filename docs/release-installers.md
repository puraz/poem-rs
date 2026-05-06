# Cross-Platform Installer Release Guide

This document defines how to build and publish end-user installers for `poem-rs`.

## Target Outputs

- Windows: MSI
- macOS: DMG
- Linux: AppImage + DEB

## Prerequisites

- Rust stable toolchain
- Platform-specific tools:
  - Windows: `cargo-wix`
  - macOS: `cargo-bundle`, `create-dmg`
  - Linux: `cargo-appimage`, `cargo-deb`

## One-Command Platform Builds

All scripts auto-locate the repository root and can be run from any working directory.

- Windows (PowerShell):

```powershell
./scripts/release/windows-msi.ps1
```

- macOS:

```bash
./scripts/release/macos-dmg.sh
```

- Linux:

```bash
./scripts/release/linux-packages.sh
```

## CI Release Workflow

Workflow file: `.github/workflows/release-installers.yml`

Triggers:
- Manual: `workflow_dispatch`
- Tag push: `v*` (for example `v0.1.0`)

Artifact jobs are independent, so one platform failure does not prevent other artifacts from being uploaded.

## Signing and Notarization

- Windows signing can be added after MSI build using `signtool`.
- macOS signing/notarization can be inserted between `.app` creation and `.dmg` packaging.

Recommended environment variables (when enabled):
- `APPLE_CERT_BASE64`, `APPLE_CERT_PASSWORD`, `APPLE_TEAM_ID`, `APPLE_ID`, `APPLE_APP_PASSWORD`
- `WIN_SIGN_CERT_BASE64`, `WIN_SIGN_CERT_PASSWORD`

## Phased Release Policy (7-day backfill)

Default is same-version release on all three platforms.
If one platform is blocked (commonly macOS signing/notarization), release the other two and mark the missing one as "coming soon".
The missing platform must be delivered within 7 days.

Suggested checklist fields in release notes:
- `release_version`
- `released_platforms`
- `blocked_platform`
- `blocker_reason`
- `eta_days` (must be `<= 7`)

## Troubleshooting

- Missing icon formats (`.ico`/`.icns`) can break GUI packaging metadata on some toolchains.
- AppImage build may fail without `libfuse2`, `patchelf`, or desktop integration metadata.
- MSI upgrade issues typically come from changing WiX `UpgradeCode`; keep it stable across versions.
