//! Pairing workspace layout around the existing wireless form.
use super::components::{
    heading,
    icons::{paint_icon, Icon},
    muted, surface,
};
use super::desktop_ui::Page;
use crate::app::PhoneMirrorApp;
use crate::app::ACCENT;

impl PhoneMirrorApp {
    pub(super) fn wifi_page(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Connect without a cable",
            "Pair your Android phone once. Reconnect whenever you need it.",
        );
        ui.add_space(12.0);
        if ui.available_width() >= 820.0 {
            let form_width = ui.available_width() - 284.0;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(form_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| surface(ui, |ui| self.wireless.show(ui)),
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(272.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.pairing_help(ui),
                );
            });
        } else {
            surface(ui, |ui| self.wireless.show(ui));
            ui.add_space(16.0);
            self.pairing_help(ui);
        }
    }
    fn pairing_help(&mut self, ui: &mut egui::Ui) {
        surface(ui, |ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
            paint_icon(ui.painter(), r, Icon::Wifi, ACCENT);
            ui.label(
                egui::RichText::new("Before you connect")
                    .size(17.0)
                    .strong(),
            );
            muted(ui, "Use Android 11 or later and keep your phone and computer on the same private Wi-Fi network.");
            ui.separator();
            ui.label(egui::RichText::new("Two ports, two purposes").strong());
            muted(ui, "The pairing port authorizes this computer. The connection port opens the session. Copy each from the correct screen on your phone.");
            ui.separator();
            ui.label(egui::RichText::new("Your device stays yours").strong());
            muted(ui, "Only pair devices you trust. To revoke access, forget this computer in your phone's Wireless debugging settings.");
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [ui.available_width(), 38.0],
                    egui::Button::new("Back to devices"),
                )
                .clicked()
            {
                self.page = Page::Devices;
            }
        });
    }
}
