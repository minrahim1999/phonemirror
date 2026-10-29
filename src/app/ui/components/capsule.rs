//! Segmented capsule buttons and their anchored menus.
use super::super::theme::{DIVIDER, PILL, PILL_BORDER, PILL_H, SEG};
use super::{
    ease,
    icons::{paint_icon, Icon},
};
use crate::app::{ACCENT_SOFT, ACCENT_TEXT, SURFACE_HOVER, TEXT, TEXT_DIM};
use egui::{pos2, vec2, Rect, Stroke};

#[derive(Clone, Copy)]
pub(in crate::app::ui) struct Item<'a> {
    icon: Icon,
    label: &'a str,
    enabled: bool,
    on: bool,
}
pub(in crate::app::ui) fn item(icon: Icon, label: &str, enabled: bool, on: bool) -> Item<'_> {
    Item {
        icon,
        label,
        enabled,
        on,
    }
}

pub(in crate::app::ui) fn pill_width(items: usize) -> f32 {
    if items == 1 {
        PILL_H
    } else {
        SEG * items as f32
    }
}

/// A capsule of icon buttons separated by hairlines, like the simulator toolbar.
pub(in crate::app::ui) fn pill(ui: &mut egui::Ui, items: &[Item]) -> Vec<egui::Response> {
    let (rect, _) =
        ui.allocate_exact_size(vec2(pill_width(items.len()), PILL_H), egui::Sense::hover());
    pill_at(ui, rect, items)
}

pub(in crate::app::ui) fn pill_at(
    ui: &mut egui::Ui,
    rect: Rect,
    items: &[Item],
) -> Vec<egui::Response> {
    let segment = rect.width() / items.len().max(1) as f32;
    let painter = ui.painter();
    painter.rect(
        rect,
        rect.height() / 2.0,
        PILL,
        Stroke::new(1.0, PILL_BORDER),
        egui::StrokeKind::Inside,
    );
    for i in 1..items.len() {
        let x = (rect.left() + segment * i as f32).round() + 0.5;
        painter.line_segment(
            [pos2(x, rect.top() + 9.0), pos2(x, rect.bottom() - 9.0)],
            Stroke::new(1.0, DIVIDER),
        );
    }
    items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let slot = Rect::from_min_size(
                pos2(rect.left() + segment * i as f32, rect.top()),
                vec2(segment, rect.height()),
            );
            icon_button(ui, slot, entry)
        })
        .collect()
}

pub(in crate::app::ui) fn icon_button(
    ui: &mut egui::Ui,
    rect: Rect,
    entry: &Item,
) -> egui::Response {
    let id = ui.id().with(("chrome-button", entry.label));
    let sense = if entry.enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let response = ui.interact(rect, id, sense);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            entry.enabled,
            entry.on,
            entry.label,
        )
    });
    let hover = ease(
        ui,
        id.with("hover"),
        entry.enabled && response.hovered(),
        0.12,
    );
    // A disabled control never shows its "on" highlight; it would read as active.
    let on = ease(ui, id.with("on"), entry.enabled && entry.on, 0.16);
    let pressed = if response.is_pointer_button_down_on() {
        0.5
    } else {
        0.0
    };
    let highlight = rect.shrink2(vec2(3.0, 3.0));
    let fill = SURFACE_HOVER
        .gamma_multiply((hover + pressed).min(1.0))
        .lerp_to_gamma(ACCENT_SOFT, on);
    ui.painter()
        .rect_filled(highlight, highlight.height() / 2.0, fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            highlight,
            highlight.height() / 2.0,
            Stroke::new(1.5, ACCENT_TEXT),
            egui::StrokeKind::Inside,
        );
    }
    let color = if !entry.enabled {
        TEXT_DIM.gamma_multiply(0.4)
    } else {
        TEXT.lerp_to_gamma(ACCENT_TEXT, on)
    };
    paint_icon(ui.painter(), rect, entry.icon, color);
    response.on_hover_text(entry.label)
}

/// Opens a menu below a chrome button; any click outside the button closes it.
pub(in crate::app::ui) fn chrome_menu(
    ui: &egui::Ui,
    response: &egui::Response,
    id: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    let popup = egui::Id::new(id);
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    egui::popup_below_widget(
        ui,
        popup,
        response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(210.0);
            ui.spacing_mut().button_padding = vec2(8.0, 4.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), content);
        },
    );
}
pub(in crate::app::ui) fn menu_item(ui: &mut egui::Ui, text: &str, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new(text).frame(false))
        .clicked()
}
