<div align="center">

# 📱 PhoneMirror

**Mirror your Android phone screen on your desktop**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform: macOS · Windows · Linux](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-green.svg)]()
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://rustup.rs)
[![Version: 2.0.0](https://img.shields.io/badge/version-2.0.0-brightgreen.svg)](https://github.com/minrahim1999/phonemirror/releases/tag/v2.0.0)

Screen mirroring · Screenshots · Recording — one click away.

[Download](#download) · [Build](#build) · [Usage](#usage) · [Contributing](CONTRIBUTING.md)

</div>

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🖥️ **Screen Mirror** | One-click scrcpy mirroring with full phone control |
| 📸 **Screenshot** | Capture phone screen to desktop via ADB |
| 🎬 **Recording** | Record phone screen as MP4 |
| 🔌 **Auto-Detect** | Finds ADB and scrcpy paths automatically |
| 🌍 **Cross-Platform** | macOS, Windows, Linux |
| 🎨 **Native GUI** | Dark theme dashboard with real-time device status |
| ⚡ **PATH Fix** | Works from .app bundle after reboot (no terminal needed) |

## Screenshots

> *PhoneMirror dashboard showing connected device with mirror, screenshot, and recording controls.*

## Download

### macOS (Apple Silicon)
```bash
# Download from GitHub Releases
# https://github.com/minrahim1999/phonemirror/releases/tag/v2.0.0
```

Or install via Homebrew (coming soon):
```bash
brew install --cask phonemirror
```

### Other Platforms
Build from source (see below) — Windows and Linux binaries coming soon.

## Prerequisites

| Tool | macOS | Linux | Windows |
|------|-------|-------|---------|
| **ADB** | `brew install android-platform-tools` | `sudo apt install adb` | [Download](https://developer.android.com/tools/releases/platform-tools) |
| **scrcpy** | `brew install scrcpy` | `sudo apt install scrcpy` | [Download](https://github.com/Genymobile/scrcpy/releases) |
| **USB Debugging** | Enable on your phone (Settings → Developer Options) | Same | Same |

## Build

```bash
# Clone
git clone https://github.com/minrahim1999/phonemirror.git
cd phonemirror

# Build (release mode)
cargo build --release

# Binary location
./target/release/phonemirror
```

### macOS .app Bundle
```bash
cargo build --release
./scripts/bundle-macos.sh
# Creates /Applications/PhoneMirror.app
```

## Usage

1. **Connect** your phone via USB (with USB debugging enabled)
2. **Open** PhoneMirror
3. **Click** "Start Mirror" to begin screen mirroring
4. Use **Screenshot** or **Start Recording** as needed

### Keyboard Shortcuts (in scrcpy window)
- `Ctrl+O` — Turn screen off
- `Ctrl+S` — Screenshot
- `Ctrl+R` — Record

## Architecture

```
┌─────────────────────────────────┐
│         PhoneMirror GUI         │
│         (egui + Rust)           │
├────────────────┬────────────────┤
│      ADB       │     scrcpy     │
│  (screenshot,  │  (mirror,      │
│   device list) │   recording)   │
└────────────────┴────────────────┘
```

PhoneMirror is a **native desktop GUI** built with [egui](https://github.com/emilk/egui) — no WebView, no Electron, no browser dependency. It wraps ADB and scrcpy commands with a clean interface and handles the `.app` bundle PATH issue on macOS.

## Why egui instead of Tauri/Electron?

Tauri (v2.11) uses WKWebView which [crashes on macOS 26 (Tahoe)](https://github.com/tauri-apps/wry/issues/1576). egui renders natively via OpenGL/Metal — no browser needed, so it works on all macOS versions including Tahoe.

## Project Structure

```
phonemirror-app/
├── src/
│   └── main.rs                # App logic (egui GUI + ADB/scrcpy)
├── scripts/
│   └── bundle-macos.sh        # macOS .app bundle creator
├── phonemirror.entitlements   # macOS code signing entitlements
├── Cargo.toml                 # Rust dependencies
├── LICENSE                    # MIT
├── CHANGELOG.md               # Version history
├── CONTRIBUTING.md            # How to contribute
└── README.md                  # This file
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. PRs welcome!

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

## License

[MIT](LICENSE) — free for personal and commercial use.

---

<div align="center">

Built with 🦀 Rust · [egui](https://github.com/emilk/egui) · [scrcpy](https://github.com/Genymobile/scrcpy) · [ADB](https://developer.android.com/tools/adb)

</div>