//! Workspace composition only; widgets and screen sections live in sibling modules.
use super::components::window_drag;
use crate::app::{devices::Device, PhoneMirrorApp, BG, RED, SIDEBAR, TEXT_DIM};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Devices,
    Wifi,
}

impl PhoneMirrorApp {
    pub(crate) fn show_workspace(&mut self, ctx: &egui::Context) {
        if self.sidebar_open {
            egui::SidePanel::left("device_library")
                .resizable(false)
                .exact_width(if ctx.screen_rect().width() < 900.0 {
                    220.0
                } else {
                    236.0
                })
                .frame(egui::Frame::new().fill(SIDEBAR).inner_margin(egui::Margin {
                    left: 6,
                    right: 8,
                    top: 10,
                    bottom: 8,
                }))
                .show(ctx, |ui| self.device_library(ui));
        }
        egui::TopBottomPanel::top("hub_toolbar")
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin {
                left: 16,
                right: 12,
                top: 10,
                bottom: 10,
            }))
            .show(ctx, |ui| {
                // Registered first so every control laid out afterwards wins its own clicks.
                let drag = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("toolbar_drag"),
                    egui::Sense::click_and_drag(),
                );
                window_drag(ui, &drag);
                if self.page == Page::Devices {
                    self.device_toolbar(ui);
                } else {
                    self.wifi_toolbar(ui);
                }
            });
        if !self.status_message.is_empty() || self.is_recording {
            egui::TopBottomPanel::bottom("canvas_status")
                .show_separator_line(false)
                .frame(
                    egui::Frame::new()
                        .fill(BG)
                        .inner_margin(egui::Margin::symmetric(16, 7)),
                )
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if self.is_recording {
                            ui.colored_label(RED, egui::RichText::new("Recording").small());
                        }
                        if !self.status_message.is_empty() {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&self.status_message)
                                        .size(12.0)
                                        .color(if self.status_is_error { RED } else { TEXT_DIM }),
                                )
                                .wrap(),
                            );
                            if ui.small_button("Dismiss").clicked() {
                                self.status_message.clear();
                            }
                        }
                    });
                });
        }
        self.show_tools_drawer(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(12))
            .show(ctx, |ui| {
                if self.page == Page::Wifi {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width().min(1040.0));
                            self.wifi_page(ui);
                        });
                } else {
                    ui.add_enabled_ui(!self.show_close_warning, |ui| self.device_canvas(ui));
                }
            });
        self.show_device_info_window(ctx);
    }

    pub(super) fn selected(&self) -> Option<&Device> {
        self.inventory
            .devices
            .iter()
            .find(|d| Some(&d.id) == self.selected_device.as_ref())
    }
}

#[cfg(test)]
#[path = "desktop_ui_tests.rs"]
mod tests;
