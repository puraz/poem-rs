# poem-rs

`poem-rs` 是一个基于 Rust 与 Iced 构建的桌面古诗词应用，聚焦本地阅读、收藏、编辑，以及基于 OpenAI-compatible API 的诗词发现与赏析体验。

## 预览

![预览 1](https://pub-fc8c6b11722d40338361e3020c2e3b7b.r2.dev/1777261797640_ScreenShot_2026-04-27_112912_141.webp)
![预览 2](https://pub-fc8c6b11722d40338361e3020c2e3b7b.r2.dev/1777261800484_ScreenShot_2026-04-27_112934_153.webp)
![预览 3](https://pub-fc8c6b11722d40338361e3020c2e3b7b.r2.dev/1777261797851_ScreenShot_2026-04-27_112303_090.webp)
![预览 4](https://pub-fc8c6b11722d40338361e3020c2e3b7b.r2.dev/1777261797447_ScreenShot_2026-04-27_112827_572.webp)

## 功能特性

- 关键词搜索：支持按标题、作者、正文内容进行本地检索。
- 收藏管理：一键收藏、取消收藏，并在收藏夹中单独查看。
- 诗词编辑：支持修改标题、作者、朝代和正文。
- AI 发现：输入诗句片段、主题、意象或模糊记忆，调用 OpenAI-compatible 接口生成候选诗词并导入本地。
- AI 赏析：为当前诗词生成简洁中文赏析，并缓存到本地数据库。
- 主题切换：内置“寒江雪”“松烟笺”和“跟随系统”三种主题模式。
- 安全存储：API Key 优先保存在系统 keyring，不可用时可显式开启文件回退。

## 技术栈

- Rust 2024
- Iced `0.14`
- rusqlite（bundled SQLite）
- reqwest
- serde / serde_json
- keyring
- tracing

## 快速开始

### 环境要求

- Rust stable（支持 Edition 2024）
- 可用的 OpenAI-compatible `/chat/completions` 接口（仅 AI 功能需要）

### 运行

```bash
cargo run
```

首次启动时，应用会自动：

- 创建系统标准配置目录和数据目录
- 初始化本地 SQLite 数据库
- 从 `assets/poetry/corpus.json` 导入内置诗词种子数据

## AI 配置

在应用内打开“设置”后，可以配置以下字段：

- `Base URL`，默认值为 `https://api.openai.com/v1`
- `Model`，默认值为 `gpt-4.1-mini`
- `API Key`
- `允许文件回退存储`

说明：

- API Key 默认优先写入系统 keyring。
- 如果 keyring 不可用，并且你显式勾选“允许文件回退存储”，则会写入本地 `ai-secret.toml`。
- AI 发现和 AI 赏析都基于 OpenAI-compatible Chat Completions 接口。

## 数据与存储

- 本地数据库文件名为 `poems.sqlite3`
- 诗词、收藏、主题偏好、AI 配置元数据、赏析缓存都保存在本地数据库中
- API Key 单独存储在 keyring 或 `ai-secret.toml`
- 应用目录通过 `directories::ProjectDirs` 按系统标准路径解析

## 内置内容

- 当前内置种子诗词共 `8` 首
- 数据文件位于 `assets/poetry/corpus.json`
- 校验清单位于 `assets/poetry/manifest.json`

## 项目结构

```text
assets/
  icons/                  # SVG 图标资源
  poetry/                 # 内置诗词语料与清单
src/
  config/                 # 应用路径与 AI 配置
  domain/                 # 领域模型
  services/               # AI 请求、本地匹配、结果规范化
  storage/                # SQLite 存储与种子导入
  ui/                     # Iced 界面、状态、任务与弹窗
  lib.rs
  main.rs
Cargo.toml
```

## 开发命令

```bash
cargo fmt
cargo test
cargo run
```

## 下载与安装

安装包通过 [GitHub Releases](https://github.com/puraz/poem-rs-xus/releases) 发布，从最新 Release 的 **Assets** 区域下载对应平台的安装文件。

### macOS

**安装包：** `poem-rs-{{ 版本号 }}-macos.dmg`

**安装步骤：**

1. 下载 `.dmg` 文件并双击挂载
2. 将 `poem-rs.app` 拖入 `应用程序` 文件夹
3. 首次打开时，macOS Gatekeeper 可能会提示**“已损坏，无法打开”**或**“无法验证开发者”**，这是因为安装包尚未进行 Apple 签名与公证。请按以下任一方式绕过：

   **方式一：终端命令（推荐）**

   ```bash
   sudo xattr -rd com.apple.quarantine /Applications/poem-rs.app
   ```

   **方式二：系统设置**

   - 打开 **系统设置 → 隐私与安全性**
   - 滚动到下方 **安全性** 区域，点击 **“仍要打开”**
   - 在弹出的确认对话框中点击 **“打开”**

> 如果上述选项未出现，可先尝试方式一。未来配置 Apple 开发者证书并启用公证后，将不再需要此步骤。

### Windows

**安装包：** `poem-rs-{{ 版本号 }}-x86_64.msi`

**安装步骤：**

1. 下载 `.msi` 文件并双击运行
2. 按安装向导完成安装
3. 安装完成后在开始菜单中找到 `poem-rs` 启动

### Linux

提供两种格式：

| 格式 | 文件 | 适用发行版 |
|------|------|-----------|
| **AppImage** | `poem-rs-{{ 版本号 }}-x86_64.AppImage` | 所有主流发行版（通用） |
| **DEB** | `poem-rs_{{ 版本号 }}-1_amd64.deb` | Debian / Ubuntu 及衍生版 |

**AppImage 安装：**

```bash
chmod +x poem-rs-*-x86_64.AppImage
./poem-rs-*-x86_64.AppImage
```

可将其移动到任意目录（如 `~/Applications`）或创建桌面快捷方式。

**DEB 安装：**

```bash
sudo dpkg -i poem-rs_*-1_amd64.deb
# 如遇依赖缺失：
sudo apt-get install -f
```

安装后可在应用菜单中启动 `poem-rs`。

### 从源码构建

如需自行编译，请参考上方 [快速开始](#快速开始) 章节。
