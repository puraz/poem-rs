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

cargo wix --nocapture

Write-Host "MSI artifacts:" 
Get-ChildItem -Path target/wix -Filter *.msi -Recurse | ForEach-Object { $_.FullName }
