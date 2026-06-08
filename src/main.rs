#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

fn main() -> eframe::Result {
    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PhoneMirror")
            .with_inner_size([400.0, 600.0])
            .with_min_inner_size([320.0, 480.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "PhoneMirror",
        options,
        Box::new(|_cc| Ok(Box::new(PhoneMirrorApp::default()))),
    )
}

fn load_icon() -> egui::IconData {
    let size = 64u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;

            let phone_left = 18.0;
            let phone_right = 46.0;
            let phone_top = 6.0;
            let phone_bottom = 56.0;
            let in_phone = fx >= phone_left && fx <= phone_right && fy >= phone_top && fy <= phone_bottom;

            let screen_left = 21.0;
            let screen_right = 43.0;
            let screen_top = 12.0;
            let screen_bottom = 50.0;
            let in_screen = fx >= screen_left && fx <= screen_right && fy >= screen_top && fy <= screen_bottom;

            let in_reflection = in_screen
                && fy > 22.0 && fy < 40.0
                && fx > 26.0 && fx < 40.0
                && ((fy - fx + 10.0).abs() < 2.5 || (fy + fx - 72.0).abs() < 2.5);

            let in_home = (fx - 32.0).powi(2) + (fy - 53.0).powi(2) < 6.0;

            let (r, g, b, a) = if in_reflection {
                (100, 180, 255, 230)
            } else if in_screen {
                (15, 18, 30, 255)
            } else if in_home {
                (60, 70, 100, 255)
            } else if in_phone {
                (40, 50, 70, 255)
            } else {
                (0, 0, 0, 0)
            };

            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }

    egui::IconData { rgba, width: size, height: size }
}

// ─── Colors ───────────────────────────────────────────────

const BG: egui::Color32 = egui::Color32::from_rgb(18, 18, 26);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(30, 32, 46);
const BORDER: egui::Color32 = egui::Color32::from_rgb(50, 54, 72);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(88, 166, 255);
const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(30, 58, 90);
const GREEN: egui::Color32 = egui::Color32::from_rgb(76, 196, 136);
const GREEN_DIM: egui::Color32 = egui::Color32::from_rgb(25, 65, 45);
const RED: egui::Color32 = egui::Color32::from_rgb(235, 87, 87);
const RED_DIM: egui::Color32 = egui::Color32::from_rgb(75, 25, 25);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 193, 7);
const RECORD_RED: egui::Color32 = egui::Color32::from_rgb(220, 50, 50);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 222, 230);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(130, 134, 150);
const LABEL_W: f32 = 55.0; // consistent label column width

fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(14))
        .outer_margin(egui::Margin::same(0))
}

/// Show a card at full available width
fn show_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    card_frame().show(ui, |ui| {
        // Inside the card, available_width is already reduced by inner_margin
        // Just let content fill the available card width
        ui.set_width(ui.available_width());
        add_contents(ui);
    });
}

// ─── Truncate long paths ─────────────────────────────────

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else if let Some(slash_pos) = path.rfind('/') {
        let filename = &path[slash_pos + 1..];
        if filename.len() >= max_len {
            format!("…{}", &filename[..max_len - 1])
        } else {
            format!("…/{}", filename)
        }
    } else {
        format!("…{}", &path[path.len() - max_len + 1..])
    }
}

// ─── App ──────────────────────────────────────────────────

struct PhoneMirrorApp {
    device_status: DeviceStatus,
    mirror_running: bool,
    is_recording: bool,
    status_message: String,
    status_is_error: bool,
    last_refresh: Instant,
    pulse_time: f32,
    show_close_warning: bool,
}

#[derive(Clone)]
struct DeviceStatus {
    connected: bool,
    device_id: String,
    /// The serial to pass to scrcpy/adb with -s (prefers USB over TCP/IP)
    device_serial: String,
    adb_path: String,
    scrcpy_path: String,
    checked: bool,
}

