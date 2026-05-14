$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-RepoRoot {
  return (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
}

function Get-PackageVersion {
  $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
  $rootManifestPath = (Resolve-Path "Cargo.toml").Path
  $rootPackage = $metadata.packages | Where-Object { $_.manifest_path -eq $rootManifestPath } | Select-Object -First 1
  if (-not $rootPackage) {
    throw "failed to resolve root package from cargo metadata"
  }

  return [string]$rootPackage.version
}

function Assert-ReleaseTagMatchesVersion([string]$packageVersion) {
  if (-not $env:GITHUB_REF) {
    return
  }

  if ($env:GITHUB_REF -notmatch '^refs/tags/v(.+)$') {
    return
  }

  $tagVersion = $Matches[1]
  if ($tagVersion -ne $packageVersion) {
    throw "release tag version '$tagVersion' does not match Cargo.toml version '$packageVersion'"
  }
}

function Assert-MsiCompatibleVersion([string]$packageVersion) {
  if ($packageVersion -notmatch '^\d+\.\d+\.\d+$') {
    throw "MSI ProductVersion must use exactly three numeric components. Current version: '$packageVersion'"
  }
}

function Find-SignTool {
  $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($cmd) {
    return $cmd.Source
  }

  $kitsRoot = "C:\Program Files (x86)\Windows Kits\10\bin"
  if (Test-Path $kitsRoot) {
    $candidate = Get-ChildItem -Path $kitsRoot -Recurse -Filter signtool.exe |
      Sort-Object FullName -Descending |
      Select-Object -First 1
    if ($candidate) {
      return $candidate.FullName
    }
  }

  throw "signtool.exe was not found. Install the Windows SDK or ensure signtool is on PATH."
}

function Sign-MsiIfConfigured([string]$msiPath) {
  $hasPfx = -not [string]::IsNullOrWhiteSpace($env:WINDOWS_CERT_PFX_BASE64)
  $hasPassword = -not [string]::IsNullOrWhiteSpace($env:WINDOWS_CERT_PASSWORD)

  if (-not $hasPfx -and -not $hasPassword) {
    Write-Host "Skipping MSI signing because no signing secrets were provided."
    return
  }

  if (-not $hasPfx -or -not $hasPassword) {
    throw "MSI signing was partially configured. Both WINDOWS_CERT_PFX_BASE64 and WINDOWS_CERT_PASSWORD are required."
  }

  $timestampUrl = if ([string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNTOOL_TIMESTAMP_URL)) {
    "http://timestamp.digicert.com"
  } else {
    $env:WINDOWS_SIGNTOOL_TIMESTAMP_URL
  }

  $signtool = Find-SignTool
  $certPath = Join-Path $env:RUNNER_TEMP "poem-rs-signing-cert.pfx"
  [System.IO.File]::WriteAllBytes($certPath, [Convert]::FromBase64String($env:WINDOWS_CERT_PFX_BASE64))

  try {
    Write-Host "Signing MSI with signtool..."
    & $signtool sign /fd SHA256 /f $certPath /p $env:WINDOWS_CERT_PASSWORD /tr $timestampUrl /td SHA256 $msiPath
    if ($LASTEXITCODE -ne 0) {
      throw "signtool sign failed with exit code $LASTEXITCODE"
    }

    & $signtool verify /pa $msiPath
    if ($LASTEXITCODE -ne 0) {
      throw "signtool verify failed with exit code $LASTEXITCODE"
    }
  } finally {
    if (Test-Path $certPath) {
      Remove-Item $certPath -Force
    }
  }
}

function Write-Checksum([string]$artifactPath) {
  $hash = Get-FileHash -Path $artifactPath -Algorithm SHA256
  $checksumPath = "$artifactPath.sha256"
  "$($hash.Hash.ToLowerInvariant())  $([System.IO.Path]::GetFileName($artifactPath))" | Set-Content -Path $checksumPath -NoNewline
  Write-Host "Checksum written to $checksumPath"
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

Write-Host "Current working directory: $repoRoot"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo is required but not found."
}

if (-not (cargo wix --version 2>$null)) {
  Write-Host "Installing cargo-wix..."
  cargo install cargo-wix --locked
}

$packageVersion = Get-PackageVersion
Assert-ReleaseTagMatchesVersion -packageVersion $packageVersion
Assert-MsiCompatibleVersion -packageVersion $packageVersion

$cargoTargetBinDir = "target\release"

Write-Host "Building release binary..."
cargo build --release --locked
if ($LASTEXITCODE -ne 0) {
  throw "cargo build failed with exit code $LASTEXITCODE"
}

$exePath = Join-Path $cargoTargetBinDir "poem-rs.exe"
if (-not (Test-Path $exePath)) {
  throw "Build succeeded but $exePath was not found."
}

Write-Host "Starting cargo wix build..."
cargo wix --nocapture --no-build --target-bin-dir "$cargoTargetBinDir"
if ($LASTEXITCODE -ne 0) {
  throw "cargo wix failed with exit code $LASTEXITCODE"
}

$msi = Get-ChildItem -Path "target\wix" -Filter *.msi -Recurse |
  Sort-Object LastWriteTimeUtc -Descending |
  Select-Object -First 1
if (-not $msi) {
  throw "MSI artifacts not found."
}

Sign-MsiIfConfigured -msiPath $msi.FullName
Write-Checksum -artifactPath $msi.FullName

Write-Host "Built artifact: $($msi.FullName)"
