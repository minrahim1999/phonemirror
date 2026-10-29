//! Tool discovery settings and missing-tool setup checklist.
use super::components::{detail, muted, notice, surface};
use crate::app::{tools, PhoneMirrorApp, ACCENT, GREEN, RED, TEXT_DIM};

impl PhoneMirrorApp {
    pub(super) fn tools_section(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn setup_checklist(&mut self, ui: &mut egui::Ui, report: &tools::Report) {
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
}
