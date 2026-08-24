# Changelog

All notable changes to PhoneMirror will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0] - 2026-08-24

### Added
- **scrcpy health check**: the app now probes `scrcpy --version` on each device
  refresh and shows an actionable warning card in Device Status when scrcpy is
  present but cannot launch (e.g. broken Homebrew linkage after an ffmpeg major
  upgrade, which previously surfaced only as a bare "Mirror failed (exited 1)").

### Changed
- Mirror and recording failures now capture and display the real scrcpy stderr
  (e.g. `dyld: Library not loaded: .../libavformat.62.dylib`) instead of a
  generic exit-code message.

## [2.0.0] - 2026-05-20

### Added
- **Native GUI dashboard** using egui (Rust immediate-mode GUI)
- Screen mirroring via scrcpy with one-click start/stop
- Screenshot capture via ADB (`screencap`) — saved to ~/Pictures
- Screen recording via scrcpy (`--record`) — saved to ~/Movies
- Auto device detection (ADB `devices` polling every 5 seconds)
- Auto-detection of ADB and scrcpy paths across platforms
- Full PATH inheritance for subprocess commands (fixes .app bundle PATH issue)
- macOS .app bundle creation script with code signing
- Cross-platform support: macOS, Windows, Linux
- Dark theme UI with cards, color-coded status, and pulsing indicators

### Changed
- **Architecture**: Rebuilt from Tauri (WebView) to egui (native rendering)
  - Tauri's WKWebView crashes on macOS 26 (Tahoe) — see [wry#1576](https://github.com/tauri-apps/wry/issues/1576)
  - egui renders natively, no browser dependency needed

### Removed
- Tauri/WebView dependency (replaced with egui)
- System tray icon (replaced with proper windowed GUI)
- HTML/CSS/JS frontend (replaced with native Rust GUI)

## [1.0.0] - 2026-05-19

### Added
- Initial PhoneMirror concept with system tray menu
- Basic scrcpy launch from tray
- macOS .app bundle with PATH fix

### Fixed
- ADB/scrcpy PATH resolution when launching from .app bundle

[2.0.0]: https://github.com/minrahim1999/phonemirror/releases/tag/v2.0.0
[1.0.0]: https://github.com/minrahim1999/phonemirror/tree/v1.0.0