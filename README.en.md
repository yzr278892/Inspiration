# Inspiration

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Inspiration" width="128" height="128">
</p>

<p align="center">
  <strong>Ultra-lightweight inspiration capture tool. Summon with a shortcut, capture at the speed of thought.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green" alt="License">
  <img src="https://img.shields.io/badge/built%20with-Tauri%202-4f6ef7" alt="Tauri">
  <img src="https://img.shields.io/badge/frontend-Vanilla%20JS-ff69b4" alt="Vanilla JS">
</p>

---

## What is Inspiration?

**Inspiration** is an ultra-lightweight desktop tool for capturing fleeting thoughts before they evaporate. Press `Ctrl+Shift+I` anywhere, and a card appears right at your mouse cursor — type your idea, hit Enter, and it's saved with a timestamp. No friction, no context switching.

Think of it as a **chat-like scratchpad for your mind**: every thought becomes a card in a searchable timeline, enriched with Markdown, tags, AI polish, and WebDAV sync.

> Built with extreme minimalism in mind: zero npm dependencies, single-file frontend, ~3MB binary.

---

## Features

- **Flash Capture** — `Ctrl+Shift+I` summons a card at your cursor. Type, Enter, done.
- **Chat-like Flow** — Each idea is a card with a timestamp, like a conversation with yourself.
- **Markdown First** — Full Markdown support. Headers, code blocks, links, images — write naturally.
- **Timeline View** — All ideas arranged chronologically. Scroll through your thought history.
- **Tag & Filter** — Organize with tags. Filter the timeline by one or more tags.
- **Full-text Search** — Instantly find any idea across your entire history.
- **AI Rewrite** — Polish your raw thoughts with AI while preserving your voice and meaning. No AI flavor — just smoother writing. Smart tag suggestions based on your existing tags.
- **Todo Conversion** — Any idea can become a todo with one click. A dedicated Todo area at the top tracks what's pending.
- **WebDAV Sync** — Sync your data across devices via any WebDAV server (Nextcloud, ownCloud, etc.).
- **Local First** — All data stored in SQLite. No cloud dependency. Full offline support.
- **Extremely Lightweight** — ~3MB download, ~50MB RAM, zero npm dependencies, no build step.

---

## Installation

### Download Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/yzr278892/Inspiration/releases).

| Platform | Package |
|----------|---------|
| **Windows** | `.msi` installer |
| **macOS** | `.dmg` disk image |
| **Linux** | `.deb` package or `.AppImage` |

### Or Build from Source

```bash
# Prerequisites
# Linux: sudo apt install libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
# macOS: Xcode Command Line Tools
# Windows: WebView2 (pre-installed on Windows 10+)

git clone https://github.com/yzr278892/Inspiration.git
cd Inspiration
cargo tauri build
```

---

## Usage

### Quick Capture

1. Press **`Ctrl+Shift+I`** anywhere on your desktop
2. A card appears near your mouse cursor
3. Type your idea (Markdown supported)
4. Press **`Enter`** to save, **`Shift+Enter`** for newline
5. Press **`Escape`** to dismiss

### Browse & Organize

- Click **☰** (top-right) to toggle full view
- **Search** box filters by content in real time
- Click **tag pills** to filter by tags
- Each idea card has four action buttons:
  - **+Tag** — Add or create tags
  - **AI** — AI rewrite with tag suggestions
  - **☐** — Convert to todo
  - **×** — Delete

### AI Rewrite

1. Click **AI** on any idea card
2. The AI rewrites your thought — preserving meaning, removing AI flavor
3. Suggested tags appear below (click to select/deselect)
4. Edit the rewritten text if needed
5. Click **Apply** to save

Requires an OpenAI-compatible API key. Configure in Settings (**⚙**).

### Sync

1. Click **⚙** Settings → configure WebDAV (URL, username, password)
2. Click **↻** Sync button at any time
3. Sync merges data by last-updated time — your newest changes win

---

## Architecture

```
Inspiration/
├── src/
│   └── index.html          # Single-file frontend (HTML+CSS+JS, ~550 lines)
├── src-tauri/
│   ├── Cargo.toml          # Rust dependencies (5 crates)
│   ├── tauri.conf.json     # Window config, shortcut, packaging
│   ├── capabilities/       # Tauri v2 permission grants
│   └── src/
│       ├── main.rs         # Entry point
│       ├── lib.rs          # App builder, global shortcut, window lifecycle
│       ├── db.rs           # SQLite schema + all CRUD (~440 lines)
│       ├── commands.rs     # 14 Tauri IPC handlers + AI API (~270 lines)
│       └── sync.rs         # WebDAV sync engine (~130 lines)
```

**Tech Stack:**

| Layer | Technology | Purpose |
|-------|------------|---------|
| Desktop Shell | [Tauri v2](https://v2.tauri.app/) | Native window, global shortcut, cross-platform |
| Frontend | Vanilla HTML/CSS/JS | Zero npm, zero build, instant load |
| Local Storage | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite)) | Single-file DB, zero config |
| Sync | [reqwest](https://github.com/seanmonstar/reqwest) | WebDAV HTTP client |
| AI | OpenAI-compatible API | Any model (GPT-4o-mini recommended) |

**Design Philosophy:**
- **Zero npm** — No `package.json`, no `node_modules`, no bundler
- **Single-file frontend** — HTML+CSS+JS in one file, embedded as Tauri asset
- **Minimal Rust** — Only 5 dependency crates beyond Tauri itself
- **Instant cold start** — From shortcut press to focused input in under 500ms

---

## Android & Mobile

Android support is on the roadmap. The mobile experience is designed around touch interaction:

- **Quick Settings Tile** — Tap to capture from anywhere (Android's equivalent of a global shortcut)
- **Share Intent** — Share text from any app directly to Inspiration
- **Notification Capture** — Persistent notification for one-tap quick capture
- The card opens **centered on screen** (no cursor on mobile) with a full-size input area
- Uses Tauri v2's mobile backend for Android (currently in development)

> Mobile builds will be available once Tauri v2 mobile support stabilizes. Track progress in the [Roadmap](#) discussion.

---

## Development

```bash
# Install Tauri CLI
cargo install tauri-cli --version "^2"

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

### Project Goals

- **Binary size**: < 5 MB (compressed)
- **RAM usage**: < 80 MB idle
- **Cold start**: < 1 second
- **Code size**: < 2,000 lines total

---

## License

[GNU Affero General Public License v3.0](LICENSE)

Copyright (c) 2026 Inspiration Contributors

---

<p align="center">
  <sub>Inspired by the thoughts we lose every day.</sub>
</p>
