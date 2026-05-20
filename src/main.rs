#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PhoneMirror")
            .with_inner_size([400.0, 550.0])
            .with_min_inner_size([320.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PhoneMirror",
        options,
        Box::new(|_cc| Ok(Box::new(PhoneMirrorApp::default()))),
    )
}

// ─── PhoneMirror App ──────────────────────────────────────

struct PhoneMirrorApp {
    device_status: DeviceStatus,
    mirror_running: bool,
    is_recording: bool,
    status_message: String,
    status_is_error: bool,
    last_refresh: std::time::Instant,
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
            last_refresh: std::time::Instant::now(),
        }
    }
}

impl eframe::App for PhoneMirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-refresh device status every 5 seconds
        if self.last_refresh.elapsed() > std::time::Duration::from_secs(5) {
            self.refresh_device();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Header
                ui.add_space(8.0);
                ui.heading("📱 PhoneMirror");
                ui.label(egui::RichText::new("v2.0.0").small().weak());
                ui.add_space(12.0);
            });

            // Device Status Card
            egui::Frame::group(ui.style())
                .fill(ui.visuals().widgets.inactive.bg_fill)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔌");
                        ui.heading(egui::RichText::new("Device Status").size(16.0));
                    });
                    ui.add_space(4.0);

                    if !self.device_status.checked {
                        ui.label("⏳ Checking...");
                    } else if self.device_status.connected {
                        ui.colored_label(egui::Color32::GREEN, "● Connected");
                    } else {
                        ui.colored_label(egui::Color32::RED, "● Disconnected");
                    };

                    if self.device_status.checked {
                        ui.horizontal(|ui| {
                            ui.label("Device ID:");
                            ui.label(&self.device_status.device_id);
                        });
                        ui.horizontal(|ui| {
                            ui.label("ADB:");
                            ui.label(egui::RichText::new(&self.device_status.adb_path).small().weak());
                        });
                        ui.horizontal(|ui| {
                            ui.label("scrcpy:");
                            ui.label(egui::RichText::new(&self.device_status.scrcpy_path).small().weak());
                        });
                    }
                });

            ui.add_space(8.0);

            // Mirror Control Card
            egui::Frame::group(ui.style())
                .fill(ui.visuals().widgets.inactive.bg_fill)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🖥️");
                        ui.heading(egui::RichText::new("Screen Mirror").size(16.0));
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if self.mirror_running {
                            if ui.button("📵 Close Mirror").clicked() {
                                self.close_mirror();
                            }
                        } else {
                            if ui.button("📱 Start Mirror").clicked() {
                                self.start_mirror();
                            }
                        }
                        if ui.button("📸 Screenshot").clicked() {
                            self.take_screenshot();
                        }
                    });
                });

            ui.add_space(8.0);

            // Recording Card
            egui::Frame::group(ui.style())
                .fill(ui.visuals().widgets.inactive.bg_fill)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🎬");
                        ui.heading(egui::RichText::new("Recording").size(16.0));
                    });
                    ui.add_space(4.0);

                    if self.is_recording {
                        if ui.button("⏹ Stop Recording").clicked() {
                            self.stop_recording();
                        }
                    } else {
                        if ui.button("⏺ Start Recording").clicked() {
                            self.start_recording();
                        }
                    }
                });

            ui.add_space(8.0);

            // Actions
            egui::Frame::group(ui.style())
                .fill(ui.visuals().widgets.inactive.bg_fill)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("⚡");
                        ui.heading(egui::RichText::new("Actions").size(16.0));
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            self.refresh_device();
                        }
                        if ui.button("📵 Force Close Mirror").clicked() {
                            self.close_mirror();
                        }
                    });
                });

            // Status message
            if !self.status_message.is_empty() {
                ui.add_space(8.0);
                if self.status_is_error {
                    ui.colored_label(egui::Color32::RED, &self.status_message);
                } else {
                    ui.colored_label(egui::Color32::GREEN, &self.status_message);
                }
            }

            // Footer
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("PhoneMirror v2.0.0 · Cross-platform")
                        .small()
                        .weak(),
                );
            });
        });

        // Keep UI refreshing
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

// ─── Helper functions ─────────────────────────────────────

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
        self.last_refresh = std::time::Instant::now();
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
                self.status_message = format!("Screenshot saved: {}", filename);
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
            .args([
                "--record", &filename,
                "--no-playback",
                "--no-audio",
                "--no-window",
            ])
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .spawn()
        {
            Ok(_) => {
                self.is_recording = true;
                self.status_message = format!("Recording to: {}", filename);
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

// ─── Platform-specific paths ──────────────────────────────

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
        for candidate in &[
            format!("{}\\scoop\\shims\\scrcpy.exe", home),
            "C:\\Program Files\\scrcpy\\scrcpy.exe".to_string(),
            format!("{}\\AppData\\Local\\Microsoft\\WinGet\\Links\\scrcpy.exe", home),
        ] {
            if std::path::PathBuf::from(candidate).exists() { return candidate.to_string(); }
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