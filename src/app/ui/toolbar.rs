//! Workspace title and toolbar actions.
use super::desktop_ui::Page;
use super::{
    components::{
        capsule::{chrome_menu, item, menu_item, pill},
        device_row::kind_label,
        icons::Icon,
    },
    theme::{medium, GROUP_GAP, PILL_H, SEG, TRAFFIC_LIGHTS_WIDTH},
};
use crate::app::PhoneMirrorApp;
use crate::app::{TEXT, TEXT_DIM};
use egui::vec2;

impl PhoneMirrorApp {
    fn workspace_title(&self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 1.0;
        let (title, subtitle) = if self.page == Page::Wifi {
            ("Wi-Fi pairing".to_owned(), "Authorize a device".to_owned())
        } else if let Some(device) = self.selected() {
            (device.name.clone(), kind_label(device.kind).to_owned())
        } else {
            ("PhoneMirror".to_owned(), "No device selected".to_owned())
        };
        ui.add(
            egui::Label::new(egui::RichText::new(title).font(medium(13.0)).color(TEXT)).truncate(),
        );
        ui.add(
            egui::Label::new(egui::RichText::new(subtitle).size(11.0).color(TEXT_DIM)).truncate(),
        );
    }

    /// Reserves room for the window buttons and a "Show sidebar" control while collapsed.
    /// Reserves room for the window buttons and a "Show sidebar" control while collapsed.
    fn collapsed_sidebar_controls(&mut self, ui: &mut egui::Ui) {
        if self.sidebar_open {
            return;
        }
        let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if cfg!(target_os = "macos") && !fullscreen {
            ui.add_space(TRAFFIC_LIGHTS_WIDTH - 16.0);
        }
        if pill(ui, &[item(Icon::Sidebar, "Show sidebar", true, false)])[0].clicked() {
            self.sidebar_open = true;
        }
    }

