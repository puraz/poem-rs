# Release Installers

`poem-rs` uses platform-native installers in CI. The Windows path is intentionally opinionated because MSI quality depends more on install semantics than on simply producing an `.msi` file.

## macOS DMG strategy

The macOS installer is built as a `.app` bundle with `cargo-bundle`, then wrapped in a DMG with `create-dmg`.

Important distribution rule:

- A DMG downloaded from a browser is not a real end-user macOS installer unless the enclosed `.app` is signed with a `Developer ID Application` certificate and the final DMG is notarized by Apple.
- If CI builds the DMG without those credentials, Gatekeeper may show messages like "已损坏，无法打开" on another Mac even when the binary itself runs locally on the build machine.

### Optional macOS signing and notarization secrets

If these repository secrets exist, the workflow signs the `.app`, signs the DMG, submits it to Apple notarization, and staples the ticket:

- `MACOS_CERT_P12_BASE64`: base64-encoded Developer ID Application `.p12`
- `MACOS_CERT_PASSWORD`: password for the `.p12`
- `MACOS_SIGNING_IDENTITY`: full codesigning identity name, for example `Developer ID Application: Example, Inc. (TEAMID1234)`
- `MACOS_NOTARY_APPLE_ID`: Apple ID used for notarization
- `MACOS_NOTARY_APP_PASSWORD`: app-specific password for that Apple ID
- `MACOS_NOTARY_TEAM_ID`: Apple Developer Team ID

Behavior by configuration:

- No macOS secrets: CI still builds an unsigned DMG for internal/local testing, but it is not suitable for normal browser download distribution.
- Signing secrets only: CI signs the `.app` and DMG, but Gatekeeper may still reject downloads because notarization is missing.
- Signing + notarization secrets: CI produces the DMG shape expected for regular macOS distribution.

## Windows MSI strategy

The Windows installer is built with `cargo-wix` on top of WiX Toolset v3.

Why this path is kept:

- It fits a Rust desktop app with a single binary and minimal install-time logic.
- It keeps the installer source in-repo as WiX XML, so upgrade behavior is explicit and reviewable.
- It allows optional Authenticode signing in GitHub Actions without introducing another packaging stack.

Why the MSI is authored this way:

- `perUser` install by default.
  This app stores data under the user profile, so forcing `Program Files` + admin elevation creates friction without benefit.
- Install location is `LocalAppData\Programs\poem-rs`.
  This is the common non-admin desktop app pattern on modern Windows.
- Stable `UpgradeCode` + `MajorUpgrade`.
  New tagged releases replace older installs cleanly.
- Start Menu and Desktop shortcuts are created in the current user's profile.
- The installer embeds cabinets, so the MSI is self-contained.
- Optional signing is supported from CI. Unsigned MSIs still install, but signing is strongly recommended for real users because SmartScreen reputation matters.

## GitHub Actions flow

The workflow in `.github/workflows/release-installers.yml` does the following for Windows:

1. Installs Rust, WiX Toolset v3, and `cargo-wix`.
2. Builds the release binary with `cargo build --release --locked`.
3. Verifies the Git tag matches the Cargo package version.
4. Builds the MSI from `wix/main.wxs`.
5. Optionally signs the MSI if CI secrets are present.
6. Generates a `.sha256` checksum file.
7. Uploads both the MSI and checksum as workflow artifacts and release assets.

## Optional Windows signing secrets

If these repository secrets exist, the workflow signs the MSI:

- `WINDOWS_CERT_PFX_BASE64`: base64-encoded `.pfx` certificate
- `WINDOWS_CERT_PASSWORD`: password for the `.pfx`
- `WINDOWS_SIGNTOOL_TIMESTAMP_URL`: optional RFC3161 timestamp URL

If the cert secrets are absent, the workflow still builds the MSI and prints that signing was skipped.

## Maintenance rules

- Keep `UpgradeCode` stable across releases.
- Do not change install scope casually. Per-user and per-machine installs do not major-upgrade across contexts.
- Keep user data outside the install directory. Upgrades should only replace app binaries and shortcuts.
- If you add extra runtime files on Windows, author them in WiX explicitly instead of copying ad hoc in CI.
- If the Cargo package version changes, release tags must continue to use the same `v<version>` shape.
