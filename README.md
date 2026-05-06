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

## 安装包发布

跨平台安装包发布流程见：

- `docs/release-installers.md`
- `.github/workflows/release-installers.yml`