impl Default for PhoneMirrorApp {
    fn default() -> Self {
        Self {
            device_status: DeviceStatus {
                connected: false,
                device_id: String::new(),
                device_serial: String::new(),
                adb_path: String::new(),
                scrcpy_path: String::new(),
                checked: false,
            },
            mirror_running: false,
            is_recording: false,
            status_message: String::new(),
            status_is_error: false,
            last_refresh: Instant::now(),
            pulse_time: 0.0,
            show_close_warning: false,
        }
    }
}

impl eframe::App for PhoneMirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_refresh.elapsed() > Duration::from_secs(5) {
            self.refresh_device();
        }

        self.pulse_time = (self.pulse_time + ctx.input(|i| i.stable_dt.min(0.1))) % 4.0;

        // ── Theme ──
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = BG;
        style.visuals.extreme_bg_color = BG;
        style.visuals.window_fill = BG;
        style.visuals.widgets.inactive.bg_fill = CARD_BG;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
        ctx.set_style(style);

        // ── Close interception ──
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            if self.mirror_running || self.is_recording {
                self.show_close_warning = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // ── Close warning dialog ──
        if self.show_close_warning {
            let dialog_width = 300.0;
            // Dim overlay behind dialog
            ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("close_overlay")))
                .rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 120));

            egui::Window::new("")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .frame(egui::Frame::new()
                    .fill(egui::Color32::from_rgb(42, 44, 60))
                    .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 104, 120)))
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(20, 16)))
                .show(ctx, |ui| {
                    ui.set_width(dialog_width);

                    // ── Icon + Title + Description ──
                    ui.vertical_centered(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(44.0, 44.0), egui::Sense::hover());
                        let center = rect.center();
                        ui.painter().circle_filled(center, 20.0, egui::Color32::from_rgba_premultiplied(255, 193, 7, 50));
                        ui.painter().circle_stroke(center, 20.0, egui::Stroke::new(2.0, YELLOW));
                        ui.painter().text(center, egui::Align2::CENTER_CENTER, "⚠", egui::FontId::proportional(22.0), YELLOW);
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Mirror Still Running").size(16.0).strong().color(egui::Color32::WHITE));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("The phone mirror is still active.\nChoose what you'd like to do.").size(13.0).color(egui::Color32::WHITE));
                    });

                    // ── Buttons (centered, no separator) ──
                    ui.add_space(12.0);
                    let btn_width = dialog_width - 40.0;

                    ui.vertical_centered(|ui| {
                        ui.set_width(btn_width);
                        if colored_button_full(ui, "❌  Close Mirror & Quit", RED, RED_DIM, btn_width) {
                            self.close_mirror();
                            self.show_close_warning = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.add_space(4.0);
                        if colored_button_full(ui, "▶  Keep Running", GREEN, GREEN_DIM, btn_width) {
                            self.show_close_warning = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(4.0);
                        if colored_button_full(ui, "✕  Cancel", egui::Color32::from_rgb(180, 184, 200), egui::Color32::from_rgb(48, 50, 66), btn_width) {
                            self.show_close_warning = false;
                        }
                    });
                });
        }

        // ── Main Layout ──
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(14)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                    // ── Header ──
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("📱 PhoneMirror").size(22.0).strong().color(ACCENT));
                        ui.label(egui::RichText::new("v2.1.0 · Cross-platform").size(10.0).color(TEXT_DIM));
                    });
                    ui.add_space(10.0);

                    // ══ Device Status ══
                    show_card(ui, |ui| {
                        card_title(ui, "🔌", "Device Status");
                        ui.add_space(6.0);

                        if !self.device_status.checked {
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("⏳ Checking...").size(13.0).color(YELLOW));
                            });
                        } else if self.device_status.connected {
                            status_badge(ui, true, &self.device_status.device_id);
                            ui.add_space(4.0);
                            info_row(ui, "ADB", truncate_path(&self.device_status.adb_path, 35).as_str());
                            info_row(ui, "scrcpy", truncate_path(&self.device_status.scrcpy_path, 35).as_str());
                        } else {
                            status_badge(ui, false, "No device found");
                        }

                        if self.device_status.checked && !self.device_status.connected {
                            ui.add_space(4.0);
                            info_row(ui, "ADB", truncate_path(&self.device_status.adb_path, 35).as_str());
                            info_row(ui, "scrcpy", truncate_path(&self.device_status.scrcpy_path, 35).as_str());
                        }
                    });

                    ui.add_space(6.0);

                    // ══ Screen Mirror ══
                    show_card(ui, |ui| {
                        card_title(ui, "🖥️", "Screen Mirror");
                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            if self.mirror_running {
                                if colored_button(ui, "📵 Close Mirror", RED, RED_DIM) {
                                    self.close_mirror();
                                }
                            } else {
                                if colored_button(ui, "📱 Start Mirror", ACCENT, ACCENT_DIM) {
                                    self.start_mirror();
                                }
                            }
                            if colored_button(ui, "📸 Screenshot", GREEN, GREEN_DIM) {
                                self.take_screenshot();
                            }
                        });

                        if self.mirror_running {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("●").size(11.0).color(GREEN));
                                ui.label(egui::RichText::new("Mirror active — close window to minimize").size(11.0).color(TEXT_DIM));
                            });
                        }
                    });

                    ui.add_space(6.0);

                    // ══ Recording ══
                    show_card(ui, |ui| {
                        card_title(ui, "🎬", "Recording");
                        ui.add_space(6.0);

                        if self.is_recording {
                            ui.horizontal_wrapped(|ui| {
                                let alpha = 0.5 + 0.5 * (self.pulse_time * std::f32::consts::PI).sin();
                                ui.colored_label(
                                    egui::Color32::from_rgba_premultiplied(220, 50, 50, (alpha * 255.0) as u8),
                                    egui::RichText::new("⏺").size(14.0),
                                );
                                ui.label(egui::RichText::new("Recording...").size(12.0).color(YELLOW));
                                if colored_button(ui, "⏹ Stop", RED, RED_DIM) {
                                    self.stop_recording();
                                }
                            });
                        } else {
                            if colored_button(ui, "⏺ Start Recording", RECORD_RED, RED_DIM) {
                                self.start_recording();
                            }
                        }
                    });

                    ui.add_space(6.0);

                    // ══ Actions ══
                    show_card(ui, |ui| {
                        card_title(ui, "⚡", "Actions");
                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            if colored_button(ui, "🔄 Refresh", TEXT_DIM, CARD_BG) {
                                self.refresh_device();
                            }
                            if colored_button(ui, "📵 Force Close", RED, RED_DIM) {
                                self.close_mirror();
                            }
                        });
                    });

                    // ── Status Message ──
                    if !self.status_message.is_empty() {
                        ui.add_space(6.0);
                        let fill = if self.status_is_error {
                            egui::Color32::from_rgba_premultiplied(80, 20, 20, 200)
                        } else {
                            egui::Color32::from_rgba_premultiplied(20, 60, 40, 200)
                        };
                        let border = if self.status_is_error { RED } else { GREEN };
                        egui::Frame::new()
                            .fill(fill)
                            .stroke(egui::Stroke::new(1.0, border))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                let icon = if self.status_is_error { "✗" } else { "✓" };
                                let msg = if self.status_message.len() > 50 {
                                    format!("{} {}…", icon, &self.status_message[..47])
                                } else {
                                    format!("{} {}", icon, self.status_message)
                                };
                                ui.colored_label(border, egui::RichText::new(msg).size(12.0));
                            });
                    }

                    // ── Footer ──
                    ui.add_space(10.0);
                    ui.vertical_centered(|ui| {
                        if self.mirror_running {
                            ui.colored_label(GREEN, egui::RichText::new("📡 Mirror active — closing window minimizes app").size(9.0));
                        }
                        ui.add_space(2.0);
                        ui.hyperlink_to(
                            egui::RichText::new("github.com/minrahim1999/phonemirror").size(9.0).color(ACCENT),
                            "https://github.com/minrahim1999/phonemirror",
                        );
                        ui.label(egui::RichText::new("PhoneMirror v2.1.0 · MIT License").size(9.0).color(TEXT_DIM));
                    });
                });
            });

        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

