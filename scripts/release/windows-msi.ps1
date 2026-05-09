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

# 4. 设置目标文件夹参数，交给 cargo-wix 的内置变量使用
$cargoTargetBinDir = "target\release"

# 5. 编译 Rust 项目 (确保 .exe 文件被生成)
Write-Host "Building release binary..."
cargo build --release

# 6. 确认 exe 是否存在，提前报错以便调试
if (-not (Test-Path "$cargoTargetBinDir\poem-rs.exe")) {
    throw "Build succeeded but $cargoTargetBinDir\poem-rs.exe was not found!"
}

# 7. 调用 cargo-wix，复用已生成的 release 二进制，避免重复编译
Write-Host "Starting cargo wix build..."
cargo wix --nocapture --no-build --target-bin-dir "$cargoTargetBinDir"

if ($LASTEXITCODE -ne 0) {
  throw "cargo wix failed with exit code $LASTEXITCODE"
}

# 8. 输出产物
$msiFiles = Get-ChildItem -Path "target\wix" -Filter *.msi -Recurse
if (-not $msiFiles) { throw "MSI artifacts not found!" }
$msiFiles | ForEach-Object { Write-Host "Built artifact: $($_.FullName)" }
