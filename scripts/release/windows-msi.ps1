$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..")
Set-Location $repoRoot

# 1. 确保 Rust 和 cargo-wix 就绪
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo is required"
}

if (-not (cargo wix --version 2>$null)) {
  Write-Host "Installing cargo-wix..."
  cargo install cargo-wix --locked
}

# 2. 提取版本号
$version = cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="poem-rs") | .version'
Write-Host "CargoVersion = $version"

# 3. 【核心降维打击】设置为系统环境变量
# cargo-wix 底层调用的 candle.exe/light.exe 会自动读取这些变量
$env:CargoVersion = $version
$env:CargoTargetBinDir = Resolve-Path "target\release"

Write-Host "Setting env CargoVersion = $env:CargoVersion"
Write-Host "Setting env CargoTargetBinDir = $env:CargoTargetBinDir"

# 4. 确保 Release 二进制存在 (WiX 需要打包的文件必须真实存在)
Write-Host "Building release binary..."
cargo build --release

# 5. 纯净调用，不带任何可能引发解析错误的 -d 参数
Write-Host "Starting cargo wix build..."
cargo wix --nocapture

if ($LASTEXITCODE -ne 0) {
  throw "cargo wix failed with exit code $LASTEXITCODE"
}

# 6. 输出产物路径
$msiFiles = Get-ChildItem -Path "target\wix" -Filter *.msi -Recurse
if (-not $msiFiles) {
  throw "No MSI artifacts found!"
}

$msiFiles | ForEach-Object { $_.FullName }