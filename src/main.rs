#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

fn main() -> eframe::Result {
    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PhoneMirror")
            .with_inner_size([420.0, 640.0])
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

const BG: egui::Color32 = egui::Color32::from_rgb(22, 22, 30);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(38, 40, 56);
const BORDER: egui::Color32 = egui::Color32::from_rgb(60, 64, 80);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(88, 166, 255);
const GREEN: egui::Color32 = egui::Color32::from_rgb(76, 196, 136);
const RED: egui::Color32 = egui::Color32::from_rgb(235, 87, 87);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 193, 7);
const RECORD_RED: egui::Color32 = egui::Color32::from_rgb(220, 50, 50);
const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 222, 230);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(140, 144, 160);

fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(12))
}

// ─── Truncate long paths for display ─────────────────────

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else if let Some(slash_pos) = path.rfind('/') {
        let filename = &path[slash_pos + 1..];
        if filename.len() >= max_len {
            format!("…{}", &filename[..max_len - 1])
        } else {
            // Show "…/filename" pattern
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

        // Set visual style
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = BG;
        style.visuals.extreme_bg_color = BG;
        style.visuals.window_fill = BG;
        style.visuals.widgets.inactive.bg_fill = CARD_BG;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 54, 72);
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        style.visuals.widgets.active.bg_fill = ACCENT;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        ctx.set_style(style);

        // Intercept close request — if mirror is running, show dialog instead of quitting
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            if self.mirror_running || self.is_recording {
                self.show_close_warning = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // Close warning popup
        if self.show_close_warning {
            egui::Window::new("Mirror Still Running")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(300.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("⚠️ Mirror is still running!").size(16.0).color(YELLOW));
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("The phone mirror is still active.\nWhat would you like to do?").size(13.0).color(TEXT));
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            if ui.button(egui::RichText::new("❌ Close Mirror & Quit").size(13.0).color(RED)).clicked() {
                                self.close_mirror();
                                self.show_close_warning = false;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button(egui::RichText::new("▶️ Keep Running (Minimize)").size(13.0).color(GREEN)).clicked() {
                                self.show_close_warning = false;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                            if ui.button(egui::RichText::new("Cancel").size(13.0)).clicked() {
                                self.show_close_warning = false;
                            }
                        });
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                // Scrollable area so content never clips off-screen
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                    // Header
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("📱 PhoneMirror").size(20.0).strong().color(ACCENT));
                        ui.label(egui::RichText::new("v2.0.0 · Cross-platform").size(10.0).color(TEXT_DIM));
                    });
                    ui.add_space(10.0);

                    // ── Device Status Card ──
                    card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔌").size(15.0));
                            ui.label(egui::RichText::new("Device Status").size(14.0).strong().color(TEXT));
                        });
                        ui.add_space(4.0);

                        if !self.device_status.checked {
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("⏳ Checking...").size(13.0).color(YELLOW));
                            });
                        } else if self.device_status.connected {
                            ui.colored_label(GREEN, egui::RichText::new("● Connected").size(13.0).strong());
                            ui.add_space(2.0);
                            info_row(ui, "Device", &self.device_status.device_id);
                        } else {
                            ui.colored_label(RED, egui::RichText::new("● Disconnected").size(13.0).strong());
                        }

                        if self.device_status.checked {
                            ui.add_space(2.0);
                            // Wrap long paths using available width
                            let available = ui.available_width();
                            info_row_wrapped(ui, "ADB", truncate_path(&self.device_status.adb_path, 40).as_str(), available);
                            info_row_wrapped(ui, "scrcpy", truncate_path(&self.device_status.scrcpy_path, 40).as_str(), available);
                        }
                    });

                    ui.add_space(6.0);

                    // ── Screen Mirror Card ──
                    card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🖥️").size(15.0));
                            ui.label(egui::RichText::new("Screen Mirror").size(14.0).strong().color(TEXT));
                        });
                        ui.add_space(4.0);

                        // Use wrapped layout for buttons
                        ui.horizontal_wrapped(|ui| {
                            if self.mirror_running {
                                if action_button(ui, "📵 Close", RED) {
                                    self.close_mirror();
                                }
                            } else {
                                if action_button(ui, "📱 Start Mirror", ACCENT) {
                                    self.start_mirror();
                                }
                            }
                            if action_button(ui, "📸 Screenshot", GREEN) {
                                self.take_screenshot();
                            }
                        });

                        if self.mirror_running {
                            ui.add_space(2.0);
                            ui.colored_label(GREEN, egui::RichText::new("● Mirror active — close window to minimize").size(11.0));
                        }
                    });

                    ui.add_space(6.0);

                    // ── Recording Card ──
                    card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🎬").size(15.0));
                            ui.label(egui::RichText::new("Recording").size(14.0).strong().color(TEXT));
                        });
                        ui.add_space(4.0);

                        if self.is_recording {
                            ui.horizontal_wrapped(|ui| {
                                let alpha = 0.5 + 0.5 * (self.pulse_time * std::f32::consts::PI).sin();
                                ui.colored_label(
                                    egui::Color32::from_rgba_premultiplied(220, 50, 50, (alpha * 255.0) as u8),
                                    egui::RichText::new("⏺").size(15.0),
                                );
                                ui.label(egui::RichText::new("Recording...").size(12.0).color(YELLOW));
                                if action_button(ui, "⏹ Stop", RED) {
                                    self.stop_recording();
                                }
                            });
                        } else {
                            if action_button(ui, "⏺ Start Recording", RECORD_RED) {
                                self.start_recording();
                            }
                        }
                    });

                    ui.add_space(6.0);

                    // ── Actions Card ──
                    card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚡").size(15.0));
                            ui.label(egui::RichText::new("Actions").size(14.0).strong().color(TEXT));
                        });
                        ui.add_space(4.0);

                        ui.horizontal_wrapped(|ui| {
                            if action_button(ui, "🔄 Refresh", TEXT_DIM) {
                                self.refresh_device();
                            }
                            if action_button(ui, "📵 Force Close", RED) {
                                self.close_mirror();
                            }
                        });
                    });

                    // Status message
                    if !self.status_message.is_empty() {
                        ui.add_space(6.0);
                        let fill = if self.status_is_error {
                            egui::Color32::from_rgba_premultiplied(80, 20, 20, 180)
                        } else {
                            egui::Color32::from_rgba_premultiplied(20, 60, 40, 180)
                        };
                        egui::Frame::new()
                            .fill(fill)
                            .corner_radius(6)
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                let icon = if self.status_is_error { "✗" } else { "✓" };
                                let color = if self.status_is_error { RED } else { GREEN };
                                // Truncate status if too long
                                let msg = if self.status_message.len() > 60 {
                                    format!("{}…", &self.status_message[..57])
                                } else {
                                    self.status_message.clone()
                                };
                                ui.colored_label(color, egui::RichText::new(format!("{} {}", icon, msg)).size(12.0));
                            });
                    }

                    // Footer
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if self.mirror_running {
                            ui.colored_label(GREEN, egui::RichText::new("📡 Mirror active — closing window minimizes app").size(9.0));
                        }
                        ui.hyperlink_to(
                            egui::RichText::new("github.com/minrahim1999/phonemirror").size(9.0).color(ACCENT),
                            "https://github.com/minrahim1999/phonemirror",
                        );
                        ui.label(egui::RichText::new("PhoneMirror v2.0.0 · Open Source · MIT License").size(9.0).color(TEXT_DIM));
                    });
                });
            });

        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

