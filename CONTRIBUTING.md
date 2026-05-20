# Contributing to PhoneMirror

Thank you for your interest in contributing! 🎉

## Quick Start

1. **Fork** the repository
2. **Create a branch**: `git checkout -b feature/your-feature`
3. **Make your changes** and test them
4. **Commit**: `git commit -m "Add your feature"`
5. **Push**: `git push origin feature/your-feature`
6. **Open a Pull Request**

## Development Setup

### Prerequisites
- [Rust](https://rustup.rs/) (1.70+)
- [ADB](https://developer.android.com/tools/releases/platform-tools) — Android Debug Bridge
- [scrcpy](https://github.com/Genymobile/scenecap) — Screen mirroring tool

### Build & Run
```bash
# Debug build (fast compile, slow runtime)
cargo build

# Release build (slow compile, fast runtime)
cargo build --release

# Run
cargo run --release
```

### macOS .app Bundle
```bash
cargo build --release
./scripts/bundle-macos.sh
```

## Project Structure

```
phonemirror-app/
├── src/
│   └── main.rs          # All app logic (egui + ADB/scrcpy commands)
├── scripts/
│   └── bundle-macos.sh  # Creates .app bundle with code signing
├── phonemirror.entitlements  # macOS entitlements for codesign
├── Cargo.toml           # Rust dependencies
├── LICENSE              # MIT
├── CHANGELOG.md         # Version history
├── CONTRIBUTING.md      # This file
└── README.md            # Project documentation
```

## Code Style

- Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- Keep the UI responsive — all ADB/scrcpy calls are non-blocking
- Test on macOS first, then verify cross-platform compatibility

## Reporting Bugs

1. Include your OS version (e.g., macOS 26.4.1, Ubuntu 24.04)
2. Include ADB and scrcpy versions (`adb version`, `scrcpy --version`)
3. Include PhoneMirror version
4. Describe steps to reproduce

## Feature Requests

Open an issue with the label `enhancement`. Keep in mind:

- PhoneMirror is a **desktop companion** for ADB + scrcpy — not a replacement
- Features should work across macOS, Windows, and Linux
- The GUI uses egui — features must be achievable with immediate-mode rendering

## License

By contributing, you agree that your code will be licensed under the [MIT License](LICENSE).