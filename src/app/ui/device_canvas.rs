//! Primary and split mirror canvas, including idle and connection states.
use super::components::{
    device_row::state_label,
    icons::{paint_icon, Icon},
    muted, notice,
};
use super::desktop_ui::Page;
use crate::app::PhoneMirrorApp;
use crate::app::{devices::State, tools, ACCENT, BORDER, CARD_BG, TEXT_DIM};
use egui::vec2;

impl PhoneMirrorApp {
    pub(super) fn device_canvas(&mut self, ui: &mut egui::Ui) {
        if self.split.count() == 0 {
            self.single_device_canvas(ui);
            return;
        }
        let size = ui.available_size();
        let gap = 8.0;
        ui.spacing_mut().item_spacing.x = gap;
        let pane_width = ((size.x - gap) / 2.0).max(1.0);
        let ready = self
            .device_status
            .tools
            .as_ref()
            .is_some_and(|report| !report.missing())
            && self.device_status.scrcpy_health.is_empty();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                vec2(pane_width, size.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.push_id("primary_mirror_pane", |ui| self.single_device_canvas(ui));
                },
            );
            ui.allocate_ui_with_layout(
                vec2(pane_width, size.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    self.split.show(
                        ui,
                        &self.inventory,
                        &self.device_status.device_serial,
                        ready,
                    );
                },
            );
        });
    }

    fn single_device_canvas(&mut self, ui: &mut egui::Ui) {
        if self.embedded.has_session() {
            self.embedded.show(ui);
            return;
        }
        if let Some(report) = self
            .device_status
            .tools
            .clone()
            .filter(tools::Report::missing)
        {
            self.setup_checklist(ui, &report);
            return;
        }
        let device = self
            .inventory
            .devices
            .iter()
            .find(|d| Some(&d.id) == self.selected_device.as_ref())
            .cloned();
        let available = ui.available_size();
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.add_space((available.y * 0.22).max(0.0));
                let (r, _) = ui.allocate_exact_size(egui::vec2(62.0, 86.0), egui::Sense::hover());
                ui.painter().add(
                    egui::Shadow {
                        offset: [0, 10],
                        blur: 28,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(90),
                    }
                    .as_shape(r, 14),
                );
                ui.painter().rect_filled(r, 14.0, CARD_BG);
                ui.painter().rect_stroke(
                    r,
                    14.0,
                    egui::Stroke::new(1.0, BORDER),
                    egui::StrokeKind::Inside,
                );
                paint_icon(ui.painter(), r, Icon::Phone, TEXT_DIM);
                ui.add_space(16.0);
                if let Some(device) = device {
                    ui.add(
                        egui::Label::new(egui::RichText::new(&device.name).size(18.0).strong())
                            .wrap(),
                    );
                    let (label, color) = state_label(device.state);
                    ui.colored_label(color, egui::RichText::new(label).size(12.0));
                    match device.state {
                        State::Online => {
                            ui.add_space(8.0);
                            if ui
                                .add_enabled(
                                    self.device_status.scrcpy_health.is_empty(),
                                    egui::Button::new(
                                        egui::RichText::new("Start mirroring here")
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(ACCENT)
                                    .stroke(egui::Stroke::NONE)
                                    .corner_radius(10)
                                    .min_size(egui::vec2(ui.available_width().min(200.0), 36.0)),
                                )
                                .clicked()
                            {
                                self.start_mirror(ui.ctx());
                            }
                            muted(ui, "Or use the controls in the toolbar above.");
                        }
                        State::Stopped => {
                            let starting = self.starting_emulator.is_some();
                            if ui
                                .add_enabled(
                                    !starting && self.inventory.emulator_path.is_some(),
                                    egui::Button::new(if starting {
                                        "Starting emulator…"
                                    } else {
                                        "Start emulator"
                                    }),
                                )
                                .clicked()
                            {
                                if let Some(avd) = device.avd {
                                    self.start_emulator(device.id, avd);
                                }
                            }
                            if starting {
                                ui.spinner();
                            }
                            muted(ui, "Mirroring is available after the emulator connects.");
                        }
                        State::Unauthorized => muted(
                            ui,
                            "Unlock the phone and approve USB debugging, then refresh.",
                        ),
                        State::Offline => {
                            muted(ui, "Wait for boot or reconnect this device, then refresh.")
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new(if self.selected_device.is_some() {
                            "Device disconnected"
                        } else {
                            "Choose a device"
                        })
                        .size(20.0)
                        .strong(),
                    );
                    muted(ui, "Select an emulator or connect an Android phone.");
                    if ui.button("Pair over Wi-Fi").clicked() {
                        self.page = Page::Wifi;
                    }
                }
                if !self.device_status.adb_health.is_empty() {
                    notice(ui, "Device discovery", &self.device_status.adb_health, true);
                }
                if !self.device_status.scrcpy_health.is_empty() {
                    notice(
                        ui,
                        "Mirroring unavailable",
                        &self.device_status.scrcpy_health,
                        true,
                    );
                }
            },
        );
    }
}
