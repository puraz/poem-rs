$ErrorActionPreference = "Stop"

# 1. 切换到脚本所在的上级的上级目录（即仓库根目录）
$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent | Split-Path -Parent
Set-Location $repoRoot

Write-Host "Current working directory: $repoRoot"

# 2. 检查基础环境
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo is required but not found."
}

# 3. 安装 cargo-wix (如果不存在)
if (-not (cargo wix --version 2>$null)) {
  Write-Host "Installing cargo-wix..."
  cargo install cargo-wix --locked
}

# 4. 获取版本号并设置环境变量
$metadata = cargo metadata --format-version 1 | ConvertFrom-Json
$version = ($metadata.packages | Where-Object { $_.name -eq "poem-rs" } | Select-Object -First 1).version
if (-not $version) {
  throw "Failed to resolve poem-rs package version from cargo metadata."
}
Write-Host "CargoVersion = $version"
$env:CargoVersion = $version

# 5. 设置目标文件夹环境变量 (使用相对路径，避免 PowerShell Resolve-Path 的强校验崩溃)
$env:CargoTargetBinDir = "target\release"

# 6. 编译 Rust 项目 (确保 .exe 文件被生成)
Write-Host "Building release binary..."
cargo build --release

# 7. 确认 exe 是否存在，提前报错以便调试
if (-not (Test-Path "$env:CargoTargetBinDir\poem-rs.exe")) {
    throw "Build succeeded but $env:CargoTargetBinDir\poem-rs.exe was not found!"
}

# 8. 纯净调用 cargo wix
Write-Host "Starting cargo wix build..."
cargo wix --nocapture

if ($LASTEXITCODE -ne 0) {
  throw "cargo wix failed with exit code $LASTEXITCODE"
}

# 9. 输出产物
$msiFiles = Get-ChildItem -Path "target\wix" -Filter *.msi -Recurse
if (-not $msiFiles) { throw "MSI artifacts not found!" }
$msiFiles | ForEach-Object { Write-Host "Built artifact: $($_.FullName)" }
