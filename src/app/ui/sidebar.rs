//! Searchable device library and its filter menu.
use super::desktop_ui::Page;
use super::{
    components::{
        capsule::{chrome_menu, item, menu_item, pill_at, pill_width},
        device_row::device_row,
        icons::{paint_icon, Icon},
        section_label, window_drag,
    },
    theme::{medium, GROUP_GAP, PILL_BORDER, PILL_H, SEARCH_FILL},
};
use crate::app::PhoneMirrorApp;
use crate::app::{
    devices::{Device, State},
    ACCENT, TEXT_DIM,
};
use egui::{pos2, vec2, Rect, Stroke};

impl PhoneMirrorApp {
    pub(super) fn device_library(&mut self, ui: &mut egui::Ui) {
        // Header row: window buttons on the left (macOS), actions on the right.
        let (header, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), PILL_H), egui::Sense::hover());
        let drag = ui.interact(
            header,
            egui::Id::new("sidebar_drag"),
            egui::Sense::click_and_drag(),
        );
        window_drag(ui, &drag);
        let toggle = Rect::from_min_size(
            pos2(header.right() - PILL_H, header.top()),
            vec2(PILL_H, PILL_H),
        );
        let actions = Rect::from_min_size(
            pos2(toggle.left() - GROUP_GAP - pill_width(2), header.top()),
            vec2(pill_width(2), PILL_H),
        );
        let library = pill_at(
            ui,
            actions,
            &[
                item(Icon::Plus, "Pair a Wi-Fi device", true, false),
                item(Icon::Filter, "Filter devices", true, self.hide_unavailable),
            ],
        );
        if library[0].clicked() {
            self.embedded.release_touch();
            self.page = Page::Wifi;
        }
        chrome_menu(ui, &library[1], "filter_menu", |ui| {
            let mut show = !self.hide_unavailable;
            if ui.checkbox(&mut show, "Show unavailable devices").changed() {
                self.hide_unavailable = !show;
            }
            ui.separator();
            if menu_item(ui, "Refresh devices", self.refresh_job.is_none()) {
                self.rescan_tools();
            }
        });
        if pill_at(
            ui,
            toggle,
            &[item(Icon::Sidebar, "Hide sidebar", true, false)],
        )[0]
        .clicked()
        {
            self.sidebar_open = false;
        }

        // Search field.
        ui.add_space(12.0);
        let (search, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 30.0), egui::Sense::hover());
        ui.painter().rect(
            search,
            search.height() / 2.0,
            SEARCH_FILL,
            Stroke::new(1.0, PILL_BORDER),
            egui::StrokeKind::Inside,
        );
        paint_icon(
            ui.painter(),
            Rect::from_center_size(
                pos2(search.left() + 16.0, search.center().y),
                vec2(20.0, 20.0),
            ),
            Icon::Search,
            TEXT_DIM,
        );
        let field = ui.put(
            Rect::from_min_max(
                pos2(search.left() + 30.0, search.top()),
                pos2(search.right() - 10.0, search.bottom()),
            ),
            egui::TextEdit::singleline(&mut self.device_filter)
                .hint_text("Search")
                .frame(false)
                .font(egui::FontId::proportional(13.0))
                .margin(vec2(0.0, 0.0))
                .vertical_align(egui::Align::Center),
        );
        if field.has_focus() {
            ui.painter().rect_stroke(
                search,
                search.height() / 2.0,
                Stroke::new(1.5, ACCENT),
                egui::StrokeKind::Inside,
            );
        }
        ui.add_space(6.0);

        let query = self.device_filter.trim().to_lowercase();
        let all = self.inventory.devices.clone();
        let matching = |d: &&Device| {
            format!("{} {}", d.name, d.serial.as_deref().unwrap_or(""))
                .to_lowercase()
                .contains(&query)
        };
        let available: Vec<&Device> = all
            .iter()
            .filter(|d| matches!(d.state, State::Online | State::Stopped))
            .filter(matching)
            .collect();
        let unavailable: Vec<&Device> = if self.hide_unavailable {
            Vec::new()
        } else {
            all.iter()
                .filter(|d| matches!(d.state, State::Offline | State::Unauthorized))
                .filter(matching)
                .collect()
        };
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .id_salt("device_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (title, group) in [("Available", &available), ("Unavailable", &unavailable)] {
                    if group.is_empty() {
                        continue;
                    }
                    section_label(ui, title);
                    for device in group.iter() {
                        let starting = self
                            .starting_emulator
                            .as_ref()
                            .is_some_and(|(id, _)| id == &device.id);
                        let selected = self.selected_device.as_deref() == Some(&device.id);
                        if ui
                            .push_id(&device.id, |ui| device_row(ui, device, selected, starting))
                            .inner
                            .clicked()
                        {
                            chosen = Some(device.id.clone());
                        }
                    }
                    ui.add_space(8.0);
                }
                if available.is_empty() && unavailable.is_empty() {
                    ui.add_space(10.0);
                    let (title, body) = if self.refresh_job.is_some() || !self.device_status.checked
                    {
                        (
                            "Finding devices…",
                            "Checking connected phones and installed emulators.",
                        )
                    } else if all.is_empty() {
                        (
                            "No devices",
                            "Connect a phone with USB, or pair one over Wi-Fi with +.",
                        )
                    } else if !query.is_empty() {
                        (
                            "No results",
                            "No device name or serial matches your search.",
                        )
                    } else {
                        (
                            "No available devices",
                            "Show unavailable devices from the filter menu.",
                        )
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            ui.label(egui::RichText::new(title).font(medium(13.0)));
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(body).size(11.0).color(TEXT_DIM),
                                )
                                .wrap(),
                            );
                        });
                    });
                }
            });
        if let Some(id) = chosen {
            self.selected_device = Some(id);
            self.page = Page::Devices;
            self.sync_selection();
        }
    }
}
