//! Headless workspace integration and layout regression tests.
use super::*;
use crate::app::devices::{Kind, State};
use crate::app::tools;
use crate::app::ui::{
    components::{
        capsule::{item, pill},
        device_row::device_row,
        icons::Icon,
    },
    theme::{configure_theme, PILL_H},
};
use egui::{vec2, Rect};
fn sample(id: &str, name: &str, kind: Kind, state: State) -> Device {
    Device {
        id: id.into(),
        name: name.into(),
        serial: (state != State::Stopped).then(|| id.into()),
        avd: (kind == Kind::Emulator).then(|| name.into()),
        kind,
        state,
    }
}

/// Headless paint-command fingerprint: no ADB, window or physical device access.
#[test]
fn workspace_paint_fingerprints() {
    use std::hash::{Hash, Hasher};
    let mut baseline = Vec::new();
    for pass in 0..2 {
        for width in [720.0, 943.0, 1060.0, 1440.0] {
            for scenario in 0..8 {
                let ctx = egui::Context::default();
                configure_theme(&ctx);
                let mut app = PhoneMirrorApp::default();
                app.device_status.checked = true;
                app.inventory.devices = vec![
                    sample("usb", "Pixel 9", Kind::Usb, State::Online),
                    sample("avd", "Pixel Tablet", Kind::Emulator, State::Stopped),
                    sample("wifi", "Galaxy", Kind::Wifi, State::Unauthorized),
                ];
                app.sync_selection();
                match scenario {
                    1 => app.sidebar_open = false,
                    2 => {
                        app.hide_unavailable = true;
                        app.device_filter = "Pixel".into();
                    }
                    3 => app.page = Page::Wifi,
                    4 => app.show_device_info = true,
                    5 => app.split.set_count(3),
                    6 => app.drawer.open = true,
                    7 => ctx.memory_mut(|m| m.toggle_popup(egui::Id::new("more_menu"))),
                    _ => {}
                }
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                for frame in 0..3 {
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                egui::Pos2::ZERO,
                                vec2(width, 820.0),
                            )),
                            time: Some(frame as f64 / 60.0),
                            ..Default::default()
                        },
                        |ctx| app.show_workspace(ctx),
                    );
                    format!("{:?}", output.shapes).hash(&mut hash);
                }
                let fingerprint = hash.finish();
                if pass == 0 {
                    baseline.push(fingerprint);
                } else {
                    let index = [720.0, 943.0, 1060.0, 1440.0]
                        .iter()
                        .position(|w| *w == width)
                        .unwrap()
                        * 8
                        + scenario;
                    assert_eq!(
                        fingerprint, baseline[index],
                        "non-deterministic paint at {width}, scenario {scenario}"
                    );
                }
            }
        }
    }
}

#[test]
fn toolbar_controls_share_height_and_centerline() {
    for enabled in [false, true] {
        let ctx = egui::Context::default();
        configure_theme(&ctx);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut rects = Vec::new();
                ui.horizontal(|ui| {
                    for count in [2, 4, 2, 3] {
                        let entries: Vec<_> = (0..count)
                            .map(|i| {
                                item(
                                    [Icon::Keyboard, Icon::Split, Icon::ZoomFit, Icon::Info][i],
                                    ["Keyboard", "Split", "Fit", "Info"][i],
                                    enabled,
                                    false,
                                )
                            })
                            .collect();
                        let group = ui.push_id(rects.len(), |ui| pill(ui, &entries)).inner;
                        rects.extend(group.into_iter().map(|r| r.rect));
                    }
                });
                assert_eq!(rects.len(), 11);
                for rect in &rects {
                    assert!((rect.height() - PILL_H).abs() < 0.1, "height: {rect:?}");
                    assert!(
                        (rect.center().y - rects[0].center().y).abs() < 0.1,
                        "alignment: {rects:?}"
                    );
                }
            });
        });
    }
}

#[test]
fn compact_toolbar_stays_under_32_points() {
    let ctx = egui::Context::default();
    configure_theme(&ctx);
    let mut app = PhoneMirrorApp::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.scope(|ui| app.device_toolbar(ui));
            assert!(
                response.response.rect.height() <= 32.0,
                "toolbar height: {}",
                response.response.rect.height()
            );
        });
    });
}