// ─── UI Components ─────────────────────────────────────────

fn card_title(ui: &mut egui::Ui, emoji: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(emoji).size(14.0));
        ui.label(egui::RichText::new(title).size(13.0).strong().color(TEXT));
    });
}

fn status_badge(ui: &mut egui::Ui, connected: bool, detail: &str) {
    let (color, label) = if connected { (GREEN, "Connected") } else { (RED, "Disconnected") };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("●").size(12.0).color(color));
        ui.label(egui::RichText::new(label).size(13.0).strong().color(color));
        if connected && !detail.is_empty() {
            ui.label(egui::RichText::new(detail).size(11.0).color(TEXT_DIM));
        }
    });
}

fn colored_button(ui: &mut egui::Ui, text: &str, color: egui::Color32, dim: egui::Color32) -> bool {
    ui.add(egui::Button::new(egui::RichText::new(text).size(12.0).color(egui::Color32::WHITE))
        .fill(dim)
        .stroke(egui::Stroke::new(1.0, color))
        .corner_radius(6)
        .min_size(egui::Vec2::new(0.0, 28.0)))
    .clicked()
}

fn colored_button_full(ui: &mut egui::Ui, text: &str, color: egui::Color32, dim: egui::Color32, width: f32) -> bool {
    ui.horizontal_centered(|ui| {
        ui.add_sized([width, 36.0],
            egui::Button::new(egui::RichText::new(text).size(13.0).color(egui::Color32::WHITE))
                .fill(dim)
                .stroke(egui::Stroke::new(1.5, color))
                .corner_radius(8)
        )
    }).inner.clicked()
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.set_width(LABEL_W);
            ui.label(egui::RichText::new(format!("{}:", label)).size(11.0).color(TEXT_DIM));
        });
        ui.add(egui::Label::new(egui::RichText::new(value).size(11.0).color(TEXT).monospace()).wrap());
    });
}

