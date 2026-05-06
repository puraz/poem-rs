$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

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

$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$package = $metadata.packages | Where-Object { $_.name -eq "poem-rs" } | Select-Object -First 1
if (-not $package) {
  throw "Could not resolve package metadata for poem-rs"
}
$version = $package.version
cargo wix `
  --nocapture `
  --target-bin-dir "target/release"

if ($LASTEXITCODE -ne 0) {
  throw "cargo wix failed with exit code $LASTEXITCODE"
}

Write-Host "CargoVersion = $version"
Write-Host "MSI artifacts:" 
$msiDir = Join-Path $repoRoot "target\\wix"
if (-not (Test-Path $msiDir)) {
  throw "MSI output directory not found: $msiDir"
}

$msiFiles = Get-ChildItem -Path $msiDir -Filter *.msi -Recurse
if (-not $msiFiles) {
  throw "No MSI artifacts found under $msiDir"
}

$msiFiles | ForEach-Object { $_.FullName }
