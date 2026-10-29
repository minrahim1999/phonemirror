# Changelog

All notable changes to PhoneMirror will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.5.0] - 2026-10-29

### Added
- Split workspace for up to four independent device mirrors, with per-pane selection, controls and cleanup.
- Expandable Tools drawer with explicit-target Logcat snapshots, search, severity filters, pause/resume and copy.
- Copyable diagnostics covering tool paths/versions, connection health and mirror session errors.
- Wake screen action and session-scoped keep-active support without persistent Android setting changes.
- Wireless connection discovery with explicit endpoint selection and targeted disconnect that preserves pairing.

### Changed
- Compact mirroring toolbar; secondary actions live in the Device menu.
- Launcher icon in the sidebar and Refresh/Wi-Fi pairing in the macOS Settings menu.

### Limitations
- Logcat is read-only and bounded to the latest 500 lines / 256 KiB per snapshot; interactive terminals are not included.
- Automated fixtures do not establish live-device or multi-device E2E acceptance. macOS builds remain ad-hoc signed, not notarized.

## [2.4.1] - 2026-10-28

### Fixed
- Corrected the tool-discovery test that blocked Windows release builds by treating valid Scoop and WinGet paths under the user home directory as Android SDK paths.
- Added coverage for all SDK roots while preserving Windows package-manager discovery assertions.

## [2.4.0] - 2026-10-28

### Added
- Embedded screen canvas with focused keyboard/touch control and fit-relative zoom.
- Android emulator inventory, guided wireless pairing and configurable tool discovery.
- macOS native application menus and actionable connection diagnostics.

### Changed
- Organized application internals into Android, UI and platform modules with a minimal executable entry point.
- Moved icons and entitlements into dedicated asset/packaging folders; generated bundles now stage under `dist/`.
- Added verified macOS installation with a retained rollback copy.
- Release builds use locked dependencies, test gates and tag/package version consistency checks.

### Limitations
- Ad-hoc signed macOS builds are not notarized. Device E2E and other platform builds require separate acceptance.

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