// ─── App Actions ──────────────────────────────────────────

impl PhoneMirrorApp {
    fn refresh_device(&mut self) {
        let adb = adb_path();
        let env = build_full_env();
        let output = std::process::Command::new(&adb)
            .args(["devices"])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .output();

        let mut connected = false;
        let mut device_id = String::from("No device");
        let mut device_serial = String::new();

        if let Ok(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            // Collect all connected devices, preferring USB over TCP/IP
            let mut usb_serial = String::new();
            let mut tcpip_serial = String::new();
            for line in stdout.lines().skip(1) {
                let trimmed = line.trim();
                if trimmed.contains("device") && !trimmed.contains("daemon") && !trimmed.is_empty() {
                    let serial = trimmed.split_whitespace().next().unwrap_or("unknown").to_string();
                    connected = true;
                    if serial.contains("._adb-tls-connect._tcp") {
                        if tcpip_serial.is_empty() { tcpip_serial = serial.clone(); }
                    } else {
                        if usb_serial.is_empty() { usb_serial = serial.clone(); }
                    }
                }
            }
            // Prefer USB connection over TCP/IP (avoids scrcpy "Multiple ADB devices" error)
            if !usb_serial.is_empty() {
                device_serial = usb_serial.clone();
                device_id = usb_serial;
            } else if !tcpip_serial.is_empty() {
                device_serial = tcpip_serial.clone();
                device_id = tcpip_serial;
            }
        }

        self.device_status = DeviceStatus {
            connected,
            device_id,
            device_serial,
            adb_path: adb,
            scrcpy_path: scrcpy_path(),
            checked: true,
        };
        self.mirror_running = is_mirror_running();
        self.last_refresh = Instant::now();
    }

