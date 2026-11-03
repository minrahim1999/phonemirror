//! Device information window and explicit-target connection actions.
use super::components::{detail, device_row::kind_label, muted, notice};
use crate::app::devices::Kind;
use crate::app::PhoneMirrorApp;

impl PhoneMirrorApp {
    pub(super) fn show_device_info_window(&mut self, ctx: &egui::Context) {
        if self.show_device_info {
            let mut open = true;
            egui::Window::new("Device information")
            .id(egui::Id::new("device-info"))
            .open(&mut open)
            .default_width(340.0)
            .max_width((ctx.screen_rect().width() - 48.0).max(200.0))
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                if let Some(device) = self.selected().cloned() {
                    detail(ui, "Name", &device.name);
                    detail(ui, "Connection", kind_label(device.kind));
                    if let Some(serial) = &device.serial {
                        detail(ui, "Serial", serial);
                        if device.kind == Kind::Wifi {
                            muted(ui, "Disconnect only this connection. Pairing and other connections stay intact; ADB may rediscover it.");
                            let target = self.disconnect_target(&device);
                            if ui.add_enabled(target.is_ok(), egui::Button::new("Disconnect this wireless connection")).clicked() {
                                if let Ok(serial) = target.clone() {
                                    self.disconnect_device(ui.ctx(), serial);
                                }
                            }
                            if let Err(reason) = target {
                                muted(ui, reason);
                            }
                        }
                    }
                    if device.kind != Kind::Emulator {
                        muted(ui, "To make a clean new connection, forget this device and pair it again.");
                        if ui.button("Forget this device…").clicked() {
                            self.request_forget(device.clone());
                        }
                    }
                    if let Some(avd) = &device.avd {
                        detail(ui, "Virtual device", avd);
                    }
                }
                ui.separator();
                self.tools_section(ui);
                ui.separator();
                if !self.device_status.scrcpy_health.is_empty() {
                    notice(ui, "scrcpy", &self.device_status.scrcpy_health, true);
                }
                if !self.device_status.adb_health.is_empty() {
                    notice(ui, "Device discovery", &self.device_status.adb_health, true);
                }
                if !self.inventory.emulator_health.is_empty() {
                    muted(ui, &self.inventory.emulator_health);
                }
                ui.separator();
                muted(
                    ui,
                    "Embedded screen: scrcpy 4.1 + FFmpeg. No audio playback.",
                );
                if (self.embedded.active() || self.is_recording)
                    && ui.button("Stop PhoneMirror sessions").clicked()
                {
                    self.close_mirror();
                }
                ui.hyperlink_to(
                    "Help & source",
                    "https://github.com/minrahim1999/phonemirror",
                );
            });
            self.show_device_info = open;
        }
    }
}
