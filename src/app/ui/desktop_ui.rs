use crate::app::{
    devices::{Device, Kind, State},
    tools, PhoneMirrorApp, ACCENT, ACCENT_SOFT, ACCENT_TEXT, BG, BORDER, CARD_BG, ERROR_FILL,
    GREEN, INFO_FILL, RED, SELECTED, SIDEBAR, SURFACE_HOVER, TEXT, TEXT_DIM, YELLOW,
};

/// Smoothly fades a hover/selection factor; egui keeps repainting while it animates.
fn ease(ui: &egui::Ui, id: egui::Id, on: bool, seconds: f32) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, seconds)
}

pub fn configure_theme(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.panel_fill = BG;
    v.window_fill = CARD_BG;
    v.extreme_bg_color = egui::Color32::from_rgb(17, 19, 23);
    v.faint_bg_color = SIDEBAR;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT_TEXT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT_TEXT);
    v.text_cursor.stroke = egui::Stroke::new(2.0, ACCENT_TEXT);
    v.window_corner_radius = egui::CornerRadius::same(14);
    v.menu_corner_radius = egui::CornerRadius::same(10);
    v.window_stroke = egui::Stroke::new(1.0, BORDER);
    v.window_shadow = egui::Shadow {
        offset: [0, 12],
        blur: 36,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };
    v.popup_shadow = egui::Shadow {
        offset: [0, 6],
        blur: 20,
        spread: 0,
        color: egui::Color32::from_black_alpha(100),
    };
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    w.inactive.bg_fill = CARD_BG;
    w.inactive.weak_bg_fill = CARD_BG;
    w.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(66, 73, 88));
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
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);
    style.spacing.interact_size.y = 36.0;
    for (role, size) in [
        (egui::TextStyle::Body, 14.0),
        (egui::TextStyle::Button, 14.0),
        (egui::TextStyle::Small, 12.0),
        (egui::TextStyle::Heading, 26.0),
    ] {
        style
            .text_styles
            .insert(role, egui::FontId::proportional(size));
    }
    ctx.set_style(style);
}

pub fn muted(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(TEXT_DIM)).wrap());
}
pub fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add(egui::Label::new(egui::RichText::new(title).size(26.0).strong()).wrap());
    muted(ui, subtitle);
    ui.add_space(6.0);
}
pub fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 44.0],
        egui::Button::new(
            egui::RichText::new(text)
                .strong()
                .color(egui::Color32::WHITE),
        )
        .fill(ACCENT)
        .stroke(egui::Stroke::NONE)
        .corner_radius(10),
    )
}
pub fn surface(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(14)
        .inner_margin(22)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui);
        });
}
pub fn notice(ui: &mut egui::Ui, title: &str, detail: &str, error: bool) {
    let (fill, color) = if error {
        (ERROR_FILL, RED)
    } else {
        (INFO_FILL, ACCENT_TEXT)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.22)))
        .corner_radius(10)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(egui::RichText::new(title).strong().color(color)).wrap());
            ui.add(egui::Label::new(detail).wrap());
        });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Devices,
    Wifi,
}

fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Emulator => "Android emulator",
        Kind::Usb => "USB device",
        Kind::Wifi => "Wi-Fi device",
    }
}
fn state_label(state: State) -> (&'static str, egui::Color32) {
    match state {
        State::Online => ("Connected", GREEN),
        State::Stopped => ("Shut down", TEXT_DIM),
        State::Offline => ("Offline", YELLOW),
        State::Unauthorized => ("Needs authorization", YELLOW),
    }
}
#[derive(Clone, Copy)]
enum Icon {
    Devices,
    Wifi,
    Play,
    Stop,
    Expand,
    Camera,
}
fn paint_icon(p: &egui::Painter, rect: egui::Rect, icon: Icon, color: egui::Color32) {
    let c = rect.center();
    let stroke = egui::Stroke::new(1.6, color);
    match icon {
        Icon::Devices => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(12.0, 19.0)),
                3.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            p.circle_filled(c + egui::vec2(0.0, 6.0), 1.0, color);
        }
        Icon::Wifi => {
            for radius in [6.0_f32, 11.0] {
                let points: Vec<_> = (0..17)
                    .map(|i| {
                        let angle = std::f32::consts::PI * (1.25 + i as f32 / 32.0);
                        c + egui::vec2(angle.cos(), angle.sin()) * radius
                    })
                    .collect();
                p.add(egui::Shape::line(points, stroke));
            }
            p.circle_filled(c + egui::vec2(0.0, 3.0), 1.8, color);
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + egui::vec2(-4.0, -7.0),
                    c + egui::vec2(7.0, 0.0),
                    c + egui::vec2(-4.0, 7.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        Icon::Stop => {
            p.rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(12.0, 12.0)),
                2.0,
                color,
            );
        }
        Icon::Expand => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let corner = c + egui::vec2(8.0 * x, 8.0 * y);
                p.line_segment([corner, corner - egui::vec2(5.0 * x, 0.0)], stroke);
                p.line_segment([corner, corner - egui::vec2(0.0, 5.0 * y)], stroke);
            }
        }
        Icon::Camera => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(21.0, 15.0)),
                3.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            p.circle_stroke(c, 4.0, stroke);
            p.line_segment(
                [c + egui::vec2(-4.0, -9.0), c + egui::vec2(4.0, -9.0)],
                stroke,
            );
        }
    }
}