    fn start_mirror(&mut self) {
        let scrcpy = scrcpy_path();
        let env = build_full_env();
        let mut args: Vec<String> = vec![];
        // Add -s serial to disambiguate when both USB and TCP/IP are connected
        if !self.device_status.device_serial.is_empty() {
            args.push("-s".to_string());
            args.push(self.device_status.device_serial.clone());
        }
        if cfg!(target_os = "macos") {
            args.push("--shortcut-mod=lctrl".to_string());
        }

        match std::process::Command::new(&scrcpy)
            .args(&args)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .spawn()
        {
            Ok(mut child) => {
                // Give scrcpy a moment to start — if it exits immediately, it's an error
                std::thread::sleep(std::time::Duration::from_millis(500));
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process already exited — likely an error
                        self.mirror_running = false;
                        self.status_message = format!("Mirror failed (exited {})", status.code().unwrap_or(-1));
                        self.status_is_error = true;
                    }
                    Ok(None) | Err(_) => {
                        // Still running — success
                        self.mirror_running = true;
                        self.status_message = "Mirror started".to_string();
                        self.status_is_error = false;
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Failed: {}", e);
                self.status_is_error = true;
            }
        }
    }

    fn close_mirror(&mut self) {
        let env = build_full_env();
        if cfg!(target_os = "windows") {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/IM", "scrcpy.exe"])
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .spawn();
        } else {
            let _ = std::process::Command::new("pkill")
                .args(["-x", "scrcpy"])
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .spawn();
        }
        self.mirror_running = false;
        self.is_recording = false;
        self.status_message = "Mirror closed".to_string();
        self.status_is_error = false;
    }

    fn take_screenshot(&mut self) {
        let adb = adb_path();
        let env = build_full_env();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let dir = screenshots_dir();
        let filename = format!("{}/phone_screenshot_{}.png", dir, timestamp);
        let remote_path = "/sdcard/phone_screenshot.png";

        // Build args with -s serial if available
        let serial = self.device_status.device_serial.clone();
        let shell_args: Vec<String> = if !serial.is_empty() {
            vec!["-s".to_string(), serial.clone(), "shell".to_string(), "screencap".to_string(), "-p".to_string(), remote_path.to_string()]
        } else {
            vec!["shell".to_string(), "screencap".to_string(), "-p".to_string(), remote_path.to_string()]
        };
        let _ = std::process::Command::new(&adb)
            .args(&shell_args)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .status();

        let pull_args: Vec<String> = if !serial.is_empty() {
            vec!["-s".to_string(), serial.clone(), "pull".to_string(), remote_path.to_string(), filename.clone()]
        } else {
            vec!["pull".to_string(), remote_path.to_string(), filename.clone()]
        };
        let pull_result = std::process::Command::new(&adb)
            .args(&pull_args)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .status();

        let rm_args: Vec<String> = if !serial.is_empty() {
            vec!["-s".to_string(), serial, "shell".to_string(), "rm".to_string(), remote_path.to_string()]
        } else {
            vec!["shell".to_string(), "rm".to_string(), remote_path.to_string()]
        };
        let _ = std::process::Command::new(&adb)
            .args(&rm_args)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .status();

        match pull_result {
            Ok(s) if s.success() => {
                self.status_message = "Screenshot saved!".to_string();
                self.status_is_error = false;
            }
            _ => {
                self.status_message = "Screenshot failed — phone connected?".to_string();
                self.status_is_error = true;
            }
        }
    }

