$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..")
Set-Location $repoRoot

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo is required"
}

if (-not (cargo wix --version 2>$null)) {
  Write-Host "Installing cargo-wix..."
  cargo install cargo-wix --locked
}

$version = cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="poem-rs") | .version'
Write-Host "CargoVersion = $version"

cargo wix `
  --nocapture `
  -dCargoVersion="$version" `
  -dCargoTargetBinDir="target/release"

if ($LASTEXITCODE -ne 0) {
  throw "cargo wix failed with exit code $LASTEXITCODE"
}

$msiDir = Join-Path $repoRoot "target\wix"
if (-not (Test-Path $msiDir)) {
  throw "MSI output directory not found: $msiDir"
}

$msiFiles = Get-ChildItem -Path $msiDir -Filter *.msi -Recurse
if (-not $msiFiles) {
  throw "No MSI artifacts found under $msiDir"
}

$msiFiles | ForEach-Object { $_.FullName }