// ─── UI Helpers ───────────────────────────────────────────

fn action_button(ui: &mut egui::Ui, text: &str, color: egui::Color32) -> bool {
    let fill = egui::Color32::from_rgb(color.r() / 3, color.g() / 3, color.b() / 3);
    ui.add(egui::Button::new(egui::RichText::new(text).size(12.0).color(egui::Color32::WHITE))
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, color))
        .corner_radius(6))
    .clicked()
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}:", label)).size(11.0).color(TEXT_DIM));
        ui.label(egui::RichText::new(value).size(11.0).color(TEXT).monospace());
    });
}

fn info_row_wrapped(ui: &mut egui::Ui, label: &str, value: &str, _available_width: f32) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}:", label)).size(11.0).color(TEXT_DIM));
        // Use wrapping label for long paths
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

        if let Ok(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            for line in stdout.lines().skip(1) {
                let trimmed = line.trim();
                if trimmed.contains("device") && !trimmed.contains("daemon") && !trimmed.is_empty() {
                    device_id = trimmed.split_whitespace().next().unwrap_or("unknown").to_string();
                    connected = true;
                    break;
                }
            }
        }

        self.device_status = DeviceStatus {
            connected,
            device_id,
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
        let args: Vec<String> = if cfg!(target_os = "macos") {
            vec!["--shortcut-mod=lctrl".to_string()]
        } else {
            vec![]
        };
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        match std::process::Command::new(&scrcpy)
            .args(&args_refs)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .spawn()
        {
            Ok(_) => {
                self.mirror_running = true;
                self.status_message = "Mirror started".to_string();
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = format!("Failed to start mirror: {}", e);
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

        let _ = std::process::Command::new(&adb)
            .args(["shell", "screencap", "-p", remote_path])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .status();

        let pull_result = std::process::Command::new(&adb)
            .args(["pull", remote_path, &filename])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .status();

        let _ = std::process::Command::new(&adb)
            .args(["shell", "rm", remote_path])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .status();

        match pull_result {
            Ok(s) if s.success() => {
                self.status_message = "Screenshot saved!".to_string();
                self.status_is_error = false;
            }
            _ => {
                self.status_message = "Screenshot failed — is your phone connected?".to_string();
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

        match std::process::Command::new(&scrcpy)
            .args(["--record", &filename, "--no-playback", "--no-audio", "--no-window"])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .spawn()
        {
            Ok(_) => {
                self.is_recording = true;
                self.status_message = "Recording started".to_string();
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = format!("Failed to start recording: {}", e);
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
        let homebrew_sdk = format!("{}/Library/Android/sdk/platform-tools/adb", home);
        if std::path::PathBuf::from(&homebrew_sdk).exists() { return homebrew_sdk; }
        let homebrew = "/opt/homebrew/bin/adb".to_string();
        if std::path::PathBuf::from(&homebrew).exists() { return homebrew; }
        "adb".to_string()
    } else {
        for candidate in &["/usr/bin/adb", "/usr/local/bin/adb"] {
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
        format!("{}/Pictures", home)
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
        format!("{}/Movies", home)
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