    fn start_recording(&mut self) {
        let scrcpy = scrcpy_path();
        let env = build_full_env();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let dir = recordings_dir();
        let filename = format!("{}/phone_recording_{}.mp4", dir, timestamp);

        let mut args: Vec<String> = vec![];
        // Add -s serial to disambiguate when both USB and TCP/IP are connected
        if !self.device_status.device_serial.is_empty() {
            args.push("-s".to_string());
            args.push(self.device_status.device_serial.clone());
        }
        args.extend(["--record".to_string(), filename.clone(), "--no-playback".to_string(), "--no-audio".to_string(), "--no-window".to_string()]);

        match std::process::Command::new(&scrcpy)
            .args(&args)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .spawn()
        {
            Ok(mut child) => {
                // Give scrcpy a moment to start — if it exits immediately, it's an error
                std::thread::sleep(std::time::Duration::from_millis(500));
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process already exited — recording failed
                        self.is_recording = false;
                        self.status_message = format!("Recording failed (exited {})", status.code().unwrap_or(-1));
                        self.status_is_error = true;
                    }
                    Ok(None) | Err(_) => {
                        // Still running — success
                        self.is_recording = true;
                        self.status_message = format!("Recording started → {}", filename);
                        self.status_is_error = false;
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Failed: {}", e);
                self.status_is_error = true;
            }
        }
    }

    fn stop_recording(&mut self) {
        let env = build_full_env();
        if cfg!(target_os = "windows") {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/IM", "scrcpy.exe"])
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .spawn();
        } else {
            let _ = std::process::Command::new("pkill")
                .args(["-f", "scrcpy.*--record"])
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .spawn();
        }
        self.is_recording = false;
        self.status_message = "Recording stopped".to_string();
        self.status_is_error = false;
    }
}

// ─── Platform Paths ────────────────────────────────────────

fn adb_path() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());

    if cfg!(target_os = "windows") {
        let path = format!("{}\\AppData\\Local\\Android\\Sdk\\platform-tools\\adb.exe", home);
        if std::path::PathBuf::from(&path).exists() { return path; }
        "adb.exe".to_string()
    } else if cfg!(target_os = "macos") {
        // Order: Homebrew cask → Homebrew SDK → Android SDK → fallback
        let candidates = [
            "/opt/homebrew/bin/adb".to_string(),
            format!("{}/Library/Android/sdk/platform-tools/adb", home),
            format!("{}/Android/Sdk/platform-tools/adb", home),
            "/usr/local/bin/adb".to_string(),
        ];
        for candidate in &candidates {
            if std::path::PathBuf::from(candidate).exists() { return candidate.clone(); }
        }
        "adb".to_string()
    } else {
        for candidate in &[
            "/usr/bin/adb",
            "/usr/local/bin/adb",
            &format!("{}/Android/Sdk/platform-tools/adb", home)[..],
        ] {
            if std::path::PathBuf::from(candidate).exists() { return candidate.to_string(); }
        }
        "adb".to_string()
    }
}

fn scrcpy_path() -> String {
    if cfg!(target_os = "windows") {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        let candidates = [
            format!("{}\\scoop\\shims\\scrcpy.exe", home),
            "C:\\Program Files\\scrcpy\\scrcpy.exe".to_string(),
            format!("{}\\AppData\\Local\\Microsoft\\WinGet\\Links\\scrcpy.exe", home),
        ];
        for candidate in &candidates {
            if std::path::PathBuf::from(candidate).exists() { return candidate.clone(); }
        }
        "scrcpy.exe".to_string()
    } else if cfg!(target_os = "macos") {
        let path = "/opt/homebrew/bin/scrcpy".to_string();
        if std::path::PathBuf::from(&path).exists() { return path; }
        "scrcpy".to_string()
    } else {
        for candidate in &["/usr/bin/scrcpy", "/usr/local/bin/scrcpy", "/snap/bin/scrcpy"] {
            if std::path::PathBuf::from(candidate).exists() { return candidate.to_string(); }
        }
        "scrcpy".to_string()
    }
}

fn screenshots_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());
    if cfg!(target_os = "macos") {
        let path = format!("{}/Pictures/PhoneMirror", home);
        let _ = std::fs::create_dir_all(&path);
        path
    } else if cfg!(target_os = "windows") {
        let path = format!("{}\\Pictures\\PhoneMirror", home);
        let _ = std::fs::create_dir_all(&path);
        path
    } else {
        let path = format!("{}/Pictures/PhoneMirror", home);
        let _ = std::fs::create_dir_all(&path);
        path
    }
}

