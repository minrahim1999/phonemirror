# PhoneMirror v2.0.0

Cross-platform desktop app to mirror and control Android phones via ADB + scrcpy.

## Features
- 📱 Screen mirroring with scrcpy
- 📸 Screenshot capture
- 🎬 Screen recording
- 🔌 Auto device detection
- 🖥️ Native GUI (egui — no WebView needed)

## Prerequisites
- [ADB](https://developer.android.com/tools/releases/platform-tools) — Android Debug Bridge
- [scrcpy](https://github.com/Genymobile/scrcpy) — Screen mirroring tool
- USB debugging enabled on your phone

### macOS
```bash
brew install android-platform-tools scrcpy
# Or install Android SDK and add to PATH
```

### Linux
```bash
sudo apt install adb scrcpy  # Debian/Ubuntu
```

### Windows
Download [ADB](https://developer.android.com/studio/releases/platform-tools) and [scrcpy](https://github.com/Genymobile/scrcpy/releases), add both to PATH.

## Build
```bash
cargo build --release
```

Binary: `target/release/phonemirror`

### macOS .app Bundle
```bash
./scripts/bundle-macos.sh
```

Creates `PhoneMirror.app` in `/Applications/`.

## Usage
1. Connect your phone via USB (with USB debugging enabled)
2. Open PhoneMirror
3. Click **Start Mirror** to begin screen mirroring
4. Use **Screenshot** or **Start Recording** as needed

## Architecture
- **Rust** with **egui** for native cross-platform GUI
- No WebView/HTML dependency — works on macOS 26 (Tahoe) and all platforms
- Auto-detects ADB and scrcpy paths
- Full PATH inheritance for subprocess commands (fixes .app bundle PATH issue)

## License
MIT
