//! Shared fonts, theme and simulator chrome metrics.
use crate::app::{
    ACCENT, ACCENT_TEXT, BG, BORDER, CARD_BG, SELECTED, SIDEBAR, SURFACE_HOVER, TEXT,
};
use egui::{vec2, Color32, Stroke};

// ─── Simulator-style chrome metrics ───────────────────────
pub(in crate::app::ui) const PILL_H: f32 = 32.0;
pub(in crate::app::ui) const SEG: f32 = 36.0;
pub(in crate::app::ui) const GROUP_GAP: f32 = 10.0;
/// Space the macOS window buttons occupy at the top-left of the window.
pub(in crate::app::ui) const TRAFFIC_LIGHTS_WIDTH: f32 = 80.0;
pub(in crate::app::ui) const PILL: Color32 = Color32::from_rgb(65, 65, 67);
pub(in crate::app::ui) const PILL_BORDER: Color32 =
    Color32::from_rgba_premultiplied(18, 18, 18, 18);
pub(in crate::app::ui) const DIVIDER: Color32 = Color32::from_rgb(70, 70, 74);
pub(in crate::app::ui) const SEARCH_FILL: Color32 = Color32::from_rgb(87, 87, 89);
const MEDIUM: &str = "medium";

pub(in crate::app::ui) fn medium(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(MEDIUM.into()))
}

/// Reads a system font once per process; the bytes live for the whole program.
#[cfg(target_os = "macos")]
pub(in crate::app::ui) fn system_font(
    path: &str,
    slot: &'static std::sync::OnceLock<Option<&'static [u8]>>,
) -> Option<&'static [u8]> {
    *slot.get_or_init(|| {
        std::fs::read(path)
            .ok()
            .map(|bytes| &*Box::leak(bytes.into_boxed_slice()))
    })
}

/// San Francisco for body text and Helvetica Neue Medium for titles on macOS.
/// The "medium" family always exists so text never panics when fonts are missing.
pub(in crate::app::ui) fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    #[cfg(target_os = "macos")]
    {
        use std::sync::{Arc, OnceLock};
        static SF: OnceLock<Option<&'static [u8]>> = OnceLock::new();
        static NEUE: OnceLock<Option<&'static [u8]>> = OnceLock::new();
        if let Some(bytes) = system_font("/System/Library/Fonts/SFNS.ttf", &SF) {
            fonts
                .font_data
                .insert("sf".into(), Arc::new(egui::FontData::from_static(bytes)));
            if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                list.insert(0, "sf".into());
            }
        }
        let mut medium = fonts.families[&egui::FontFamily::Proportional].clone();
        if let Some(bytes) = system_font("/System/Library/Fonts/HelveticaNeue.ttc", &NEUE) {
            let mut data = egui::FontData::from_static(bytes);
            data.index = 10; // Helvetica Neue Medium
            fonts.font_data.insert("neue-medium".into(), Arc::new(data));
            medium.insert(0, "neue-medium".into());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name(MEDIUM.into()), medium);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let medium = fonts.families[&egui::FontFamily::Proportional].clone();
        fonts
            .families
            .insert(egui::FontFamily::Name(MEDIUM.into()), medium);
    }
    ctx.set_fonts(fonts);
}

pub fn configure_theme(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.panel_fill = BG;
    v.window_fill = CARD_BG;
    v.extreme_bg_color = Color32::from_rgb(24, 24, 26);
    v.faint_bg_color = SIDEBAR;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT_TEXT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, ACCENT_TEXT);
    v.text_cursor.stroke = Stroke::new(2.0, ACCENT_TEXT);
    v.window_corner_radius = egui::CornerRadius::same(14);
    v.menu_corner_radius = egui::CornerRadius::same(10);
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_shadow = egui::Shadow {
        offset: [0, 12],
        blur: 36,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    v.popup_shadow = egui::Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.inactive.bg_fill = CARD_BG;
    w.inactive.weak_bg_fill = CARD_BG;
    w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(78, 78, 84));
    w.active.bg_fill = SELECTED;
    w.active.weak_bg_fill = SELECTED;
    w.open.weak_bg_fill = SURFACE_HOVER;
    for state in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.corner_radius = egui::CornerRadius::same(8);
        // No size jump on hover/press: state changes read as smooth color shifts.
        state.expansion = 0.0;
    }
    style.animation_time = 0.14;
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = vec2(14.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    for (role, size) in [
        (egui::TextStyle::Body, 13.0),
        (egui::TextStyle::Button, 13.0),
        (egui::TextStyle::Small, 11.0),
        (egui::TextStyle::Heading, 24.0),
    ] {
        style
            .text_styles
            .insert(role, egui::FontId::proportional(size));
    }
    ctx.set_style(style);
}
