# Inspiration · 灵感捕手

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Inspiration" width="128" height="128">
</p>

<p align="center">
  <strong>极致轻量的灵感记录工具。快捷键呼出，以思维的速度捕捉想法。</strong>
</p>

<p align="center">
  <a href="README.en.md">🇺🇸 English</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/平台-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/badge/许可-AGPL--3.0-green" alt="License">
  <img src="https://img.shields.io/badge/构建-Tauri%202-4f6ef7" alt="Tauri">
  <img src="https://img.shields.io/badge/前端-原生%20JS-ff69b4" alt="Vanilla JS">
  <img src="https://img.shields.io/badge/体积-<5MB-orange" alt="Size">
</p>

---

## 这是什么？

**Inspiration（灵感捕手）** 是一个极致轻量的桌面工具，用于在灵感闪现时瞬间捕捉它。

按下 `Ctrl+Shift+I`，一张小卡片出现在鼠标旁边——输入你的想法，回车，带时间戳保存。无需切换窗口、无需等待加载。

> 用极简主义构建：零 npm 依赖、单文件前端、约 3MB 二进制。

---

## 功能特性

- **闪现捕捉** — `Ctrl+Shift+I` 在鼠标旁呼出卡片。输入，回车，完成。
- **对话式交互** — 每条想法像聊天消息一样，带时间戳，像与自己的对话。
- **Markdown 原生支持** — 标题、代码块、链接、图片，自然书写。
- **时间流视图** — 所有想法按时间排列，翻阅你的思维历史。
- **标签与筛选** — 用标签组织。可按一个或多个标签筛选时间流。
- **全文搜索** — 即时搜索所有内容。
- **AI 润色** — 用 AI 打磨你的原始想法，保留原意和语气，去掉 AI 味。智能标签建议基于你已有的标签。
- **转为待办** — 一键将任何想法转为待办。顶部独立待办区追踪进度。
- **WebDAV 同步** — 通过任意 WebDAV 服务器（Nextcloud、ownCloud 等）跨设备同步数据。
- **本地优先** — 所有数据存储在 SQLite 中。无需云服务，完全离线可用。
- **极致轻量** — 约 3MB 下载，约 50MB 内存占用，零 npm 依赖，无构建步骤。

---

## 安装

### 下载预编译包

从 [GitHub Releases](https://github.com/yzr278892/Inspiration/releases) 下载最新版本。

| 平台 | 安装包 |
|------|--------|
| **Windows** | `.msi` 安装程序 |
| **macOS** | `.dmg` 磁盘映像 |
| **Linux** | `.deb` 包 或 `.AppImage` |

### 或从源码构建

```bash
# 系统依赖
# Linux: sudo apt install libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
# macOS: Xcode Command Line Tools
# Windows: WebView2（Windows 10+ 已预装）

git clone https://github.com/yzr278892/Inspiration.git
cd Inspiration
cargo tauri build
```

---

## 使用指南

### 快速捕捉

1. 在桌面任意位置按 **`Ctrl+Shift+I`**
2. 鼠标旁出现一张卡片
3. 输入你的想法（支持 Markdown）
4. 按 **`Enter`** 保存，**`Shift+Enter`** 换行
5. 按 **`Escape`** 关闭

### 浏览与整理

- 点击右上角 **☰** 切换完整视图
- **搜索框** 实时筛选内容
- 点击 **标签胶囊** 按标签筛选
- 每条想法有四个操作按钮：
  - **+Tag** — 添加或创建标签
  - **AI** — AI 润色与标签建议
  - **☐** — 转为待办
  - **×** — 删除

### AI 润色

1. 点击任意想法卡片上的 **AI** 按钮
2. AI 重写你的想法——保留原意，去掉 AI 味
3. 下方显示建议标签（点击选中/取消）
4. 可直接编辑润色后的文本
5. 点击 **Apply** 保存

需要配置 OpenAI 兼容 API 密钥。在设置（**⚙**）中配置。

### 同步

1. 点击 **⚙** 设置 → 配置 WebDAV（地址、用户名、密码）
2. 随时点击 **↻** 同步按钮
3. 同步按最后更新时间合并——你的最新修改优先

---

## 架构

```
Inspiration/
├── src/
│   └── index.html          # 单文件前端（HTML+CSS+JS，约 550 行）
├── src-tauri/
│   ├── Cargo.toml          # Rust 依赖（5 个 crate）
│   ├── tauri.conf.json     # 窗口配置、快捷键、打包设置
│   ├── capabilities/       # Tauri v2 权限配置
│   └── src/
│       ├── main.rs         # 入口
│       ├── lib.rs          # App 构建、全局快捷键、窗口生命周期
│       ├── db.rs           # SQLite 建表 + 全部 CRUD（约 440 行）
│       ├── commands.rs     # 14 个 Tauri IPC 处理器 + AI API（约 270 行）
│       └── sync.rs         # WebDAV 同步引擎（约 130 行）
```

**技术栈：**

| 层 | 技术 | 用途 |
|----|------|------|
| 桌面壳 | [Tauri v2](https://v2.tauri.app/) | 原生窗口、全局快捷键、跨平台 |
| 前端 | 原生 HTML/CSS/JS | 零 npm、零构建、即时加载 |
| 本地存储 | SQLite（[rusqlite](https://github.com/rusqlite/rusqlite)） | 单文件数据库、零配置 |
| 同步 | [reqwest](https://github.com/seanmonstar/reqwest) | WebDAV HTTP 客户端 |
| AI | OpenAI 兼容 API | 任意模型（推荐 GPT-4o-mini） |

**设计理念：**
- **零 npm** — 无 `package.json`、无 `node_modules`、无打包器
- **单文件前端** — HTML+CSS+JS 合并在一个文件，作为 Tauri 静态资源嵌入
- **极简 Rust** — Tauri 之外仅 5 个依赖 crate
- **瞬时冷启动** — 从按下快捷键到聚焦输入框，不到 500 毫秒

---

## Android 与移动端

Android 端正在规划中。移动端体验将围绕触屏交互设计：

- **快捷设置磁贴** — 从通知栏一键点开捕捉界面（Android 版「全局快捷键」）
- **分享意图** — 从任意 App 分享文字到 Inspiration
- **通知栏捕捉** — 常驻通知，一键快速记录
- 卡片**居中全屏**打开（移动端无鼠标光标）
- 使用 Tauri v2 的移动端后端（开发中）

> 移动端构建将在 Tauri v2 移动支持稳定后提供。

---

## 开发

```bash
# 安装 Tauri CLI
cargo install tauri-cli --version "^2"

# 开发模式运行
cargo tauri dev

# 生产构建
cargo tauri build
```

### 项目目标

- **体积**：< 5 MB（压缩后）
- **内存**：< 80 MB 空闲
- **冷启动**：< 1 秒
- **代码量**：< 2,000 行总数

---

## 许可证

[GNU Affero General Public License v3.0](LICENSE)

Copyright (c) 2026 Inspiration Contributors

---

<p align="center">
  <sub>献给那些我们每天都在丢失的灵感。</sub>
</p>