fn recordings_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());
    if cfg!(target_os = "macos") {
        let path = format!("{}/Movies/PhoneMirror", home);
        let _ = std::fs::create_dir_all(&path);
        path
    } else if cfg!(target_os = "windows") {
        let path = format!("{}\\Videos\\PhoneMirror", home);
        let _ = std::fs::create_dir_all(&path);
        path
    } else {
        let path = format!("{}/Videos/PhoneMirror", home);
        let _ = std::fs::create_dir_all(&path);
        path
    }
}

fn build_full_env() -> Vec<(String, String)> {
    let essential_paths = if cfg!(target_os = "macos") {
        vec![
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]
    } else if cfg!(target_os = "windows") {
        vec![]
    } else {
        vec![
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
            "/snap/bin",
        ]
    };

    let mut env_pairs: Vec<(String, String)> = std::env::vars().collect();

    if !essential_paths.is_empty() {
        if let Some(pos) = env_pairs.iter().position(|(k, _)| k == "PATH") {
            let existing = &env_pairs[pos].1;
            let mut path_parts: Vec<String> = essential_paths.iter().map(|s| s.to_string()).collect();
            for part in existing.split(':') {
                let p = part.to_string();
                if !path_parts.contains(&p) {
                    path_parts.push(p);
                }
            }
            env_pairs[pos].1 = path_parts.join(":");
        } else {
            env_pairs.push(("PATH".to_string(), essential_paths.join(":")));
        }
    }

    // Set SCRCPY_SERVER_PATH so scrcpy can find its server jar
    // even when launched from Finder with a minimal PATH
    if !env_pairs.iter().any(|(k, _)| k == "SCRCPY_SERVER_PATH") {
        if cfg!(target_os = "macos") {
            // Auto-discover: scan Cellar for any scrcpy version, then fallback paths
            let cellar_dir = std::path::Path::new("/opt/homebrew/Cellar/scrcpy");
            let mut found = None;
            if let Ok(entries) = std::fs::read_dir(cellar_dir) {
                for entry in entries.flatten() {
                    let server = entry.path().join("share/scrcpy/scrcpy-server");
                    if server.exists() {
                        found = Some(server.to_string_lossy().to_string());
                        break;
                    }
                }
            }
            if let Some(path) = found {
                env_pairs.push(("SCRCPY_SERVER_PATH".to_string(), path));
            } else {
                let fallbacks = [
                    "/opt/homebrew/share/scrcpy/scrcpy-server",
                    "/usr/local/share/scrcpy/scrcpy-server",
                    "/usr/share/scrcpy/scrcpy-server",
                ];
                for candidate in &fallbacks {
                    if std::path::PathBuf::from(candidate).exists() {
                        env_pairs.push(("SCRCPY_SERVER_PATH".to_string(), candidate.to_string()));
                        break;
                    }
                }
            }
        }
    }

    // Set ADB_SERVER_PATH so scrcpy can locate the adb server binary
    if !env_pairs.iter().any(|(k, _)| k == "ADB_SERVER_PATH") {
        let adb = adb_path();
        if !adb.is_empty() {
            env_pairs.push(("ADB_SERVER_PATH".to_string(), adb));
        }
    }

    env_pairs
}

fn is_mirror_running() -> bool {
    let env = build_full_env();
    if cfg!(target_os = "windows") {
        let output = std::process::Command::new("tasklist")
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .output();
        if let Ok(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            return stdout.to_lowercase().contains("scrcpy");
        }
        false
    } else {
        let output = std::process::Command::new("pgrep")
            .args(["-x", "scrcpy"])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .output();
        if let Ok(o) = output {
            !String::from_utf8_lossy(&o.stdout).trim().is_empty()
        } else {
            false
        }
    }
}