fn device_row(
    ui: &mut egui::Ui,
    device: &Device,
    selected: bool,
    starting: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0), egui::Sense::click());
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
            .gamma_multiply(hover)
            .lerp_to_gamma(SELECTED, sel);
        p.rect_filled(rect, 10.0, fill);
        if response.has_focus() {
            p.rect_stroke(
                rect.shrink(1.0),
                10.0,
                egui::Stroke::new(1.5, ACCENT),
                egui::StrokeKind::Inside,
            );
        }
        if sel > 0.0 {
            p.rect_filled(
                egui::Rect::from_center_size(
                    egui::pos2(rect.left() + 1.5, rect.center().y),
                    egui::vec2(3.0, 24.0 * sel),
                ),
                2.0,
                ACCENT.gamma_multiply(sel),
            );
        }
        let icon_rect =
            egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 9.0), egui::vec2(28.0, 35.0));
        p.rect_filled(icon_rect, 8.0, CARD_BG.lerp_to_gamma(ACCENT_SOFT, sel));
        paint_icon(
            p,
            icon_rect,
            Icon::Devices,
            TEXT_DIM.lerp_to_gamma(ACCENT_TEXT, sel),
        );
        let (state, color) = state_label(device.state);
        let x = rect.left() + 46.0;
        let text_painter = p.with_clip_rect(egui::Rect::from_min_max(
            egui::pos2(x, rect.top()),
            rect.max - egui::vec2(10.0, 0.0),
        ));
        let mut job = egui::text::LayoutJob::simple(
            device.name.clone(),
            egui::FontId::proportional(14.0),
            TEXT,
            (rect.width() - 68.0).max(20.0),
        );
        job.wrap.max_rows = 1;
        let galley = text_painter.layout_job(job);
        text_painter.galley(egui::pos2(x, rect.top() + 9.0), galley, TEXT);
        text_painter.circle_filled(egui::pos2(x + 3.0, rect.top() + 36.0), 2.5, color);
        text_painter.text(
            egui::pos2(x + 13.0, rect.top() + 29.0),
            egui::Align2::LEFT_TOP,
            if starting { "Starting…" } else { state },
            egui::FontId::proportional(12.0),
            color,
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{}
{}",
            device.name,
            kind_label(device.kind)
        ))
}

fn tool(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<Icon>,
    enabled: bool,
    selected: bool,
) -> egui::Response {
    // Reserve the background slot first so the animated fill sits beneath the label.
    let background = ui.painter().add(egui::Shape::Noop);
    let button = egui::Button::new(if icon.is_some() { "" } else { label })
        .frame(false)
        .min_size(egui::vec2(if icon.is_some() { 28.0 } else { 34.0 }, 26.0));
    let response = ui.add_enabled(enabled, button);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, label)
    });
    let hover = ease(
        ui,
        response.id.with("hover"),
        enabled && response.hovered(),
        0.12,
    );
    // A disabled control never shows its "on" highlight; it would read as active.
    let sel = ease(ui, response.id.with("selected"), enabled && selected, 0.16);
    let pressed = if response.is_pointer_button_down_on() {
        0.08
    } else {
        0.0
    };
    let fill = SURFACE_HOVER
        .gamma_multiply((hover + pressed).min(1.0))
        .lerp_to_gamma(ACCENT_SOFT, sel);
    ui.painter().set(
        background,
        egui::epaint::RectShape::filled(response.rect, 7, fill),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            7.0,
            egui::Stroke::new(1.5, ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(icon) = icon {
        paint_icon(
            ui.painter(),
            response.rect,
            icon,
            if !enabled {
                TEXT_DIM.gamma_multiply(0.45)
            } else {
                TEXT.lerp_to_gamma(ACCENT_TEXT, sel)
            },
        );
    }
    response.on_hover_text(label)
}
fn tool_group(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, BORDER.gamma_multiply(0.7)))
        .corner_radius(7)
        .inner_margin(2)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                content(ui);
            });
        });
}

