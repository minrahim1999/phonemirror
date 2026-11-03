//! Explicit-target connection actions and the reasons controls are unavailable.
//!
//! ADB has no per-device "unpair": Android keeps the pairing on the phone. "Forget" is
//! therefore a guided flow (disconnect here, forget on the phone, pair again). It never
//! deletes ADB keys, disconnects other transports or changes network settings.
use super::components::{capsule::menu_item_why, muted};
use super::desktop_ui::Page;
use crate::app::devices::{Device, Kind, State};
use crate::app::{PhoneMirrorApp, BORDER, CARD_BG, GREEN};
use egui::{RichText, Stroke};

impl PhoneMirrorApp {
    /// Why the selected device cannot be used right now, ignoring scrcpy health.
    pub(super) fn device_unavailable_reason(&self) -> Option<&'static str> {
        if self.device_status.connected {
            return None;
        }
        Some(match self.selected().map(|d| d.state) {
            None => "Select a device in the sidebar first.",
            Some(State::Stopped) => "Start this emulator first.",
            Some(State::Offline) => "This device is offline. Reconnect it, then refresh.",
            Some(State::Unauthorized) => "Approve USB debugging on the phone, then refresh.",
            Some(State::Online) => "Waiting for this device's ADB connection.",
        })
    }

    /// Why mirroring or recording cannot start for the selected device.
    pub(super) fn mirror_unavailable_reason(&self) -> Option<&'static str> {
        self.device_unavailable_reason().or_else(|| {
            (!self.device_status.scrcpy_health.is_empty())
                .then_some("Mirroring needs attention. Open Device information for details.")
        })
    }

    /// Why controls that act on the live screen (zoom, keys, keyboard) are unavailable.
    pub(super) fn stream_unavailable_reason(&self) -> Option<&'static str> {
        if self.embedded.streaming() {
            None
        } else if self.embedded.active() {
            Some("Waiting for the first video frame.")
        } else {
            Some(
                self.mirror_unavailable_reason()
                    .unwrap_or("Start mirroring to use this control."),
            )
        }
    }

    /// Recording has no recorded target, so it conservatively blocks every disconnect.
    fn session_uses(&self, serial: &str) -> bool {
        self.is_recording
            || (self.embedded.active() && self.device_status.device_serial == serial)
            || self.split.contains_serial(serial)
    }

    /// The exact serial to disconnect, or why disconnecting this device is unavailable.
    pub(super) fn disconnect_target(&self, device: &Device) -> Result<String, &'static str> {
        if device.kind != Kind::Wifi {
            return Err("Only Wi-Fi connections can be disconnected. Unplug USB devices instead.");
        }
        let Some(serial) = device.serial.clone() else {
            return Err("This device is not connected to this computer.");
        };
        if self.wireless.busy() {
            return Err("Another Wi-Fi operation is still running.");
        }
        if self.session_uses(&serial) {
            return Err("Stop mirroring and recording for this device first.");
        }
        Ok(serial)
    }

    pub(super) fn disconnect_device(&mut self, ctx: &egui::Context, serial: String) {
        self.embedded.release_touch();
        self.wireless.disconnect(ctx, serial);
        // The pairing page reports the result and how to reconnect or pair again.
        self.page = Page::Wifi;
    }

    pub(super) fn request_forget(&mut self, device: Device) {
        self.embedded.release_touch();
        self.forget_device = Some(device);
    }

    /// Connection entries shared by the toolbar More menu and the sidebar context menu.
    pub(super) fn connection_menu_items(&mut self, ui: &mut egui::Ui, device: &Device) {
        if device.kind == Kind::Wifi {
            let target = self.disconnect_target(device);
            if menu_item_why(
                ui,
                "Disconnect Wi-Fi connection",
                target.is_ok(),
                target.as_ref().err().copied(),
            ) {
                if let Ok(serial) = target {
                    self.disconnect_device(ui.ctx(), serial);
                }
                ui.close_menu();
            }
        }
        if device.kind != Kind::Emulator && menu_item_why(ui, "Forget device…", true, None) {
            self.request_forget(device.clone());
            ui.close_menu();
        }
    }

    /// Right-click menu for one sidebar row. Acts on that row only and never changes selection.
    pub(super) fn device_context_menu(&mut self, ui: &mut egui::Ui, device: &Device) {
        ui.set_min_width(210.0);
        ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        if let Some(serial) = &device.serial {
            if menu_item_why(ui, "Copy serial", true, None) {
                ui.ctx().copy_text(serial.clone());
                ui.close_menu();
            }
        }
        if device.kind != Kind::Emulator {
            if device.serial.is_some() {
                ui.separator();
            }
            self.connection_menu_items(ui, device);
        }
    }

    /// Guided "forget" flow for the device captured when the dialog opened.
    pub(super) fn show_forget_dialog(&mut self, ctx: &egui::Context) {
        let Some(target) = self.forget_device.clone() else {
            return;
        };
        // Live state: the device may disconnect or disappear while the dialog is open.
        let live = self
            .inventory
            .devices
            .iter()
            .find(|d| d.id == target.id && d.serial.is_some())
            .cloned();
        let mut close = false;
        let width = (ctx.screen_rect().width() - 64.0).clamp(240.0, 400.0);
        let modal = egui::Modal::new(egui::Id::new("forget-device"))
            .frame(
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(14)
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("Forget {}?", target.name))
                            .size(20.0)
                            .strong(),
                    )
                    .wrap(),
                );
                muted(
                    ui,
                    "Android keeps the pairing on the phone, so PhoneMirror can't remove it alone. Follow these steps for a clean new connection.",
                );
                ui.add_space(6.0);
                if target.kind == Kind::Wifi {
                    step(ui, "1", "Disconnect from this computer");
                    match &live {
                        Some(device) => {
                            let action = self.disconnect_target(device);
                            if ui
                                .add_enabled(action.is_ok(), egui::Button::new("Disconnect now"))
                                .clicked()
                            {
                                if let Ok(serial) = action.clone() {
                                    self.disconnect_device(ctx, serial);
                                }
                            }
                            if let Err(reason) = action {
                                muted(ui, reason);
                            }
                        }
                        None => {
                            ui.label(
                                RichText::new("Not connected to this computer.").color(GREEN),
                            );
                        }
                    }
                    step(ui, "2", "Forget this computer on the phone");
                    muted(ui, "Settings > Developer options > Wireless debugging > Paired devices. Tap this computer, then Forget. Until then, ADB may reconnect automatically.");
                    step(ui, "3", "Pair again");
                    if ui.button("Open Wi-Fi pairing").clicked() {
                        self.embedded.release_touch();
                        self.page = Page::Wifi;
                        close = true;
                    }
                } else {
                    step(ui, "1", "Unplug the USB cable");
                    muted(ui, "PhoneMirror can't disconnect USB devices.");
                    step(ui, "2", "Revoke debugging access on the phone");
                    muted(ui, "Settings > Developer options > Revoke USB debugging authorisations. This removes every computer, not only this one.");
                    step(ui, "3", "Reconnect and allow");
                    muted(ui, "Plug the phone back in, unlock it and allow USB debugging for this computer.");
                }
                ui.add_space(8.0);
                muted(
                    ui,
                    "PhoneMirror never deletes ADB keys or changes network settings.",
                );
                if ui
                    .add_sized([ui.available_width(), 36.0], egui::Button::new("Done"))
                    .clicked()
                {
                    close = true;
                }
            });
        if close || modal.should_close() {
            self.forget_device = None;
        }
    }
}

fn step(ui: &mut egui::Ui, number: &str, title: &str) {
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new(format!("{number}  {title}")).strong()).wrap());
}