#[test]
fn split_workspace_fits_minimum_window() {
    for width in [720.0, 1060.0, 1440.0] {
        for extra in 1..=3 {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            let mut app = PhoneMirrorApp::default();
            app.split.set_count(extra);
            for _ in 0..2 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 820.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.show_workspace(ctx),
                );
                for shape in output.shapes {
                    let rect = shape.shape.visual_bounding_rect();
                    if rect.is_finite() && rect.is_positive() {
                        assert!(
                            rect.right() <= width + 8.0,
                            "split {extra} overflow at {width}: {rect:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn sidebar_and_toolbar_fit_when_collapsed_or_filtered() {
    for width in [720.0, 943.0, 1060.0] {
        for sidebar in [false, true] {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            let mut app = PhoneMirrorApp::default();
            app.sidebar_open = sidebar;
            app.hide_unavailable = true;
            app.device_filter = "Pixel".into();
            app.inventory.devices = vec![sample("one", "Pixel 9", Kind::Usb, State::Online)];
            app.sync_selection();
            for _ in 0..2 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(width, 620.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.show_workspace(ctx),
                );
                for shape in output.shapes {
                    let rect = shape.shape.visual_bounding_rect();
                    if rect.is_finite() && rect.is_positive() {
                        assert!(
                            rect.right() <= width + 8.0,
                            "sidebar={sidebar}, width={width}: {rect:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn hub_and_pairing_render_without_horizontal_overflow() {
    for width in [720.0, 940.0, 1060.0, 1440.0] {
        for page in [Page::Devices, Page::Wifi] {
            for selection in [
                None,
                Some("avd:Pixel"),
                Some("emulator-5554"),
                Some("USB123"),
                Some("192.168.1.7:33445"),
            ] {
                let ctx = egui::Context::default();
                configure_theme(&ctx);
                let mut app = PhoneMirrorApp::default();
                app.page = page;
                if selection.is_some() {
                    app.inventory.devices = vec![
                        sample("avd:Pixel", "Pixel 9 Pro", Kind::Emulator, State::Stopped),
                        sample(
                            "emulator-5554",
                            "Pixel Tablet",
                            Kind::Emulator,
                            State::Online,
                        ),
                        sample("USB123", "Samsung Galaxy", Kind::Usb, State::Unauthorized),
                        sample("192.168.1.7:33445", "Pixel 8", Kind::Wifi, State::Offline),
                    ];
                    app.selected_device = selection.map(str::to_owned);
                    app.sync_selection();
                }
                for _ in 0..2 {
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, 820.0),
                            )),
                            ..Default::default()
                        },
                        |ctx| app.show_workspace(ctx),
                    );
                    for shape in &output.shapes {
                        let rect = shape.shape.visual_bounding_rect();
                        if rect.is_finite() && rect.is_positive() {
                            assert!(
                                rect.right() <= width + 8.0,
                                "overflow at width {width}, {page:?}: {rect:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn device_rows_support_keyboard_activation() {
    let ctx = egui::Context::default();
    configure_theme(&ctx);
    let device = sample("usb", "Pixel 9", Kind::Usb, State::Online);
    let mut activated = false;
    for frame in 0..2 {
        let _ = ctx.run(
            egui::RawInput {
                events: if frame == 1 {
                    vec![egui::Event::Key {
                        key: egui::Key::Enter,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }]
                } else {
                    vec![]
                },
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = device_row(ui, &device, false, false);
                    if frame == 0 {
                        response.request_focus();
                    }
                    activated |= response.clicked();
                });
            },
        );
    }
    assert!(activated, "Focused device rows must activate with Enter");
}

#[test]
fn long_identifiers_and_active_error_states_fit_minimum_window() {
    for page in [Page::Devices, Page::Wifi] {
        let ctx = egui::Context::default();
        configure_theme(&ctx);
        let mut app = PhoneMirrorApp::default();
        app.page = page;
        app.mirror_running = true;
        app.is_recording = true;
        app.status_message = "Recording is active on this computer.".into();
        app.show_device_info = true;
        let long = "adb-LongDeviceName (2)._adb-tls-connect._tcp".repeat(5);
        app.inventory.devices = vec![sample(
            &long,
            &"Pixel Android Emulator ".repeat(10),
            Kind::Wifi,
            State::Online,
        )];
        app.device_status.scrcpy_health =
            "Cannot open scrcpy. Check the installed version and dependencies.".into();
        app.sync_selection();
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 620.0),
                    )),
                    ..Default::default()
                },
                |ctx| app.show_workspace(ctx),
            );
            for shape in output.shapes {
                let rect = shape.shape.visual_bounding_rect();
                if rect.is_finite() && rect.is_positive() {
                    assert!(rect.right() <= 728.0, "long content overflow: {rect:?}");
                }
            }
        }
    }
}

#[test]
fn embedded_screen_states_fit_hub_without_external_window() {
    for width in [720.0, 1060.0, 1440.0] {
        for (state, dimensions) in [
            (crate::app::mirror::Status::Starting, [2, 3]),
            (crate::app::mirror::Status::Streaming, [2, 3]),
            (crate::app::mirror::Status::Streaming, [3, 2]),
            (
                crate::app::mirror::Status::Failed(
                    "FFmpeg cannot decode this screen. Check the matching scrcpy server.".into(),
                ),
                [2, 3],
            ),
        ] {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            let mut app = PhoneMirrorApp::default();
            app.inventory.devices =
                vec![sample("fixture", "Pixel 9", Kind::Emulator, State::Online)];
            app.sync_selection();
            app.embedded = crate::app::mirror_ui::MirrorUi::fixture(
                "fixture",
                state,
                Some(crate::app::mirror::Frame {
                    width: dimensions[0],
                    height: dimensions[1],
                    rgb: vec![100; (dimensions[0] * dimensions[1] * 3) as usize],
                }),
            );
            app.embedded.poll(&ctx);
            for _ in 0..2 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 620.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.show_workspace(ctx),
                );
                for shape in output.shapes {
                    let rect = shape.shape.visual_bounding_rect();
                    if rect.is_finite() && rect.is_positive() {
                        assert!(
                            rect.right() <= width + 8.0,
                            "embedded overflow {width}: {rect:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn setup_checklist_and_tools_panel_fit_minimum_window() {
    let found = |path: &str, version: &str| tools::Resolution {
        path: Some(path.into()),
        source: "the Android SDK".into(),
        version: version.into(),
        warnings: vec![format!(
            "Different ADB versions are installed: {}",
            path.repeat(6)
        )],
    };
    let report = tools::Report {
        tools: [
            found(
                "/Users/someone/Library/Android/sdk/platform-tools/adb",
                "37.0.1",
            ),
            tools::Resolution::default(),
            found("/opt/homebrew/bin/ffmpeg", "8.0"),
        ],
    };
    for width in [720.0, 1060.0] {
        let ctx = egui::Context::default();
        configure_theme(&ctx);
        let mut app = PhoneMirrorApp::default();
        app.device_status.tools = Some(report.clone());
        app.tool_inputs = Some(Default::default());
        app.show_device_info = true;
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 620.0),
                    )),
                    ..Default::default()
                },
                |ctx| app.show_workspace(ctx),
            );
            for shape in output.shapes {
                let rect = shape.shape.visual_bounding_rect();
                if rect.is_finite() && rect.is_positive() {
                    assert!(
                        rect.right() <= width + 8.0,
                        "tools overflow {width}: {rect:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn selected_device_controls_target_and_disconnect_never_redirects() {
    let mut app = PhoneMirrorApp::default();
    app.inventory.devices = vec![
        sample("usb-a", "Phone A", Kind::Usb, State::Online),
        sample("usb-b", "Phone B", Kind::Usb, State::Online),
    ];
    app.selected_device = Some("usb-b".into());
    app.sync_selection();
    assert_eq!(app.device_status.device_serial, "usb-b");
    app.inventory.devices.pop();
    app.sync_selection();
    assert!(!app.device_status.connected);
    assert!(app.device_status.device_serial.is_empty());
    assert_eq!(app.selected_device.as_deref(), Some("usb-b"));
}

#[test]
fn stopped_and_unauthorized_selections_disable_actions() {
    for state in [State::Stopped, State::Offline, State::Unauthorized] {
        let mut app = PhoneMirrorApp::default();
        app.inventory.devices = vec![sample("one", "One", Kind::Emulator, state)];
        app.sync_selection();
        assert!(!app.device_status.connected);
        assert!(app.device_status.device_serial.is_empty());
    }
}
