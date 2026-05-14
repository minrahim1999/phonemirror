# 📱 PhoneMirror

A lightweight system tray app for Android phone mirroring, screenshots, and screen recording — powered by [scrcpy](https://github.com/Genymobile/scrcpy).

**Cross-platform:** macOS · Windows · Linux

![License](https://img.shields.io/badge/license-MIT-blue.svg)

## ✨ Features

- **📱 Mirror Screen** — Open scrcpy mirror window
- **📸 Screenshot** — Save phone screenshots automatically
- **🎬 Record Video** — Record phone screen to video file
- **📵 Close Mirror** — Close scrcpy window without quitting the app
- **🔄 Auto-detect** — Polls for phone connection every 5 seconds
- **🔔 Notifications** — Alerts when phone connects/disconnects
- **🪶 Lightweight** — ~5MB RAM on macOS, ~10MB on other platforms

## 📦 Prerequisites

### All Platforms
- **[ADB](https://developer.android.com/studio/releases/platform-tools)** (Android Debug Bridge)
- **[scrcpy](https://github.com/Genymobile/scrcpy)** — screen mirroring tool
- **USB debugging** enabled on your Android phone

### macOS
```bash
brew install scrcpy android-platform-tools
```

### Linux (Ubuntu/Debian)
```bash
sudo apt install scrcpy adb
```

### Windows
1. Download [scrcpy](https://github.com/Genymobile/scrcpy/releases) and extract
2. Download [ADB platform-tools](https://developer.android.com/studio/releases/platform-tools) and extract
3. Add both directories to your system PATH

## 🚀 Quick Start

### Option 1: Download Release (Recommended)

Go to [Releases](https://github.com/minrahim1999/phonemirror/releases) and download the binary for your platform.

### Option 2: Build from Source

**You need [Rust](https://rustup.rs/) installed.**

```bash
git clone https://github.com/minrahim1999/phonemirror.git
cd phonemirror
cargo build --release
```

The binary will be at `target/release/phonemirror` (or `phonemirror.exe` on Windows).

### macOS: SwiftUI Version (Recommended)

macOS users can also use the native SwiftUI menu bar app for a richer experience:

```bash
cd macos-swiftui
# Build the .app bundle
swiftc -parse-as-library -o PhoneMirror PhoneMirror.swift -framework Cocoa -framework SwiftUI
```

See [`macos-swiftui/`](macos-swiftui/) for details.

## 🖥️ Platform Support

| Platform | UI | Auto-start | Notifications | Status |
|---|---|---|---|---|
| **macOS** | SwiftUI MenuBarExtra ✅ | LaunchAgent | osascript | Full |
| **macOS** | Rust tray-item | launchd | osascript | Full |
| **Windows** | Rust tray-item | Startup shortcut | Console | Full |
| **Linux** | Rust tray-item | .desktop file | notify-send | Full |

## 📂 Project Structure

```
phonemirror/
├── src/main.rs              # Cross-platform Rust app (all platforms)
├── Cargo.toml               # Rust dependencies
├── macos-swiftui/           # Native macOS SwiftUI app
│   ├── PhoneMirror.swift    # Source code
│   ├── Info.plist           # App bundle config
│   └── com.muhaimin.phonemirror.plist  # LaunchAgent
├── scripts/                 # Standalone shell scripts
│   ├── phone-screenshot     # Quick screenshot
│   ├── phone-record         # Quick record
│   └── phone-mirror         # Quick mirror
├── README.md
└── LICENSE
```

## ⚙️ Configuration

The app auto-detects ADB and scrcpy paths. If they're not in standard locations, set these environment variables:

| Variable | Description | Default |
|---|---|---|
| `HOME` / `USERPROFILE` | User home directory | Auto-detected |

### Path Detection Order

**ADB:**
- macOS: `~/Library/Android/sdk/platform-tools/adb` → `adb` in PATH
- Linux: `/usr/bin/adb` → `/usr/local/bin/adb` → `adb` in PATH
- Windows: `%USERPROFILE%\AppData\Local\Android\Sdk\platform-tools\adb.exe` → `adb.exe` in PATH

**scrcpy:**
- macOS: `/opt/homebrew/bin/scrcpy` → `scrcpy` in PATH
- Linux: `/usr/bin/scrcpy` → `/usr/local/bin/scrcpy` → `/snap/bin/scrcpy`
- Windows: `%USERPROFILE%\scoop\shims\scrcpy.exe` → `C:\Program Files\scrcpy\scrcpy.exe` → `scrcpy.exe` in PATH

### Output Directories

| Platform | Screenshots | Recordings |
|---|---|---|
| macOS | `~/Pictures/` | `~/Movies/` |
| Windows | `%USERPROFILE%\Pictures\PhoneMirror\` | `%USERPROFILE%\Videos\PhoneMirror\` |
| Linux | `~/Pictures/PhoneMirror/` | `~/Videos/PhoneMirror/` |

## 🔧 Auto-Start

### macOS (SwiftUI version)
```bash
cp macos-swiftui/com.muhaimin.phonemirror.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.muhaimin.phonemirror.plist
```

### macOS (Rust version)
```bash
cat > ~/Library/LaunchAgents/com.muhaimin.phonemirror.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.muhaimin.phonemirror</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/phonemirror</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>
EOF
launchctl load ~/Library/LaunchAgents/com.muhaimin.phonemirror.plist
```

### Linux
```bash
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/phonemirror.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=PhoneMirror
Exec=/usr/local/bin/phonemirror
Hidden=false
EOF
```

### Windows
1. Press `Win+R`, type `shell:startup`, press Enter
2. Create a shortcut to `phonemirror.exe` in that folder

## 🛠️ Development

### Build
```bash
cargo build --release
```

### Cross-compile for Windows (from any platform)
```bash
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

### Cross-compile for Linux (from macOS)
```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

## 🤝 Contributing

1. Fork the repo
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [scrcpy](https://github.com/Genymobile/scrcpy) — The amazing screen mirroring tool that makes this possible
- [tray-item-rs](https://github.com/olback/tray-item-rs) — Cross-platform system tray library for Rust
- [ADB](https://developer.android.com/studio/releases/platform-tools) — Android Debug Bridge