//! Reusable egui widgets and small presentation helpers.
pub(in crate::app::ui) mod capsule;
pub(in crate::app::ui) mod device_row;
pub(in crate::app::ui) mod icons;

use super::theme::medium;
use crate::app::{ACCENT, ACCENT_TEXT, BORDER, CARD_BG, ERROR_FILL, INFO_FILL, RED, TEXT_DIM};
use egui::{vec2, Color32, Stroke};

/// Smoothly fades a hover/selection factor; egui keeps repainting while it animates.
pub(in crate::app::ui) fn ease(ui: &egui::Ui, id: egui::Id, on: bool, seconds: f32) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, seconds)
}

pub(in crate::app) fn muted(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(TEXT_DIM)).wrap());
}
pub(in crate::app) fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add(egui::Label::new(egui::RichText::new(title).size(26.0).strong()).wrap());
    muted(ui, subtitle);
    ui.add_space(6.0);
}
pub(in crate::app) fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 44.0],
        egui::Button::new(egui::RichText::new(text).strong().color(Color32::WHITE))
            .fill(ACCENT)
            .stroke(Stroke::NONE)
            .corner_radius(10),
    )
}
pub(in crate::app) fn surface(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(14)
        .inner_margin(22)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}
pub(in crate::app) fn notice(ui: &mut egui::Ui, title: &str, detail: &str, error: bool) {
    let (fill, color) = if error {
        (ERROR_FILL, RED)
    } else {
        (INFO_FILL, ACCENT_TEXT)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.22)))
        .corner_radius(10)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(egui::RichText::new(title).strong().color(color)).wrap());
            ui.add(egui::Label::new(detail).wrap());
        });
}

/// Moves the window when its empty chrome is dragged; double-click zooms like macOS.
pub(in crate::app::ui) fn window_drag(ui: &egui::Ui, response: &egui::Response) {
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    } else if response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

pub(in crate::app::ui) fn section_label(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(title)
                .font(medium(11.0))
                .color(TEXT_DIM),
        );
    });
    ui.add_space(2.0);
}

pub(in crate::app::ui) fn detail(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).small().color(TEXT_DIM));
    ui.horizontal_top(|ui| {
        let width = (ui.available_width() - 58.0).max(40.0);
        ui.allocate_ui_with_layout(
            vec2(width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.add(egui::Label::new(value).wrap());
            },
        );
        if ui
            .small_button("Copy")
            .on_hover_text(format!("Copy {}", label.to_lowercase()))
            .clicked()
        {
            ui.ctx().copy_text(value.into());
        }
    });
}