fn detail(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).small().color(TEXT_DIM));
    ui.horizontal_top(|ui| {
        let width = (ui.available_width() - 58.0).max(40.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
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

impl PhoneMirrorApp {
    pub(crate) fn show_workspace(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("device_library")
            .resizable(false)
            .exact_width(if ctx.screen_rect().width() < 900.0 {
                220.0
            } else {
                248.0
            })
            .frame(egui::Frame::new().fill(SIDEBAR).inner_margin(12))
            .show(ctx, |ui| self.device_library(ui));
        egui::TopBottomPanel::top("hub_toolbar")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                if self.page == Page::Devices && ui.available_width() >= 580.0 {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(170.0, 36.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| self.workspace_title(ui),
                        );
                        self.device_toolbar(ui);
                    });
                } else {
                    self.workspace_title(ui);
                    if self.page == Page::Devices {
                        self.device_toolbar(ui);
                    } else if ui.button("Back to devices").clicked() {
                        self.page = Page::Devices;
                    }
                }
            });
        egui::TopBottomPanel::bottom("canvas_status")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(16, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button(if self.drawer.open { "Hide tools" } else { "Tools" }).clicked() {
                        self.drawer.open = !self.drawer.open;
                        self.embedded.release_touch();
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
                    } else {
                        ui.label(
                            egui::RichText::new(if self.wireless.busy() {
                                "Wi-Fi pairing in progress"
                            } else if self.embedded.streaming() {
                                "Click screen to control · drag to swipe · Esc releases keyboard"
                            } else if self.refresh_job.is_some() || !self.device_status.checked {
                                "Refreshing devices…"
                            } else {
                                "Select a device to begin"
                            })
                            .size(11.0)
                            .color(TEXT_DIM),
                        );
                    }
                    if self.is_recording {
                        ui.colored_label(RED, egui::RichText::new("Recording").small());
                    }
                });
            });
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
                    if let Some(device) = self
                        .inventory
                        .devices
                        .iter()
                        .find(|d| Some(&d.id) == self.selected_device.as_ref())
                        .cloned()
                    {
                        detail(ui, "Name", &device.name);
                        detail(ui, "Connection", kind_label(device.kind));
                        if let Some(serial) = &device.serial {
                            detail(ui, "Serial", serial);
                            if device.kind == Kind::Wifi {
                                muted(ui, "Disconnect only this connection. Pairing and other connections stay intact; ADB may rediscover it.");
                                if ui.add_enabled(!self.wireless.busy() && !self.embedded.active() && !self.is_recording, egui::Button::new("Disconnect this wireless connection")).clicked() {
                                    self.wireless.disconnect(ui.ctx(), serial.clone());
                                    self.page = Page::Wifi;
                                }
                                if self.embedded.active() || self.is_recording {
                                    muted(ui, "Stop PhoneMirror sessions before disconnecting.");
                                }
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

    fn workspace_title(&self, ui: &mut egui::Ui) {
        let name = if self.page == Page::Wifi {
            "Wi-Fi pairing"
        } else if self.device_status.device_id.is_empty() {
            "PhoneMirror"
        } else {
            &self.device_status.device_id
        };
        ui.add(egui::Label::new(egui::RichText::new(name).strong().size(14.0)).truncate());
        ui.label(
            egui::RichText::new(if self.page == Page::Wifi {
                "Authorize a device"
            } else if self.embedded.active() {
                "Live screen"
            } else {
                "Android workspace"
            })
            .size(11.0)
            .color(TEXT_DIM),
        );
    }

    fn device_toolbar(&mut self, ui: &mut egui::Ui) {
        let ready = self.device_status.connected && self.device_status.scrcpy_health.is_empty();
        let streaming = self.embedded.streaming();
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);
        ui.spacing_mut().button_padding = egui::vec2(7.0, 4.0);
        ui.spacing_mut().interact_size.y = 26.0;
        ui.horizontal_wrapped(|ui| {
            tool_group(ui, |ui| {
                if tool(ui, if self.embedded.active() { "Stop mirroring" } else { "Start mirroring here" },
                    Some(if self.embedded.active() { Icon::Stop } else { Icon::Play }),
                    ready || self.embedded.active(), self.embedded.active()).clicked() {
                    if self.embedded.active() { self.embedded.stop(); } else { self.start_mirror(ui.ctx()); }
                }
                if tool(ui, "Save screenshot", Some(Icon::Camera), self.device_status.connected, false).clicked() {
                    self.take_screenshot();
                }
            });
            ui.menu_button(if self.is_recording { "Device •" } else { "Device" }, |ui| {
                if ui.add_enabled(ready || self.is_recording, egui::Button::new(if self.is_recording { "Stop recording" } else { "Record screen" })).clicked() {
                    if self.is_recording { self.stop_recording(); } else { self.start_recording(); }
                    ui.close_menu();
                }
                if ui.add_enabled(streaming, egui::Button::new("Forward keyboard input").selected(self.embedded.keyboard_enabled())).clicked() {
                    self.embedded.toggle_keyboard();
                    ui.close_menu();
                }
                if ui.add_enabled(self.embedded.can_wake(), egui::Button::new("Wake screen")).clicked() {
                    self.embedded.wake_screen();
                    ui.close_menu();
                }
                ui.add_enabled_ui(streaming, |ui| {
                    for (label, code) in [("Back", 4), ("Home", 3), ("Recent apps", 187), ("Power", 26), ("Volume down", 25), ("Volume up", 24)] {
                        if ui.button(label).clicked() { self.embedded.navigation_key(code); ui.close_menu(); }
                    }
                    ui.separator();
                    if ui.button("Zoom in").clicked() { self.embedded.zoom_by(1.25); ui.close_menu(); }
                    if ui.button("Zoom out").clicked() { self.embedded.zoom_by(0.8); ui.close_menu(); }
                });
                ui.separator();
                if ui.button("Device information & tools").clicked() {
                    self.show_device_info = !self.show_device_info;
                    ui.close_menu();
                }
                if ui.button("Pair a Wi-Fi device").clicked() {
                    self.embedded.release_touch(); self.page = Page::Wifi; ui.close_menu();
                }
            });
            tool_group(ui, |ui| {
                if tool(ui, "Fit", None, streaming, false).on_hover_text("Fit screen to pane").clicked() { self.embedded.fit(); }
                if tool(ui, "Toggle fullscreen", Some(Icon::Expand), true, false).clicked() {
                    let full = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                }
            });
            ui.menu_button(if self.split.count() == 0 { "Split" } else { "Split •" }, |ui| {
                for (extra, label) in [(0, "Single device"), (1, "2 devices"), (2, "3 devices"), (3, "4 devices")] {
                    if ui.selectable_label(self.split.count() == extra, label).clicked() {
                        self.embedded.release_touch();
                        self.split.set_count(extra);
                        ui.close_menu();
                    }
                }
                ui.label(egui::RichText::new("Reducing panes stops their sessions.").small().color(TEXT_DIM));
            });
        });
    }

    fn device_library(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let id = egui::Id::new("launcher_icon");
            let texture = ui.ctx().data_mut(|data| data.get_temp::<egui::TextureHandle>(id))
                .unwrap_or_else(|| {
                    let icon = crate::app::load_icon();
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [icon.width as usize, icon.height as usize],
                        &icon.rgba,
                    );
                    let texture = ui.ctx().load_texture(
                        "PhoneMirror launcher", image, egui::TextureOptions::LINEAR,
                    );
                    ui.ctx().data_mut(|data| data.insert_temp(id, texture.clone()));
                    texture
                });
            ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(28.0, 28.0)))
                .on_hover_text("PhoneMirror");
            ui.label(egui::RichText::new("PhoneMirror").size(13.0).strong());
            #[cfg(not(target_os = "macos"))]
            ui.menu_button("Settings", |ui| {
                if ui.add_enabled(self.refresh_job.is_none(), egui::Button::new("Refresh devices")).clicked() {
                    self.rescan_tools();
                    ui.close_menu();
                }
                if ui.button("Pair a Wi-Fi device…").clicked() {
                    self.embedded.release_touch();
                    self.page = Page::Wifi;
                    ui.close_menu();
                }
            });
        });
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.device_filter)
                .hint_text("Search devices")
                .font(egui::TextStyle::Small)
                .margin(egui::vec2(10.0, 7.0))
                .desired_width(ui.available_width()),
        );
        ui.add_space(8.0);
        let query = self.device_filter.trim().to_lowercase();
        let all = self.inventory.devices.clone();
        egui::ScrollArea::vertical()
            .id_salt("device_list")
            .auto_shrink([false, false])
            .max_height((ui.available_height() - 42.0).max(80.0))
            .show(ui, |ui| {
                for (title, emulators) in [("Emulators", true), ("Physical devices", false)] {
                    let group: Vec<_> = all
                        .iter()
                        .filter(|d| (d.kind == Kind::Emulator) == emulators)
                        .collect();
                    ui.label(
                        egui::RichText::new(format!("{title}  {}", group.len()))
                            .size(11.0)
                            .color(TEXT_DIM),
                    );
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let visible: Vec<_> = group
                        .iter()
                        .filter(|d| {
                            format!("{} {}", d.name, d.serial.as_deref().unwrap_or(""))
                                .to_lowercase()
                                .contains(&query)
                        })
                        .collect();
                    if visible.is_empty() {
                        ui.label(
                            egui::RichText::new(if !query.is_empty() {
                                "No matches"
                            } else if emulators {
                                "No virtual devices"
                            } else {
                                "Connect via USB or Wi-Fi"
                            })
                            .size(11.0)
                            .color(TEXT_DIM),
                        );
                    }
                    for device in visible {
                        let starting = self
                            .starting_emulator
                            .as_ref()
                            .is_some_and(|(id, _)| id == &device.id);
                        if ui
                            .push_id(&device.id, |ui| {
                                device_row(
                                    ui,
                                    device,
                                    self.selected_device.as_deref() == Some(&device.id),
                                    starting,
                                )
                            })
                            .inner
                            .clicked()
                        {
                            self.selected_device = Some(device.id.clone());
                            self.page = Page::Devices;
                            self.sync_selection();
                        }
                    }
                    ui.add_space(18.0);
                }
            });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!(
                "{} connected",
                all.iter().filter(|d| d.state == State::Online).count()
            ))
            .size(11.0)
            .color(TEXT_DIM),
        );
    }

    fn device_canvas(&mut self, ui: &mut egui::Ui) {
        if self.split.count() == 0 {
            self.single_device_canvas(ui);
            return;
        }
        let size = ui.available_size();
        let gap = 8.0;
        ui.spacing_mut().item_spacing.x = gap;
        let pane_width = ((size.x - gap) / 2.0).max(1.0);
        let ready = self.device_status.tools.as_ref().is_some_and(|report| !report.missing())
            && self.device_status.scrcpy_health.is_empty();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(egui::vec2(pane_width, size.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.push_id("primary_mirror_pane", |ui| self.single_device_canvas(ui));
            });
            ui.allocate_ui_with_layout(egui::vec2(pane_width, size.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                self.split.show(ui, &self.inventory, &self.device_status.device_serial, ready);
            });
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
                paint_icon(ui.painter(), r, Icon::Devices, TEXT_DIM);
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

    fn tools_section(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Tools").strong());
        muted(
            ui,
            "PhoneMirror finds these programs automatically. Enter a path to use a specific copy instead.",
        );
        let report = self.device_status.tools.clone();
        let mut inputs = self.tool_inputs.take().unwrap_or_else(tools::custom_inputs);
        let can_browse = self.browse_job.is_none();
        let mut save = None;
        let mut browse = None;
        for tool in tools::ALL {
            ui.add_space(6.0);
            let found = report.as_ref().map(|r| r.get(tool));
            let (status, color) = match found {
                None => ("Checking…".to_string(), TEXT_DIM),
                Some(r) if r.path.is_some() && r.version.is_empty() => {
                    (format!("Found in {}", r.source), GREEN)
                }
                Some(r) if r.path.is_some() => (format!("{} from {}", r.version, r.source), GREEN),
                Some(_) => ("Not found".to_string(), RED),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(tool.label()).strong());
                ui.label(egui::RichText::new(status).small().color(color));
            });
            if let Some(path) = found.and_then(|r| r.path.as_deref()) {
                detail(ui, "Path", path);
            }
            for warning in found.map(|r| r.warnings.as_slice()).unwrap_or_default() {
                notice(
                    ui,
                    &format!("Check your {} setup", tool.label()),
                    warning,
                    true,
                );
            }
            let input = &mut inputs[tool.index()];
            ui.label(egui::RichText::new("Custom path").small().color(TEXT_DIM));
            ui.add(
                egui::TextEdit::singleline(input)
                    .id_salt(("tool-path", tool.key()))
                    .hint_text("Automatic")
                    .font(egui::TextStyle::Small)
                    .desired_width(ui.available_width()),
            );
            ui.horizontal_wrapped(|ui| {
                if cfg!(target_os = "macos")
                    && ui
                        .add_enabled(can_browse, egui::Button::new("Choose…"))
                        .clicked()
                {
                    browse = Some(tool);
                }
                if ui
                    .add_enabled(!input.trim().is_empty(), egui::Button::new("Use this path"))
                    .clicked()
                {
                    save = Some((tool, Some(input.clone())));
                }
                if ui.button("Use automatic").clicked() {
                    input.clear();
                    save = Some((tool, None));
                }
            });
        }
        ui.add_space(6.0);
        if ui
            .add_enabled(
                self.refresh_job.is_none(),
                egui::Button::new("Check tools again"),
            )
            .clicked()
        {
            self.rescan_tools();
        }
        if let Some(file) = tools::config_file() {
            muted(ui, &format!("Custom paths are saved in {}", file.display()));
        }
        self.tool_inputs = Some(inputs);
        if let Some((tool, path)) = save {
            self.save_tool_path(tool, path);
        }
        if let Some(tool) = browse {
            self.browse_tool(tool);
        }
    }

    fn setup_checklist(&mut self, ui: &mut egui::Ui, report: &tools::Report) {
        let available = ui.available_size();
        let width = available.x.min(520.0);
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.add_space((available.y * 0.1).max(0.0));
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        surface(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new("Finish setting up PhoneMirror")
                                        .size(20.0)
                                        .strong(),
                                )
                                .wrap(),
                            );
                            muted(
                                ui,
                                "PhoneMirror uses these programs on your computer. Install what's missing, then check again.",
                            );
                            for tool in tools::ALL {
                                let found = report.get(tool);
                                ui.add_space(10.0);
                                ui.horizontal_wrapped(|ui| {
                                    let (dot, _) = ui.allocate_exact_size(
                                        egui::vec2(10.0, 18.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(
                                        dot.center(),
                                        4.0,
                                        if found.path.is_some() { GREEN } else { RED },
                                    );
                                    ui.label(egui::RichText::new(tool.label()).strong());
                                    ui.label(
                                        egui::RichText::new(tool.purpose())
                                            .small()
                                            .color(TEXT_DIM),
                                    );
                                });
                                if let Some(path) = &found.path {
                                    muted(ui, &format!("Installed at {path}"));
                                    continue;
                                }
                                egui::Frame::new()
                                    .fill(ui.visuals().extreme_bg_color)
                                    .corner_radius(8)
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal_top(|ui| {
                                            let text_width = (ui.available_width() - 58.0).max(40.0);
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(text_width, 0.0),
                                                egui::Layout::top_down(egui::Align::Min),
                                                |ui| {
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(
                                                                tool.install_command(),
                                                            )
                                                            .monospace(),
                                                        )
                                                        .wrap(),
                                                    );
                                                },
                                            );
                                            if ui
                                                .small_button("Copy")
                                                .on_hover_text("Copy install command")
                                                .clicked()
                                            {
                                                ui.ctx().copy_text(tool.install_command().into());
                                            }
                                        });
                                    });
                            }
                            ui.add_space(14.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add_enabled(
                                        self.refresh_job.is_none(),
                                        egui::Button::new(
                                            egui::RichText::new("Check again")
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        )
                                        .fill(ACCENT)
                                        .stroke(egui::Stroke::NONE)
                                        .corner_radius(10)
                                        .min_size(egui::vec2(130.0, 38.0)),
                                    )
                                    .clicked()
                                {
                                    self.rescan_tools();
                                }
                                if ui
                                    .add(
                                        egui::Button::new("Set paths manually")
                                            .min_size(egui::vec2(0.0, 38.0)),
                                    )
                                    .clicked()
                                {
                                    self.show_device_info = true;
                                }
                            });
                        });
                    },
                );
            },
        );
    }

    fn wifi_page(&mut self, ui: &mut egui::Ui) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::devices::{Device, Kind, State};

    fn sample(id: &str, name: &str, kind: Kind, state: State) -> Device {
        Device {
            id: id.into(),
            name: name.into(),
            serial: (state != State::Stopped).then(|| id.into()),
            avd: (kind == Kind::Emulator).then(|| name.into()),
            kind,
            state,
        }
    }

    #[test]
    fn compact_toolbar_stays_under_32_points() {
        let ctx = egui::Context::default();
        configure_theme(&ctx);
        let mut app = PhoneMirrorApp::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = ui.scope(|ui| app.device_toolbar(ui));
                assert!(response.response.rect.height() <= 32.0, "toolbar height: {}", response.response.rect.height());
            });
        });
    }

    #[test]
    fn split_workspace_fits_minimum_window() {
        for width in [720.0, 1060.0, 1440.0] {
            for extra in 1..=3 {
                let ctx = egui::Context::default();
                configure_theme(&ctx);
                let mut app = PhoneMirrorApp::default();
                app.split.set_count(extra);
                for _ in 0..2 {
                    let output = ctx.run(egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 820.0))),
                        ..Default::default()
                    }, |ctx| app.show_workspace(ctx));
                    for shape in output.shapes {
                        let rect = shape.shape.visual_bounding_rect();
                        if rect.is_finite() && rect.is_positive() {
                            assert!(rect.right() <= width + 8.0, "split {extra} overflow at {width}: {rect:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sidebar_reuses_launcher_texture_across_frames() {
        let ctx = egui::Context::default();
        configure_theme(&ctx);
        let mut app = PhoneMirrorApp::default();
        let mut texture_id = None;
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.device_library(ui));
            });
            let texture = ctx.data_mut(|data| {
                data.get_temp::<egui::TextureHandle>(egui::Id::new("launcher_icon"))
            }).expect("sidebar must load the launcher icon");
            assert_eq!(texture.size(), [64, 64]);
            if let Some(previous) = texture_id {
                assert_eq!(previous, texture.id());
            }
            texture_id = Some(texture.id());
        }
    }

    #[test]
    fn hub_and_pairing_render_without_horizontal_overflow() {
        for width in [720.0, 940.0, 1060.0, 1440.0] {
            for page in [Page::Devices, Page::Wifi] {
                for selection in [
                    None,
                    Some("avd:Pixel"),
                    Some("emulator-5554"),
                    Some("USB123"),
                    Some("192.168.1.7:33445"),
                ] {
                    let ctx = egui::Context::default();
                    configure_theme(&ctx);
                    let mut app = PhoneMirrorApp::default();
                    app.page = page;
                    if selection.is_some() {
                        app.inventory.devices = vec![
                            sample("avd:Pixel", "Pixel 9 Pro", Kind::Emulator, State::Stopped),
                            sample(
                                "emulator-5554",
                                "Pixel Tablet",
                                Kind::Emulator,
                                State::Online,
                            ),
                            sample("USB123", "Samsung Galaxy", Kind::Usb, State::Unauthorized),
                            sample("192.168.1.7:33445", "Pixel 8", Kind::Wifi, State::Offline),
                        ];
                        app.selected_device = selection.map(str::to_owned);
                        app.sync_selection();
                    }
                    for _ in 0..2 {
                        let output = ctx.run(
                            egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    egui::vec2(width, 820.0),
                                )),
                                ..Default::default()
                            },
                            |ctx| app.show_workspace(ctx),
                        );
                        for shape in &output.shapes {
                            let rect = shape.shape.visual_bounding_rect();
                            if rect.is_finite() && rect.is_positive() {
                                assert!(
                                    rect.right() <= width + 8.0,
                                    "overflow at width {width}, {page:?}: {rect:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn device_rows_support_keyboard_activation() {
        let ctx = egui::Context::default();
        configure_theme(&ctx);
        let device = sample("usb", "Pixel 9", Kind::Usb, State::Online);
        let mut activated = false;
        for frame in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    events: if frame == 1 {
                        vec![egui::Event::Key {
                            key: egui::Key::Enter,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = device_row(ui, &device, false, false);
                        if frame == 0 {
                            response.request_focus();
                        }
                        activated |= response.clicked();
                    });
                },
            );
        }
        assert!(activated, "Focused device rows must activate with Enter");
    }

    #[test]
    fn long_identifiers_and_active_error_states_fit_minimum_window() {
        for page in [Page::Devices, Page::Wifi] {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            let mut app = PhoneMirrorApp::default();
            app.page = page;
            app.mirror_running = true;
            app.is_recording = true;
            app.status_message = "Recording is active on this computer.".into();
            app.show_device_info = true;
            let long = "adb-LongDeviceName (2)._adb-tls-connect._tcp".repeat(5);
            app.inventory.devices = vec![sample(
                &long,
                &"Pixel Android Emulator ".repeat(10),
                Kind::Wifi,
                State::Online,
            )];
            app.device_status.scrcpy_health =
                "Cannot open scrcpy. Check the installed version and dependencies.".into();
            app.sync_selection();
            for _ in 0..2 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(720.0, 620.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.show_workspace(ctx),
                );
                for shape in output.shapes {
                    let rect = shape.shape.visual_bounding_rect();
                    if rect.is_finite() && rect.is_positive() {
                        assert!(rect.right() <= 728.0, "long content overflow: {rect:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn embedded_screen_states_fit_hub_without_external_window() {
        for width in [720.0, 1060.0, 1440.0] {
            for (state, dimensions) in [
                (crate::app::mirror::Status::Starting, [2, 3]),
                (crate::app::mirror::Status::Streaming, [2, 3]),
                (crate::app::mirror::Status::Streaming, [3, 2]),
                (
                    crate::app::mirror::Status::Failed(
                        "FFmpeg cannot decode this screen. Check the matching scrcpy server."
                            .into(),
                    ),
                    [2, 3],
                ),
            ] {
                let ctx = egui::Context::default();
                configure_theme(&ctx);
                let mut app = PhoneMirrorApp::default();
                app.inventory.devices =
                    vec![sample("fixture", "Pixel 9", Kind::Emulator, State::Online)];
                app.sync_selection();
                app.embedded = crate::app::mirror_ui::MirrorUi::fixture(
                    "fixture",
                    state,
                    Some(crate::app::mirror::Frame {
                        width: dimensions[0],
                        height: dimensions[1],
                        rgb: vec![100; (dimensions[0] * dimensions[1] * 3) as usize],
                    }),
                );
                app.embedded.poll(&ctx);
                for _ in 0..2 {
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, 620.0),
                            )),
                            ..Default::default()
                        },
                        |ctx| app.show_workspace(ctx),
                    );
                    for shape in output.shapes {
                        let rect = shape.shape.visual_bounding_rect();
                        if rect.is_finite() && rect.is_positive() {
                            assert!(
                                rect.right() <= width + 8.0,
                                "embedded overflow {width}: {rect:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn setup_checklist_and_tools_panel_fit_minimum_window() {
        let found = |path: &str, version: &str| tools::Resolution {
            path: Some(path.into()),
            source: "the Android SDK".into(),
            version: version.into(),
            warnings: vec![format!(
                "Different ADB versions are installed: {}",
                path.repeat(6)
            )],
        };
        let report = tools::Report {
            tools: [
                found(
                    "/Users/someone/Library/Android/sdk/platform-tools/adb",
                    "37.0.1",
                ),
                tools::Resolution::default(),
                found("/opt/homebrew/bin/ffmpeg", "8.0"),
            ],
        };
        for width in [720.0, 1060.0] {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            let mut app = PhoneMirrorApp::default();
            app.device_status.tools = Some(report.clone());
            app.tool_inputs = Some(Default::default());
            app.show_device_info = true;
            for _ in 0..2 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 620.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.show_workspace(ctx),
                );
                for shape in output.shapes {
                    let rect = shape.shape.visual_bounding_rect();
                    if rect.is_finite() && rect.is_positive() {
                        assert!(
                            rect.right() <= width + 8.0,
                            "tools overflow {width}: {rect:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn selected_device_controls_target_and_disconnect_never_redirects() {
        let mut app = PhoneMirrorApp::default();
        app.inventory.devices = vec![
            sample("usb-a", "Phone A", Kind::Usb, State::Online),
            sample("usb-b", "Phone B", Kind::Usb, State::Online),
        ];
        app.selected_device = Some("usb-b".into());
        app.sync_selection();
        assert_eq!(app.device_status.device_serial, "usb-b");
        app.inventory.devices.pop();
        app.sync_selection();
        assert!(!app.device_status.connected);
        assert!(app.device_status.device_serial.is_empty());
        assert_eq!(app.selected_device.as_deref(), Some("usb-b"));
    }

    #[test]
    fn stopped_and_unauthorized_selections_disable_actions() {
        for state in [State::Stopped, State::Offline, State::Unauthorized] {
            let mut app = PhoneMirrorApp::default();
            app.inventory.devices = vec![sample("one", "One", Kind::Emulator, state)];
            app.sync_selection();
            assert!(!app.device_status.connected);
            assert!(app.device_status.device_serial.is_empty());
        }
    }
}
