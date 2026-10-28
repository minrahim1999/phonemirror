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
- [Rust](https://rustup.rs/) (current stable)
- [ADB](https://developer.android.com/tools/releases/platform-tools) — Android Debug Bridge
- [scrcpy](https://github.com/Genymobile/scrcpy) — Screen mirroring tool

### Build & Run
```bash
# Debug build (fast compile, slow runtime)
cargo build

# Release build (slow compile, fast runtime)
cargo build --release --locked

# Run
cargo run --release
```

### macOS .app Bundle
```bash
cargo build --release
./scripts/bundle-macos.sh
```

## Project Structure

```text
phonemirror/
├── src/
│   ├── main.rs                # Minimal executable entry point
│   └── app/
│       ├── mod.rs             # Application state and lifecycle
│       ├── android/           # Device inventory, wireless pairing, mirror protocol
│       ├── ui/                # Dashboard, pairing and screen/input presentation
│       └── platform/          # Host tool discovery and native menus
├── assets/macos/              # App icon
├── packaging/macos/           # Signing entitlements
├── scripts/                   # Bundle, guarded install, icon generation
├── docs/                      # Architecture and release procedure
├── .github/workflows/         # Tested, version-checked release builds
├── Cargo.toml                 # Package metadata and dependencies
├── Cargo.lock                 # Locked dependency resolution
└── dist/                      # Generated signed bundle (ignored)
```

## Verification and releases

Run `cargo test --locked`, `cargo clippy --locked --all-targets`, and
`cargo build --release --locked`. Tests live beside their modules.
See [release procedure](docs/RELEASING.md) before installing or tagging.

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
