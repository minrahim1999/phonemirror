# Release procedure

1. Review all changes, including pre-existing work. Set the package version in `Cargo.toml`, synchronize `Cargo.lock`, and update `CHANGELOG.md`.
2. Run `cargo test --locked`, `cargo clippy --locked --all-targets`, `cargo build --release --locked`, and `actionlint .github/workflows/release.yml`.
3. On macOS run `LOCAL_BUILD=1 ./scripts/bundle-macos.sh`. It stages and verifies `dist/PhoneMirror.app` before replacing the previous staged output. Signing is ad-hoc, not notarized distribution signing.
4. Quit PhoneMirror; run `./scripts/install-macos.sh`. It refuses running apps/symlinks and an existing rollback copy, verifies a temporary copy before replacement, and keeps the old installation at `/Applications/PhoneMirror.previous.app`. Never remove that backup without checking ownership and rollback needs.
5. Verify installed signature with `codesign --verify --deep --strict /Applications/PhoneMirror.app` and compare `dist/PhoneMirror.app` with the installed bundle using `diff -rq`. Perform manual app acceptance, including mirroring/capture as needed. Automated fixture tests do not establish device E2E; never automate physical devices.
6. Only after acceptance, commit the reviewed release changes and create an annotated `v<version>` tag. Push the branch and that exact tag (never force-push). The tag starts release CI for macOS, Linux and Windows; each platform must test/build before publication. Remote CI is not established by local macOS checks.

The release workflow checks that the tag exactly matches Cargo metadata and derives package versions from that metadata. The current bundle is host-architecture only; it is not a universal macOS binary. Runtime requirements include matching scrcpy 4.1 server and FFmpeg. See README release-readiness limitations.
