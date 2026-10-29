//! Independent additional mirror panes. Hidden slots retain their asynchronous cleanup queues.
use super::mirror_ui::MirrorUi;
use crate::app::{adb_path, build_full_env, devices, mirror, scrcpy_path};

#[derive(Default)]
struct Pane {
    serial: String,
    mirror: MirrorUi,
}

#[derive(Default)]
pub(crate) struct SplitUi {
    count: usize,
    panes: [Pane; 3],
}

impl SplitUi {
    /// Number of additional panes, excluding the existing primary screen.
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn set_count(&mut self, count: usize) {
        self.count = count.min(3);
        for pane in &mut self.panes[self.count..] {
            pane.mirror.stop();
            // A visible pane may claim this serial while the slot is hidden.
            pane.serial.clear();
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context, visible: bool) {
        for (index, pane) in self.panes.iter_mut().enumerate() {
            pane.mirror.poll(ctx);
            if !visible || index >= self.count {
                pane.mirror.release_touch();
            }
        }
    }

    pub fn active(&self) -> bool {
        self.panes.iter().any(|pane| pane.mirror.active())
    }

    /// Reservations only apply to live/starting sessions, not stopped pane selections.
    pub fn contains_serial(&self, serial: &str) -> bool {
        !serial.is_empty()
            && self
                .panes
                .iter()
                .any(|pane| pane.serial == serial && pane.mirror.active())
    }

    /// Primary selection always wins; stop a conflicting extra without changing primary selection.
    pub fn reconcile_primary(&mut self, serial: &str) {
        if serial.is_empty() {
            return;
        }
        for pane in &mut self.panes {
            if pane.serial == serial {
                pane.mirror.stop();
                pane.serial.clear();
            }
        }
    }

    pub fn diagnostics(&self) -> String {
        self.panes[..self.count].iter().enumerate().map(|(index, pane)| format!("Pane {}: {}", index + 2, pane.mirror.diagnostic())).collect::<Vec<_>>().join("\n")
    }

    pub fn stop(&mut self) {
        for pane in &mut self.panes {
            pane.mirror.stop();
        }
    }

    pub fn shutdown_on_exit(&mut self) {
        // Signal all workers before waiting for any individual cleanup.
        self.stop();
        for pane in &mut self.panes {
            pane.mirror.shutdown_on_exit();
        }
    }

    fn choices<'a>(
        &self,
        inventory: &'a devices::Inventory,
        primary: &str,
        index: usize,
    ) -> Vec<&'a devices::Device> {
        let mut seen = std::collections::HashSet::new();
        inventory
            .devices
            .iter()
            .filter(|device| {
                let Some(serial) = device.serial.as_deref().filter(|serial| !serial.is_empty())
                else {
                    return false;
                };
                device.state == devices::State::Online
                    && serial != primary
                    && !self.panes[..self.count]
                        .iter()
                        .enumerate()
                        .any(|(other, pane)| other != index && pane.serial == serial)
                    && seen.insert(serial)
            })
            .collect()
    }

    /// Render into the caller's bounded area; each pane owns its input/focus/scroll IDs.
    /// ready describes shared tool readiness, not the primary device's connection state.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        inventory: &devices::Inventory,
        primary_serial: &str,
        ready: bool,
    ) {
        self.reconcile_primary(primary_serial);
        if self.count == 0 {
            return;
        }
        let area = ui.available_rect_before_wrap();
        let gap = ui
            .spacing()
            .item_spacing
            .y
            .min(area.height().max(0.0) / self.count as f32);
        let height = ((area.height() - gap * (self.count - 1) as f32) / self.count as f32).max(0.0);
        for index in 0..self.count {
            let choices: Vec<(String, String)> = self
                .choices(inventory, primary_serial, index)
                .into_iter()
                .map(|device| {
                    (
                        device.serial.clone().unwrap(),
                        format!("{} · {}", device.name, device.serial.as_deref().unwrap()),
                    )
                })
                .collect();
            let rect = egui::Rect::from_min_size(
                area.min + egui::vec2(0.0, index as f32 * (height + gap)),
                egui::vec2(area.width().max(0.0), height),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("split-pane", index))
                    .max_rect(rect),
            );
            {
                let ui = &mut child;
                ui.set_clip_rect(ui.clip_rect().intersect(rect));
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                let pane = &mut self.panes[index];
                let online = choices.iter().any(|(serial, _)| serial == &pane.serial);
                if !online && pane.mirror.active() {
                    pane.mirror.stop();
                }
                // A short scrollable control band stays bounded even in very small panes.
                egui::ScrollArea::vertical()
                    .id_salt("controls")
                    .max_height(height.min(76.0))
                    .show(ui, |ui| {
                        let previous = pane.serial.clone();
                        let label = choices
                            .iter()
                            .find(|(serial, _)| serial == &pane.serial)
                            .map(|(_, label)| label.as_str())
                            .unwrap_or(if pane.serial.is_empty() {
                                "Choose device"
                            } else {
                                "Device unavailable"
                            });
                        egui::ComboBox::from_id_salt("device")
                            .width((ui.available_width() - 24.0).max(1.0))
                            .selected_text(label)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut pane.serial,
                                    String::new(),
                                    "Choose device",
                                );
                                for (serial, label) in &choices {
                                    ui.selectable_value(&mut pane.serial, serial.clone(), label);
                                }
                            });
                        if pane.serial != previous {
                            pane.mirror.stop();
                        }
                        ui.horizontal_wrapped(|ui| {
                            if pane.mirror.active() {
                                if ui.button("Stop").clicked() {
                                    pane.mirror.stop();
                                }
                            } else if ui
                                .add_enabled(
                                    ready
                                        && choices.iter().any(|(serial, _)| serial == &pane.serial),
                                    egui::Button::new("Start"),
                                )
                                .clicked()
                            {
                                pane.mirror.start(
                                    mirror::Config {
                                        serial: pane.serial.clone(),
                                        adb_path: adb_path(),
                                        scrcpy_path: scrcpy_path(),
                                        env: build_full_env(),
                                    },
                                    ui.ctx().clone(),
                                );
                            }
                            if ui.button("Fit").clicked() {
                                pane.mirror.fit();
                            }
                            ui.menu_button("More", |ui| {
                                let mut keyboard = pane.mirror.keyboard_enabled();
                                if ui.checkbox(&mut keyboard, "Forward keyboard").changed() {
                                    pane.mirror.toggle_keyboard();
                                }
                                ui.add_enabled_ui(pane.mirror.streaming(), |ui| {
                                    for (label, code) in
                                        [("Back", 4), ("Home", 3), ("Recent apps", 187)]
                                    {
                                        if ui.button(label).clicked() {
                                            pane.mirror.navigation_key(code);
                                            ui.close_menu();
                                        }
                                    }
                                    if ui.button("Zoom in").clicked() {
                                        pane.mirror.zoom_by(1.25);
                                    }
                                    if ui.button("Zoom out").clicked() {
                                        pane.mirror.zoom_by(0.8);
                                    }
                                    ui.label(pane.mirror.zoom_label());
                                });
                                if ui
                                    .add_enabled(
                                        pane.mirror.can_wake(),
                                        egui::Button::new("Wake screen"),
                                    )
                                    .clicked()
                                {
                                    pane.mirror.wake_screen();
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                if pane.mirror.has_session() {
                    pane.mirror.show(ui);
                } else {
                    ui.label(
                        egui::RichText::new("Choose an online device, then Start.")
                            .color(crate::app::TEXT_DIM),
                    );
                }
            }
        }
        ui.allocate_rect(area, egui::Sense::hover());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(serial: &str) -> Pane {
        Pane {
            serial: serial.into(),
            mirror: MirrorUi::fixture(
                serial,
                mirror::Status::Streaming,
                Some(mirror::Frame {
                    width: 2,
                    height: 4,
                    rgb: vec![128; 24],
                }),
            ),
        }
    }

    fn inventory() -> devices::Inventory {
        devices::Inventory {
            devices: ["primary", "one", "two", "three"]
                .into_iter()
                .map(|serial| devices::Device {
                    id: serial.into(),
                    name: serial.into(),
                    serial: Some(serial.into()),
                    avd: None,
                    kind: devices::Kind::Usb,
                    state: devices::State::Online,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn independent_lifecycle_and_hidden_cleanup() {
        let ctx = egui::Context::default();
        let mut split = SplitUi::default();
        split.set_count(99);
        assert_eq!(split.count(), 3);
        split.panes = [fixture("one"), fixture("two"), fixture("three")];
        split.poll(&ctx, true);
        assert!(split.panes.iter().all(|pane| pane.mirror.streaming()));
        split.panes[0].mirror.stop();
        assert!(split.panes[1].mirror.streaming());
        assert!(split.panes[2].mirror.streaming());
        split.set_count(1);
        assert!(!split.active());
        assert!(!split.panes[1].mirror.has_session());
        assert!(split.panes[1].serial.is_empty());
        assert!(split.panes[2].serial.is_empty());
        split.poll(&ctx, false);
        split.set_count(3);
        assert!(!split.active());
        split.panes[0] = fixture("one");
        split.stop();
        assert!(!split.active());
        split.shutdown_on_exit();
        split.set_count(0);
        assert_eq!(split.count(), 0);
    }

    #[test]
    fn duplicate_exclusion_and_primary_reconciliation() {
        let mut split = SplitUi::default();
        split.set_count(3);
        split.panes[0] = fixture("one");
        split.panes[1] = fixture("two");
        let mut inventory = inventory();
        inventory.devices.push(inventory.devices[3].clone());
        inventory.devices[3].state = devices::State::Offline;
        let choices = split.choices(&inventory, "primary", 2);
        assert_eq!(
            choices
                .iter()
                .map(|device| device.serial.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec!["three"]
        );
        assert!(split.contains_serial("one"));
        assert!(!split.contains_serial(""));
        split.reconcile_primary("one");
        assert!(!split.contains_serial("one"));
        assert!(split.contains_serial("two"));
        assert!(split.panes[0].serial.is_empty());
    }

    #[test]
    fn responsive_fixture_rendering_and_error_surfaces() {
        let ctx = egui::Context::default();
        let mut split = SplitUi::default();
        split.set_count(3);
        split.panes = [
            fixture("one"),
            fixture("two"),
            Pane {
                serial: "three".into(),
                mirror: MirrorUi::fixture(
                    "three",
                    mirror::Status::Failed("fixture error".into()),
                    None,
                ),
            },
        ];
        split.poll(&ctx, true);
        for size in [
            egui::vec2(1200.0, 800.0),
            egui::vec2(320.0, 420.0),
            egui::vec2(100.0, 100.0),
        ] {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        split.show(ui, &inventory(), "primary", false);
                        assert!(ui.min_rect().width() <= size.x);
                        assert!(ui.min_rect().height() <= size.y);
                    });
                },
            );
            assert!(!output.shapes.is_empty());
        }
        assert!(split.panes[0].mirror.streaming());
        assert!(split.panes[1].mirror.streaming());
        split.shutdown_on_exit();
    }
}
