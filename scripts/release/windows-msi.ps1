$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\\..")
Set-Location $repoRoot

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo is required"
}

if (-not (cargo wix --version 2>$null)) {
  Write-Host "Installing cargo-wix..."
  cargo install cargo-wix
}

$version = cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="poem-rs") | .version'
cargo wix `
  --nocapture `
  -dCargoVersion="$version" `
  -dCargoTargetBinDir="target/release"

Write-Host "CargoVersion = $version"
Write-Host "MSI artifacts:" 
Get-ChildItem -Path target/wix -Filter *.msi -Recurse | ForEach-Object { $_.FullName }
