//! Read-only device logs and host diagnostics; no interactive command execution.
use crate::app::{android::logcat, devices, tools, PhoneMirrorApp, TEXT_DIM};
use std::time::{Duration, Instant};

pub(crate) struct ToolsDrawer {
    pub open: bool,
    diagnostics: bool,
    serial: String,
    text: String,
    error: String,
    query: String,
    severity: usize,
    paused: bool,
    last: Option<Instant>,
    reader: logcat::Reader,
}
impl Default for ToolsDrawer {
    fn default() -> Self {
        Self {
            open: false,
            diagnostics: false,
            serial: String::new(),
            text: String::new(),
            error: String::new(),
            query: String::new(),
            severity: 0,
            paused: true,
            last: None,
            reader: logcat::Reader::default(),
        }
    }
}
const LEVELS: [&str; 7] = [
    "All", "Verbose", "Debug", "Info", "Warning", "Error", "Fatal",
];
fn matches(line: &str, query: &str, severity: usize) -> bool {
    let level = line
        .as_bytes()
        .first()
        .and_then(|b| b"VDIWEF".iter().position(|c| c == b));
    line.to_lowercase().contains(&query.to_lowercase())
        && (severity == 0 || level.is_some_and(|level| level + 1 >= severity))
}
impl ToolsDrawer {
    fn accept(&mut self, serial: String, result: Result<String, String>) {
        if serial != self.serial || self.paused || !self.open || self.diagnostics {
            return;
        }
        match result {
            Ok(text) => {
                self.text = text;
                self.error.clear();
            }
            Err(error) => self.error = error,
        }
    }
}
impl PhoneMirrorApp {
    pub(crate) fn show_tools_drawer(&mut self, ctx: &egui::Context) {
        if let Some((serial, result)) = self.drawer.reader.poll() {
            self.drawer.accept(serial, result);
        }
        if !self.drawer.open {
            return;
        }
        let diagnostic = self.drawer_diagnostics();
        egui::TopBottomPanel::bottom("tools_drawer")
            .resizable(true)
            .default_height(230.0)
            .min_height(130.0)
            .max_height((ctx.screen_rect().height() * 0.45).max(130.0))
            .show(ctx, |ui| {
                ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
                ui.spacing_mut().interact_size.y = 26.0;
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.drawer.diagnostics, false, "Logcat");
                    ui.selectable_value(&mut self.drawer.diagnostics, true, "Diagnostics");
                });
                if self.drawer.diagnostics {
                    if ui.small_button("Copy diagnostics").clicked() {
                        ui.ctx().copy_text(diagnostic.clone());
                    }
                    egui::ScrollArea::both()
                        .id_salt("diagnostic_text")
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(&diagnostic).monospace())
                                    .wrap(),
                            );
                        });
                    return;
                }
                let drawer = &mut self.drawer;
                let previous = drawer.serial.clone();
                ui.horizontal_wrapped(|ui| {
                    ui.label("Target");
                    egui::ComboBox::from_id_salt("logcat_target")
                        .width(190.0)
                        .selected_text(if drawer.serial.is_empty() {
                            "Choose device"
                        } else {
                            &drawer.serial
                        })
                        .truncate()
                        .show_ui(ui, |ui| {
                            for device in &self.inventory.devices {
                                if device.state == devices::State::Online {
                                    if let Some(serial) = &device.serial {
                                        ui.selectable_value(
                                            &mut drawer.serial,
                                            serial.clone(),
                                            format!("{} · {}", device.name, serial),
                                        );
                                    }
                                }
                            }
                        });
                    if ui
                        .button(if drawer.paused { "Resume" } else { "Pause" })
                        .clicked()
                    {
                        drawer.paused = !drawer.paused;
                    }
                    egui::ComboBox::from_id_salt("logcat_level")
                        .width(85.0)
                        .selected_text(LEVELS[drawer.severity])
                        .show_ui(ui, |ui| {
                            for (index, label) in LEVELS.iter().enumerate() {
                                ui.selectable_value(&mut drawer.severity, index, *label);
                            }
                        });
                    ui.add(
                        egui::TextEdit::singleline(&mut drawer.query)
                            .hint_text("Search logs")
                            .desired_width(130.0),
                    );
                });
                if previous != drawer.serial {
                    drawer.text.clear();
                    drawer.error.clear();
                    drawer.last = None;
                    drawer.paused = true;
                }
                let online = self.inventory.devices.iter().any(|device| {
                    device.state == devices::State::Online
                        && device.serial.as_deref() == Some(drawer.serial.as_str())
                });
                if !online {
                    drawer.paused = true;
                }
                let filtered = drawer
                    .text
                    .lines()
                    .filter(|line| matches(line, &drawer.query, drawer.severity))
                    .collect::<Vec<_>>()
                    .join("\n");
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("Copy visible logs").clicked() {
                        ui.ctx().copy_text(filtered.clone());
                    }
                    ui.label(
                        egui::RichText::new(if !online {
                            "Choose an online target. Collection is paused."
                        } else if drawer.paused {
                            "Paused · latest snapshot retained"
                        } else {
                            "Latest 500 lines · refreshes every 2 seconds"
                        })
                        .small()
                        .color(TEXT_DIM),
                    );
                });
                if !drawer.error.is_empty() {
                    ui.colored_label(crate::app::RED, &drawer.error);
                }
                if !drawer.paused && online {
                    if !drawer.reader.busy()
                        && drawer
                            .last
                            .is_none_or(|last| last.elapsed() >= Duration::from_secs(2))
                    {
                        drawer.reader.request(
                            ctx,
                            crate::app::adb_path(),
                            drawer.serial.clone(),
                            crate::app::build_full_env(),
                        );
                        drawer.last = Some(Instant::now());
                    }
                    ctx.request_repaint_after(Duration::from_millis(200));
                }
                egui::ScrollArea::both()
                    .id_salt("logcat_text")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.label(
                                "No matching logs. Select a target and Resume to read Logcat.",
                            );
                        } else {
                            ui.add(
                                egui::Label::new(egui::RichText::new(filtered).monospace())
                                    .selectable(true),
                            );
                        }
                    });
            });
    }
    fn drawer_diagnostics(&self) -> String {
        let mut text = format!(
            "PhoneMirror {}
Primary target: {}
Online: {}
Primary mirror active: {}
Additional mirror active: {}
ADB health: {}
Mirror health: {}
Status: {}
",
            env!("CARGO_PKG_VERSION"),
            self.device_status.device_serial,
            self.device_status.connected,
            self.embedded.active(),
            self.split.active(),
            self.device_status.adb_health,
            self.device_status.scrcpy_health,
            self.status_message
        );
        text.push_str(&format!(
            "Primary session: {}\n{}\n",
            self.embedded.diagnostic(),
            self.split.diagnostics()
        ));
        if let Some(report) = &self.device_status.tools {
            for tool in tools::ALL {
                let found = report.get(tool);
                text.push_str(&format!(
                    "
{}: {}
Version: {}
Source: {}
",
                    tool.label(),
                    found.path.as_deref().unwrap_or("Not found"),
                    found.version,
                    found.source
                ));
                for warning in &found.warnings {
                    text.push_str(&format!(
                        "Warning: {warning}
"
                    ));
                }
            }
        } else {
            text.push_str(
                "Tool discovery pending.
",
            );
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_targets_and_paused_results_never_replace_snapshot() {
        let mut drawer = ToolsDrawer {
            open: true,
            paused: false,
            serial: "two".into(),
            text: "retained".into(),
            ..Default::default()
        };
        drawer.accept("one".into(), Ok("wrong phone".into()));
        assert_eq!(drawer.text, "retained");
        drawer.paused = true;
        drawer.accept("two".into(), Ok("late result".into()));
        assert_eq!(drawer.text, "retained");
        drawer.paused = false;
        drawer.accept("two".into(), Ok("current".into()));
        assert_eq!(drawer.text, "current");
    }
    #[test]
    fn diagnostics_include_session_failure_without_running_commands() {
        let mut app = PhoneMirrorApp::default();
        app.embedded = super::super::mirror_ui::MirrorUi::fixture(
            "fixture",
            crate::app::mirror::Status::Failed("decoder failed".into()),
            None,
        );
        assert!(app.drawer_diagnostics().contains("decoder failed"));
    }
    #[test]
    fn filters_brief_severity_and_case_insensitive_search() {
        assert!(matches("E/Example( 42): Broken", "broken", 5));
        assert!(!matches("I/Example( 42): ready", "", 5));
        assert!(!matches("--------- beginning of main", "", 5));
        assert!(matches("W/Example: warning", "EXAMPLE", 4));
        assert!(matches("--------- beginning of main", "", 0));
    }
    #[test]
    fn drawer_tabs_fit_minimum_window_without_collecting_logs() {
        for diagnostics in [false, true] {
            let ctx = egui::Context::default();
            super::super::desktop_ui::configure_theme(&ctx);
            let mut app = PhoneMirrorApp::default();
            app.drawer.serial =
                "very-long-device-identifier-with-spaces-and-a-wireless-host-address.local:5555"
                    .into();
            app.drawer.open = true;
            app.drawer.diagnostics = diagnostics;
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
            assert!(!app.drawer.reader.busy());
            for shape in output.shapes {
                let rect = shape.shape.visual_bounding_rect();
                if rect.is_finite() && rect.is_positive() {
                    assert!(rect.right() <= 728.0, "drawer overflow: {rect:?}");
                }
            }
        }
    }
}
