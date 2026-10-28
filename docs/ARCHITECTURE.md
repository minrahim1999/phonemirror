# Architecture

PhoneMirror is one native Rust/egui application, not a web app or a multi-crate workspace.

- `src/main.rs` delegates startup to `app::run`.
- `src/app/mod.rs` owns application state, refresh jobs, capture/recording and cleanup.
- `src/app/android/` owns ADB transport inventory, wireless pairing and embedded scrcpy protocol/decoder sessions.
- `src/app/ui/` owns egui presentation and focused mirror input.
- `src/app/platform/` owns external tool discovery/configuration and macOS menus.

The app module coordinates these layers. Child modules remain internal; grouping does not change device targeting, protocol or UI contracts. Existing unit, fixture, loopback and headless rendering tests stay alongside their implementation. ADB and scrcpy/FFmpeg are external tools, not bundled dependencies.

Source assets live under `assets/`; platform packaging under `packaging/`; generated Rust builds under `target/`; signed app bundles under ignored `dist/`. Scripts resolve the repository from their own location.
