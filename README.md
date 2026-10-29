<div align="center">

# 📱 PhoneMirror

**Mirror your Android phone screen on your desktop**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform: macOS · Windows · Linux](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-green.svg)]()
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rustup.rs)
[![Version: 2.4.1](https://img.shields.io/badge/version-2.4.1-brightgreen.svg)](CHANGELOG.md)

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
| 🎨 **Device hub** | Native device library with Android emulators, USB phones and Wi-Fi devices |
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
| **scrcpy 4.1** | `brew install scrcpy` | `sudo apt install scrcpy` | [Download](https://github.com/Genymobile/scrcpy/releases) |
| **FFmpeg** | `brew install ffmpeg` | `sudo apt install ffmpeg` | [Download](https://ffmpeg.org/download.html) |
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
LOCAL_BUILD=1 ./scripts/bundle-macos.sh
# Builds and verifies dist/PhoneMirror.app without changing the installed app
./scripts/install-macos.sh
# Replaces /Applications/PhoneMirror.app; keeps PhoneMirror.previous.app for rollback
```

## Usage

1. **Open** PhoneMirror to the **Devices** hub.
2. Select an installed Android emulator or a connected USB / Wi-Fi phone in the sidebar.
3. For a shut-down emulator, click **Start emulator** and wait for its ADB connection.
4. Click **Start mirroring here** to view and control the selected device inside PhoneMirror.
   **Save screenshot** and **Record screen** remain available for the selected device.

The list refreshes in the background and includes offline and unauthorized transports.
For USB, enable USB debugging and approve the computer on your phone. Android virtual
devices (AVDs) come from Android Studio Device Manager; install the SDK **Android
Emulator** component to list and start them. SDK discovery checks `ANDROID_HOME`,
`ANDROID_SDK_ROOT`, standard SDK locations and `PATH`. PhoneMirror does not create
AVDs or install system images. The embedded screen does not open a separate scrcpy window.

Selection is preserved on refresh: if a device disconnects, capture controls disable
instead of silently targeting another phone. Changing the selected device stops the
old embedded session. **Stop mirroring** affects only the embedded screen;
The **Info** toolbar button opens device details and tools; **Stop PhoneMirror sessions**
stops this app's screen and recording, not other apps.

### Embedded screen requirements and controls

The embedded client targets **scrcpy 4.1** and its matching **scrcpy-server**, plus
**FFmpeg** on the host for H.264 decoding. Other server protocol versions are rejected
with an actionable error rather than guessed. Install these tools separately; on
macOS, for example, `brew install scrcpy ffmpeg`. Configure `SCRCPY_SERVER_PATH`
if the server is outside standard scrcpy locations. No new tools are downloaded at runtime.

PhoneMirror finds ADB, scrcpy and FFmpeg automatically. For each one it checks, in order:
a custom path set in **Info → Tools**, then `PHONEMIRROR_ADB` / `PHONEMIRROR_SCRCPY` /
`PHONEMIRROR_FFMPEG`, then `ANDROID_HOME` / `ANDROID_SDK_ROOT` and the standard Android SDK
folders (ADB only), then Homebrew, MacPorts and system folders, then `PATH`. It uses the
first copy that actually runs. If different ADB versions are installed, it shows a warning.
Custom paths are saved in `~/Library/Application Support/PhoneMirror/tools.conf` on macOS.
If a tool is missing, the main screen lists its install command.

Click the live screen to focus it, drag to swipe, and type or paste text.
The compact toolbar contains mirror start/stop, screenshot, **Device**, **Fit**, fullscreen
and **Split**. Recording, keyboard forwarding, zoom and information/tools live in **Device**.
Use **Split** to open 2–4 independent device mirrors; choose an online target and Start
in each additional pane. Reducing panes stops the hidden sessions. Hover an icon for its label.
The **Device** menu provides **Wake screen**, **Back**, **Home**, **Recent apps**,
**Power**, volume controls and Wi-Fi pairing. **Wake screen** wakes without toggling
an awake screen off; it does not bypass a PIN or biometric lock.
While mirroring, scrcpy 4.1 keep-active prevents idle sleep on USB or Wi-Fi, even
on battery, without changing Android settings. Normal timeout behavior resumes
when the session ends. Keeping the screen awake increases battery use.
On macOS, **Settings** beside **Device** contains Refresh devices and Wi-Fi pairing.
Other platforms provide a sidebar Settings dropdown.

Open **Tools** in the bottom status bar for **Logcat** and **Diagnostics**. Logcat starts
paused: explicitly choose a target, then Resume. Search, minimum severity and copy act on
the latest snapshot (up to 500 lines / 256 KiB, refreshed every two seconds). Collection
is read-only; it never clears device logs. Diagnostics includes connection health, tool
paths/versions and mirror session errors. Review logs/diagnostics for private data before sharing.
Interactive terminals and arbitrary ADB commands are not included.
Zoom is relative to the fitted screen (50–300%); use the scrollbars or trackpad to pan.
Device swipe/drag gestures are never used for canvas panning.
**Esc** releases keyboard focus. Text injection is limited by Android's supported
characters; full IME, multitouch, clipboard sync and audio playback are not provided.
Video is capped at 1280 pixels / 30 fps. A static screen may not emit new frames.
Portrait/landscape frames keep their aspect ratio.

PhoneMirror uses a uniquely named temporary server JAR and ADB forward for each
embedded session, and attempts cleanup only for its own processes, sockets, forward
and JAR. If the transport is lost, removing the remote temporary JAR may require
reconnecting the device. Screen/input data stays in memory unless you explicitly
capture or record. This implementation has fixture/host-decoder coverage; live device
validation is still required before treating embedded mirroring as release-ready.

### Wireless debugging (Android 11+)

Enable **Developer options → Wireless debugging** and accept your Wi-Fi network.
Open PhoneMirror's separate **Wi-Fi pairing** page. Pairing can continue while you
browse Devices; a connected phone appears in the physical-device list:

- **Pairing code:** On the phone choose **Pair device with pairing code**. Enter
  that dialog's **IP address:port** and six-digit code in PhoneMirror, then click
  **Pair this phone**. Keep the phone's dialog open until pairing finishes.
- **QR code:** Click **Generate pairing QR**. On the phone choose
  **Pair device with QR code** and scan the laptop's QR. QR sessions expire after
  two minutes; cancel and regenerate to retry. Do not share the QR: it authorizes
  debugging access. The laptop does not scan a QR from the phone.
- **Already paired:** Pairing attempts automatic connection. If paired but not connected,
  close the phone's pairing dialog and enter **IP address & port from the main
  Wireless debugging page**, then click **Connect phone**. This is a **different
  port** from the pairing port; do not assume port 5555. Ports can change.

Use **Run connection diagnostics** to see ADB version, mDNS discovery, and
network guidance. Same Wi-Fi name does not guarantee connectivity: guest Wi-Fi
client isolation, VPNs, local-network permissions, or firewalls can block traffic.
QR pairing needs mDNS; manual IP/code pairing bypasses discovery. Use current
Android SDK Platform-Tools. No USB connection or `adb tcpip` step is required.
Pairing secrets stay in memory and are passed to ADB via stdin, not shell arguments.
ADB itself stores its normal trusted-host keys; cancelling does not unpair a phone
if pairing already succeeded. To revoke access, forget the laptop on the phone.

### Troubleshooting and release readiness

If pairing reports **protocol fault**, expand **Technical details** and run diagnostics.
The ADB client can report a generic protocol error when its server cannot reach the
phone. **No route to host** means the network path failed before code verification:
check the current phone IP, Wi-Fi client isolation, and macOS **Privacy & Security →
Local Network** permissions for the app hosting ADB. Do not delete trust keys or
turn off the firewall globally. The connection port and pairing port are different. When available, PhoneMirror
checks only newly appended ADB server-log bytes for the exact pairing endpoint
and exposes a fixed network reason, not raw logs or historical failures.

The desktop workspace has guided Samsung setup, readable errors/copyable details,
background device checks, a full-size QR, and session controls gated on an online
device. The macOS bundle declares local-network usage and Bonjour service types;
this does not itself grant network permission to an external ADB server.

**Not yet a commercial release:** the local build is ad-hoc signed. Before selling,
validate real-device pairing across Android/OEM/network combinations, package or
clearly onboard ADB/scrcpy dependencies, review dependency redistribution licenses,
use Developer ID signing/notarization, and complete installer/update, privacy,
accessibility, and support testing. The redesign is not proof of release readiness.

### Keyboard input (embedded screen)
- Click the live image to focus device input; press `Esc` to release focus.
- Enter, Backspace, Delete and arrow keys are sent to the focused device.
- Use the visible capture, recording and Android navigation buttons. Desktop scrcpy shortcuts are not forwarded.

## Architecture

```
┌─────────────────────────────────┐
│         PhoneMirror GUI         │
│         (egui + Rust)           │
├────────────────┬────────────────┤
│      ADB       │ scrcpy-server  │
│  (discovery,   │ H264 + control │
│   screenshot)  │ FFmpeg → egui  │
└────────────────┴────────────────┘
```

PhoneMirror is a **native desktop GUI** built with [egui](https://github.com/emilk/egui) — no WebView, no Electron, no browser dependency. It renders the scrcpy-server H.264 stream through FFmpeg into an egui texture, sends focused input through the scrcpy control socket, wraps ADB discovery/capture and scrcpy recording, and handles the `.app` bundle PATH issue on macOS.

## Why egui instead of Tauri/Electron?

Tauri (v2.11) uses WKWebView which [crashes on macOS 26 (Tahoe)](https://github.com/tauri-apps/wry/issues/1576). egui renders natively via OpenGL/Metal — no browser needed, so it works on all macOS versions including Tahoe.

## Project Structure

```
phonemirror-app/
├── src/
│   ├── main.rs                # App state, discovery workers and capture
│   ├── desktop_ui.rs          # Device hub and Wi-Fi page
│   ├── devices.rs             # ADB + SDK emulator inventory
│   ├── mirror.rs              # Owned video/control transport + decoder
│   ├── mirror_ui.rs           # Native texture and focused input
│   ├── wireless.rs            # TLS pairing protocol and bounded ADB
│   └── wireless_ui.rs         # Guided pairing workspace
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