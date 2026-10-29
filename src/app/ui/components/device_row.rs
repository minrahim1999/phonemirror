//! Device metadata labels and selectable sidebar rows.
use super::super::theme::medium;
use super::{
    ease,
    icons::{paint_icon, Icon},
};
use crate::app::{
    devices::{Device, Kind, State},
    ACCENT, ACCENT_TEXT, GREEN, SURFACE_HOVER, TEXT, TEXT_DIM, YELLOW,
};
use egui::{pos2, vec2, Color32, Rect, Stroke};

pub(in crate::app::ui) fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Emulator => "Android emulator",
        Kind::Usb => "USB device",
        Kind::Wifi => "Wi-Fi device",
    }
}
pub(in crate::app::ui) fn kind_short(kind: Kind) -> &'static str {
    match kind {
        Kind::Emulator => "Emulator",
        Kind::Usb => "USB",
        Kind::Wifi => "Wi-Fi",
    }
}
pub(in crate::app::ui) fn state_label(state: State) -> (&'static str, Color32) {
    match state {
        State::Online => ("Connected", GREEN),
        State::Stopped => ("Shut down", TEXT_DIM),
        State::Offline => ("Offline", YELLOW),
        State::Unauthorized => ("Needs authorization", YELLOW),
    }
}

pub(in crate::app::ui) fn device_row(
    ui: &mut egui::Ui,
    device: &Device,
    selected: bool,
    starting: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 46.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            format!("{}, {}", device.name, state_label(device.state).0),
        )
    });
    if ui.is_rect_visible(rect) {
        let hover = ease(ui, response.id.with("hover"), response.hovered(), 0.12);
        let sel = ease(ui, response.id.with("selected"), selected, 0.18);
        let p = ui.painter();
        let fill = SURFACE_HOVER
            .gamma_multiply(hover * 0.7)
            .lerp_to_gamma(ACCENT, sel);
        p.rect_filled(rect, 8.0, fill);
        if response.has_focus() {
            p.rect_stroke(
                rect,
                8.0,
                Stroke::new(1.5, ACCENT_TEXT.lerp_to_gamma(Color32::WHITE, sel)),
                egui::StrokeKind::Inside,
            );
        }
        let on_accent = |base: Color32| base.lerp_to_gamma(Color32::WHITE, sel);
        let icon_center = pos2(rect.left() + 22.0, rect.center().y);
        p.circle_filled(
            icon_center,
            15.0,
            Color32::from_rgb(58, 58, 62).lerp_to_gamma(Color32::from_rgb(92, 140, 238), sel),
        );
        paint_icon(
            p,
            Rect::from_center_size(icon_center, vec2(30.0, 30.0)),
            Icon::Phone,
            TEXT_DIM.lerp_to_gamma(Color32::from_rgb(222, 234, 255), sel),
        );
        let x = rect.left() + 44.0;
        let mut right = rect.right() - 10.0;
        {
            // The reference shows OS versions here. Until inventory exposes those,
            // show truthful transport metadata rather than a fabricated version.
            let galley = p.layout_no_wrap(
                kind_short(device.kind).to_owned(),
                egui::FontId::proportional(12.0),
                on_accent(TEXT_DIM),
            );
            let pos = pos2(
                right - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            right = pos.x - 8.0;
            p.galley(pos, galley, TEXT_DIM);
        }
        let text_painter = p.with_clip_rect(Rect::from_min_max(
            pos2(x, rect.top()),
            pos2(right, rect.bottom()),
        ));
        let mut job = egui::text::LayoutJob::simple(
            device.name.clone(),
            medium(13.0),
            on_accent(TEXT),
            (right - x).max(20.0),
        );
        job.wrap.max_rows = 1;
        let galley = text_painter.layout_job(job);
        text_painter.galley(pos2(x, rect.top() + 7.0), galley, TEXT);
        let (subtitle, color) = if starting {
            ("Starting…", TEXT_DIM)
        } else {
            match device.state {
                State::Online => ("Connected", TEXT_DIM),
                State::Stopped => ("Shut down", TEXT_DIM),
                State::Offline => ("Offline", YELLOW),
                State::Unauthorized => ("Needs authorization", YELLOW),
            }
        };
        text_painter.text(
            pos2(x, rect.top() + 25.0),
            egui::Align2::LEFT_TOP,
            subtitle,
            egui::FontId::proportional(11.0),
            color.lerp_to_gamma(Color32::from_rgb(214, 226, 255), sel),
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("{}\n{}", device.name, kind_label(device.kind)))
}