    pub(super) fn wifi_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GROUP_GAP;
            self.collapsed_sidebar_controls(ui);
            let width = (ui.available_width() - 150.0).max(60.0);
            ui.allocate_ui_with_layout(
                vec2(width, PILL_H),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(width);
                    self.workspace_title(ui);
                },
            );
            if ui
                .add(egui::Button::new("Back to devices").corner_radius(PILL_H / 2.0))
                .clicked()
            {
                self.page = Page::Devices;
            }
        });
    }

    pub(super) fn device_toolbar(&mut self, ui: &mut egui::Ui) {
        let ready = self.device_status.connected && self.device_status.scrcpy_health.is_empty();
        let streaming = self.embedded.streaming();
        let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GROUP_GAP;
            self.collapsed_sidebar_controls(ui);
            // Narrow windows move the zoom group into the overflow menu.
            let compact = ui.available_width() < 580.0;
            let (segments, gaps) = if compact { (7.0, 3.0) } else { (11.0, 4.0) };
            let title_width = (ui.available_width() - SEG * segments - GROUP_GAP * gaps).max(60.0);
            ui.allocate_ui_with_layout(
                vec2(title_width, PILL_H),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(title_width);
                    self.workspace_title(ui);
                },
            );

            let input = pill(
                ui,
                &[
                    item(
                        Icon::Keyboard,
                        "Forward keyboard input",
                        streaming,
                        streaming && self.embedded.keyboard_enabled(),
                    ),
                    item(Icon::Split, "Split view", true, self.split.count() > 0),
                ],
            );
            if input[0].clicked() {
                self.embedded.toggle_keyboard();
            }
            chrome_menu(ui, &input[1], "split_menu", |ui| {
                for (extra, label) in [
                    (0, "Single device"),
                    (1, "2 devices"),
                    (2, "3 devices"),
                    (3, "4 devices"),
                ] {
                    if ui
                        .selectable_label(self.split.count() == extra, label)
                        .clicked()
                    {
                        self.embedded.release_touch();
                        self.split.set_count(extra);
                    }
                }
                ui.label(
                    egui::RichText::new("Reducing panes stops their sessions.")
                        .small()
                        .color(TEXT_DIM),
                );
            });

            if !compact {
                let zoom = pill(
                    ui,
                    &[
                        item(Icon::ZoomOut, "Zoom out", streaming, false),
                        item(Icon::ZoomFit, "Fit screen to window", streaming, false),
                        item(
                            Icon::ZoomActual,
                            "Reset zoom to 100% of fit",
                            streaming,
                            false,
                        ),
                        item(Icon::ZoomIn, "Zoom in", streaming, false),
                    ],
                );
                if zoom[0].clicked() {
                    self.embedded.zoom_by(0.8);
                }
                if zoom[1].clicked() {
                    self.embedded.fit();
                }
                if zoom[2].clicked() {
                    self.embedded.fit();
                }
                if zoom[3].clicked() {
                    self.embedded.zoom_by(1.25);
                }
            }

            let window = pill(
                ui,
                &[
                    if fullscreen {
                        item(Icon::Collapse, "Exit full screen", true, false)
                    } else {
                        item(Icon::Expand, "Enter full screen", true, false)
                    },
                    item(Icon::More, "More actions", true, self.is_recording),
                ],
            );
            if window[0].clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
            }
            chrome_menu(ui, &window[1], "more_menu", |ui| {
                let active = self.embedded.active();
                if menu_item(
                    ui,
                    if active {
                        "Stop mirroring"
                    } else {
                        "Start mirroring"
                    },
                    ready || active,
                ) {
                    if active {
                        self.embedded.stop();
                    } else {
                        self.start_mirror(ui.ctx());
                    }
                }
                if menu_item(ui, "Save screenshot", self.device_status.connected) {
                    self.take_screenshot();
                }
                if menu_item(
                    ui,
                    if self.is_recording {
                        "Stop recording"
                    } else {
                        "Record screen"
                    },
                    ready || self.is_recording,
                ) {
                    if self.is_recording {
                        self.stop_recording();
                    } else {
                        self.start_recording();
                    }
                }
                if compact {
                    ui.separator();
                    if menu_item(ui, "Zoom in", streaming) {
                        self.embedded.zoom_by(1.25);
                    }
                    if menu_item(ui, "Zoom out", streaming) {
                        self.embedded.zoom_by(0.8);
                    }
                    if menu_item(ui, "Fit screen to window", streaming) {
                        self.embedded.fit();
                    }
                    if menu_item(ui, "Reset zoom to 100% of fit", streaming) {
                        self.embedded.fit();
                    }
                }
                ui.separator();
                if menu_item(ui, "Pair a Wi-Fi device…", true) {
                    self.embedded.release_touch();
                    self.page = Page::Wifi;
                }
                if menu_item(ui, "Refresh devices", self.refresh_job.is_none()) {
                    self.rescan_tools();
                }
            });

            let panels = pill(
                ui,
                &[
                    item(
                        Icon::Sliders,
                        "Device controls",
                        self.embedded.can_wake(),
                        false,
                    ),
                    item(
                        Icon::Document,
                        "Logs and diagnostics",
                        true,
                        self.drawer.open,
                    ),
                    item(
                        Icon::Info,
                        "Device information",
                        true,
                        self.show_device_info,
                    ),
                ],
            );
            chrome_menu(ui, &panels[0], "device_controls", |ui| {
                if menu_item(ui, "Wake screen", self.embedded.can_wake()) {
                    self.embedded.wake_screen();
                }
                ui.separator();
                for (label, code) in [
                    ("Back", 4),
                    ("Home", 3),
                    ("Recent apps", 187),
                    ("Power", 26),
                    ("Volume down", 25),
                    ("Volume up", 24),
                ] {
                    if menu_item(ui, label, streaming) {
                        self.embedded.navigation_key(code);
                    }
                }
            });
            if panels[1].clicked() {
                self.drawer.open = !self.drawer.open;
                self.embedded.release_touch();
            }
            if panels[2].clicked() {
                self.show_device_info = !self.show_device_info;
            }
        });
    }
}
