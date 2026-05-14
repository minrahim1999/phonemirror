# Contributing to PhoneMirror

First off, thanks for taking the time to contribute! 🎉

## Ways to Contribute

- **Bug reports** — Open an issue with steps to reproduce
- **Feature requests** — Open an issue with the `enhancement` label
- **Code** — Fork, branch, commit, push, PR
- **Documentation** — Typos, clarifications, translations all welcome

## Development Setup

1. Install [Rust](https://rustup.rs/)
2. Install [scrcpy](https://github.com/Genymobile/scrcpy) and ADB
3. Clone and build:
   ```bash
   git clone https://github.com/minrahim1999/phonemirror.git
   cd phonemirror
   cargo build
   ```

## Pull Request Process

1. Make sure `cargo build` and `cargo test` pass
2. Update the README if you change behavior
3. Keep PRs focused — one feature or fix per PR
4. Write clear commit messages

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings

## Reporting Bugs

When filing an issue, please include:

- **OS and version** (macOS 14, Windows 11, Ubuntu 24.04, etc.)
- **Phone model** and Android version
- **scrcpy version** (`scrcpy --version`)
- **ADB version** (`adb version`)
- Steps to reproduce
- Expected vs actual behavior

Thanks! 💚