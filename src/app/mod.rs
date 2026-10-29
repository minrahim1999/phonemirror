use std::time::{Duration, Instant};

mod android;
mod platform;
mod ui;

use android::{devices, mirror, wireless};
use platform::tools;
#[cfg(target_os = "macos")]
use platform::app_menu;
use ui::{desktop_ui, mirror_ui, split_ui, wireless_ui};

pub(crate) fn run() -> eframe::Result {
    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PhoneMirror")
            .with_inner_size([1060.0, 820.0])
            .with_min_inner_size([720.0, 620.0])
            .with_icon(icon)
            .with_fullsize_content_view(cfg!(target_os = "macos"))
            .with_title_shown(!cfg!(target_os = "macos"))
            .with_titlebar_shown(!cfg!(target_os = "macos")),
        ..Default::default()
    };
    eframe::run_native(
        "PhoneMirror",
        options,
        Box::new(|cc| {
            ui::theme::configure_theme(&cc.egui_ctx);
            Ok(Box::new(PhoneMirrorApp::default()))
        }),
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

const BG: egui::Color32 = egui::Color32::from_rgb(29, 29, 31);
const SIDEBAR: egui::Color32 = egui::Color32::from_rgb(39, 39, 41);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(46, 46, 49);
const SURFACE_HOVER: egui::Color32 = egui::Color32::from_rgb(60, 60, 64);
const SELECTED: egui::Color32 = egui::Color32::from_rgb(48, 70, 110);
const BORDER: egui::Color32 = egui::Color32::from_rgb(55, 55, 58);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(10, 112, 255);
const ACCENT_SOFT: egui::Color32 = egui::Color32::from_rgb(38, 52, 86);
const ACCENT_TEXT: egui::Color32 = egui::Color32::from_rgb(150, 184, 255);
const INFO_FILL: egui::Color32 = egui::Color32::from_rgb(30, 39, 60);
const ERROR_FILL: egui::Color32 = egui::Color32::from_rgb(56, 32, 35);
const GREEN: egui::Color32 = egui::Color32::from_rgb(95, 211, 154);
const RED: egui::Color32 = egui::Color32::from_rgb(242, 133, 123);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(230, 188, 106);
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 232, 234);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(151, 151, 157);

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
    wireless: wireless_ui::WirelessUi,
    refresh_job: Option<std::sync::mpsc::Receiver<(DeviceStatus, devices::Inventory)>>,
    embedded: mirror_ui::MirrorUi,
    split: split_ui::SplitUi,
    drawer: ui::tools_ui::ToolsDrawer,
    recording_child: Option<std::process::Child>,
    recording_cleanup: Vec<std::thread::JoinHandle<()>>,
    recording_log: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    inventory: devices::Inventory,
    selected_device: Option<String>,
    page: desktop_ui::Page,
    device_filter: String,
    sidebar_open: bool,
    hide_unavailable: bool,
    show_device_info: bool,
    launch_job: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    starting_emulator: Option<(String, Instant)>,
    /// Re-probe tools on the next refresh (startup, manual refresh, settings change).
    tools_dirty: bool,
    tool_inputs: Option<[String; 3]>,
    browse_job: Option<(tools::Tool, std::sync::mpsc::Receiver<Option<String>>)>,
    #[cfg(target_os = "macos")]
    menu: Option<app_menu::AppMenu>,
}

#[derive(Clone)]
struct DeviceStatus {
    connected: bool,
    device_id: String,
    /// The explicitly selected online serial to pass to scrcpy/adb with -s.
    device_serial: String,
    /// Latest tool discovery; None until the first scan finishes.
    tools: Option<tools::Report>,
    /// Empty when scrcpy launches cleanly; otherwise an actionable reason
    /// (e.g. broken Homebrew linkage after an ffmpeg upgrade)
    scrcpy_health: String,
    checked: bool,
    adb_health: String,
}

impl Default for PhoneMirrorApp {
    fn default() -> Self {
        Self {
            device_status: DeviceStatus {
                connected: false,
                device_id: String::new(),
                device_serial: String::new(),
                tools: None,
                scrcpy_health: String::new(),
                checked: false,
                adb_health: String::new(),
            },
            mirror_running: false,
            is_recording: false,
            status_message: String::new(),
            status_is_error: false,
            last_refresh: Instant::now() - Duration::from_secs(6),
            pulse_time: 0.0,
            show_close_warning: false,
            wireless: wireless_ui::WirelessUi::default(),
            refresh_job: None,
            embedded: mirror_ui::MirrorUi::default(),
            split: split_ui::SplitUi::default(),
            drawer: ui::tools_ui::ToolsDrawer::default(),
            recording_child: None,
            recording_cleanup: Vec::new(),
            recording_log: Default::default(),
            inventory: devices::Inventory::default(),
            selected_device: None,
            page: desktop_ui::Page::Devices,
            device_filter: String::new(),
            sidebar_open: true,
            hide_unavailable: false,
            show_device_info: false,
            launch_job: None,
            starting_emulator: None,
            tools_dirty: true,
            tool_inputs: None,
            browse_job: None,
            #[cfg(target_os = "macos")]
            menu: None,
        }
    }
}

impl eframe::App for PhoneMirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_refresh();
        self.poll_emulator_launch();
        self.poll_browse();
        self.embedded.poll(ctx);
        self.split.reconcile_primary(&self.device_status.device_serial);
        self.split.poll(ctx, self.page == desktop_ui::Page::Devices);
        self.mirror_running = self.embedded.active() || self.split.active();
        if self.page != desktop_ui::Page::Devices { self.embedded.release_touch(); }
        if let Some(child) = self.recording_child.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                self.recording_child = None; self.is_recording = false;
                self.status_message = format!("Recording ended ({status}). {}", String::from_utf8_lossy(&self.recording_log.lock().unwrap()));
                self.status_is_error = !status.success();
            }
        }
        let wireless_finished = self.wireless.poll();
        if !self.wireless.busy() && (wireless_finished || self.last_refresh.elapsed() > Duration::from_secs(5)) {
            self.refresh_device();
        }

        self.pulse_time = (self.pulse_time + ctx.input(|i| i.stable_dt.min(0.1))) % 4.0;

        #[cfg(target_os = "macos")]
        self.update_menu(ctx);

        // ── Close interception ──
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && (self.mirror_running || self.is_recording) {
            self.show_close_warning = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // ── Close warning dialog ──
        if self.show_close_warning {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("close_overlay")))
                .rect_filled(ctx.screen_rect(), 0.0, egui::Color32::from_black_alpha(90));
            egui::Window::new("Active sessions")
                .collapsible(false).resizable(false).title_bar(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .frame(egui::Frame::new().fill(CARD_BG).stroke(egui::Stroke::new(1.0, BORDER)).corner_radius(14).inner_margin(24))
                .show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui.label(egui::RichText::new("You still have active sessions").size(20.0).strong());
                    ui::components::muted(ui, "Keep PhoneMirror running in the background, or stop its embedded screen and recording before quitting. Other apps are not affected.");
                    ui.add_space(12.0);
                    if ui.add_sized([ui.available_width(), 42.0], egui::Button::new(egui::RichText::new("Close sessions & quit").color(egui::Color32::WHITE)).fill(RED).corner_radius(8)).clicked() {
                        self.close_mirror();
                        self.show_close_warning = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.add_sized([ui.available_width(), 40.0], egui::Button::new("Keep running in background")).clicked() {
                        self.show_close_warning = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    if ui.add_sized([ui.available_width(), 36.0], egui::Button::new("Cancel").frame(false)).clicked() {
                        self.show_close_warning = false;
                    }
                });
        }

        self.show_workspace(ctx);

        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

// ─── App Actions ──────────────────────────────────────────

#[cfg(target_os = "macos")]
impl PhoneMirrorApp {
    /// Installs the native menu bar on the first frame, applies menu actions with the
    /// same guards as the toolbar, then mirrors the current state into the menu.
    fn update_menu(&mut self, ctx: &egui::Context) {
        use app_menu::Action;
        if self.menu.is_none() {
            match app_menu::install(ctx) {
                Ok(menu) => self.menu = Some(menu),
                Err(error) => {
                    eprintln!("PhoneMirror: menu bar unavailable: {error}");
                    return;
                }
            }
        }
        let actions = self.menu.as_mut().map(|m| m.actions()).unwrap_or_default();
        let ready = self.device_status.connected && self.device_status.scrcpy_health.is_empty();
        let streaming = self.embedded.streaming();
        for action in actions {
            match action {
                Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Action::Refresh => self.rescan_tools(),
                Action::PairWifi => {
                    self.embedded.release_touch();
                    self.page = desktop_ui::Page::Wifi;
                }
                Action::Screenshot if self.device_status.connected => self.take_screenshot(),
                Action::Record if self.is_recording => self.stop_recording(),
                Action::Record if ready => self.start_recording(),
                Action::Mirror if self.embedded.active() => {
                    self.embedded.stop();
                    self.mirror_running = self.split.active();
                }
                Action::Mirror if ready => {
                    self.page = desktop_ui::Page::Devices;
                    self.start_mirror(ctx);
                }
                Action::Fit if streaming => self.embedded.fit(),
                Action::ZoomIn if streaming => self.embedded.zoom_by(1.25),
                Action::ZoomOut if streaming => self.embedded.zoom_by(0.8),
                Action::Keyboard if streaming => self.embedded.toggle_keyboard(),
                Action::Nav(code) if streaming => self.embedded.navigation_key(code),
                Action::DeviceInfo => self.show_device_info = !self.show_device_info,
                Action::Help => ctx.open_url(egui::OpenUrl::new_tab(app_menu::HELP_URL)),
                // Unavailable right now: the menu item is disabled, ignore stale clicks.
                _ => {}
            }
        }
        let state = app_menu::MenuState {
            can_mirror: ready || self.embedded.active(),
            mirroring: self.embedded.active(),
            can_screenshot: self.device_status.connected,
            can_record: ready || self.is_recording,
            recording: self.is_recording,
            streaming: self.embedded.streaming(),
            keyboard: self.embedded.keyboard_enabled(),
            info: self.show_device_info,
            can_refresh: self.refresh_job.is_none(),
        };
        if let Some(menu) = self.menu.as_mut() {
            menu.sync(state);
        }
    }
}

impl PhoneMirrorApp {
    fn sync_selection(&mut self) {
        // Never silently redirect actions when the explicitly selected phone disappears.
        if self.selected_device.is_none() {
            self.selected_device = self.inventory.devices.iter()
                .find(|d| d.state == devices::State::Online)
                .or_else(|| self.inventory.devices.first()).map(|d| d.id.clone());
        }
        let selected = self.inventory.devices.iter()
            .find(|d| Some(&d.id) == self.selected_device.as_ref());
        self.device_status.connected = selected.is_some_and(|d| d.state == devices::State::Online && d.serial.is_some());
        self.device_status.device_serial = selected.filter(|_| self.device_status.connected)
            .and_then(|d| d.serial.clone()).unwrap_or_default();
        self.device_status.device_id = selected.map(|d| d.name.clone()).unwrap_or_default();
        self.embedded.reconcile_target(&self.device_status.device_serial);
        self.split.reconcile_primary(&self.device_status.device_serial);
        self.mirror_running = self.embedded.active() || self.split.active();
    }

    fn poll_refresh(&mut self) {
        let result = self.refresh_job.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(Ok(result)),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err(())),
        });
        match result {
            Some(Ok((status, inventory))) => {
                self.device_status = status;
                self.inventory = inventory;
                self.refresh_job = None;
                self.sync_selection();
                if self.tools_dirty { self.refresh_device(); }
            }
            Some(Err(())) => {
                self.refresh_job = None;
                self.tools_dirty = true;
                self.device_status.adb_health = "Device check stopped unexpectedly. Refresh to retry.".into();
                self.device_status.connected = false;
                self.device_status.device_serial.clear();
                self.embedded.stop();
                self.mirror_running = self.split.active();
            }
            None => {}
        }
    }

    fn refresh_device(&mut self) {
        if self.refresh_job.is_some() { return; }
        let rescan = std::mem::take(&mut self.tools_dirty);
        let (tx, rx) = std::sync::mpsc::channel();
        self.refresh_job = Some(rx);
        self.last_refresh = Instant::now();
        std::thread::spawn(move || {
            let tools = match tools::cached() {
                Some(report) if !rescan => report,
                _ => tools::rescan(&build_full_env()),
            };
            let adb_path = adb_path();
            let mut adb = wireless::LocalAdb {
                path: adb_path.clone(), env: build_full_env(),
                cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            };
            let (inventory, adb_health) = match devices::discover(&mut adb) {
                Ok(inventory) => { let health = inventory.adb_health.clone(); (inventory, health) },
                Err(error) => (devices::Inventory::default(), error),
            };
            let status = DeviceStatus {
                connected: false, device_id: String::new(), device_serial: String::new(),
                tools: Some(tools), adb_health, scrcpy_health: scrcpy_health(), checked: true,
            };
            let _ = tx.send((status, inventory));
        });
    }

    /// Manual refresh: also re-detects ADB, scrcpy and FFmpeg.
    fn rescan_tools(&mut self) {
        self.tools_dirty = true;
        self.refresh_device();
    }

    fn save_tool_path(&mut self, tool: tools::Tool, path: Option<String>) {
        let mut settings = tools::Settings::load();
        settings.set(tool, path);
        if let Some(inputs) = self.tool_inputs.as_mut() {
            inputs[tool.index()] = settings.custom(tool).unwrap_or_default().to_string();
        }
        if let Err(error) = settings.save() {
            self.status_message = format!("Couldn't save the {} path: {error}", tool.label());
            self.status_is_error = true;
            return;
        }
        self.rescan_tools();
    }

    fn browse_tool(&mut self, tool: tools::Tool) {
        if self.browse_job.is_some() { return; }
        let (tx, rx) = std::sync::mpsc::channel();
        self.browse_job = Some((tool, rx));
        std::thread::spawn(move || { let _ = tx.send(tools::choose_file(tool)); });
    }

    fn poll_browse(&mut self) {
        let picked = self.browse_job.as_ref().and_then(|(tool, rx)| match rx.try_recv() {
            Ok(choice) => Some((*tool, choice)),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Some((*tool, None)),
        });
        if let Some((tool, choice)) = picked {
            self.browse_job = None;
            if let Some(path) = choice { self.save_tool_path(tool, Some(path)); }
        }
    }

    fn start_emulator(&mut self, id: String, avd: String) {
        if self.starting_emulator.is_some() || self.launch_job.is_some() { return; }
        let Some(path) = self.inventory.emulator_path.clone() else { return; };
        let (tx, rx) = std::sync::mpsc::channel();
        self.launch_job = Some(rx);
        self.starting_emulator = Some((id, Instant::now()));
        self.status_message = format!("Starting {avd}. Waiting for its ADB connection…");
        self.status_is_error = false;
        std::thread::spawn(move || { let _ = tx.send(devices::launch(&path, &avd)); });
    }

    fn poll_emulator_launch(&mut self) {
        let result = self.launch_job.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err("Emulator launcher stopped unexpectedly.".into())),
        });
        if let Some(result) = result {
            self.launch_job = None;
            if let Err(error) = result {
                self.starting_emulator = None;
                self.status_message = error;
                self.status_is_error = true;
            }
        }
        if let Some((id, since)) = &self.starting_emulator {
            if self.inventory.devices.iter().any(|d| &d.id == id && d.state == devices::State::Online) {
                self.starting_emulator = None;
                self.status_message = "Emulator connected. Ready to mirror.".into();
            } else if self.launch_job.is_none() && since.elapsed() > Duration::from_secs(120) {
                self.starting_emulator = None;
                self.status_message = "Emulator has not connected after two minutes. Check its window and Android Studio Device Manager before retrying.".into();
                self.status_is_error = true;
            }
        }
    }

    fn start_mirror(&mut self, ctx: &egui::Context) {
        if !self.device_status.connected || self.device_status.device_serial.is_empty() { return; }
        if self.split.contains_serial(&self.device_status.device_serial) {
            self.status_message = "This device is already mirrored in another pane. Stop that session first.".into();
            self.status_is_error = true;
            return;
        }
        self.embedded.start(mirror::Config {
            serial: self.device_status.device_serial.clone(), adb_path: adb_path(),
            scrcpy_path: scrcpy_path(), env: build_full_env(),
        }, ctx.clone());
        self.mirror_running = true;
        self.status_message.clear();
        self.status_is_error = false;
    }

    fn close_mirror(&mut self) {
        self.embedded.stop();
        self.split.stop();
        self.mirror_running = false;
        if self.is_recording { self.stop_recording(); }
        self.status_message = "PhoneMirror sessions stopped. Other applications were not affected.".into();
        self.status_is_error = false;
    }

    fn take_screenshot(&mut self) {
        if !self.device_status.connected || self.device_status.device_serial.is_empty() { return; }
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
        if self.recording_child.is_some() || !self.device_status.connected || self.device_status.device_serial.is_empty() { return; }
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
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                self.recording_log = Default::default();
                if let Some(mut stderr) = child.stderr.take() {
                    let log = self.recording_log.clone();
                    self.recording_cleanup.push(std::thread::spawn(move || {
                        use std::io::Read;
                        let mut buffer = [0;4096];
                        while let Ok(n) = stderr.read(&mut buffer) {
                            if n == 0 { break; }
                            let mut tail = log.lock().unwrap(); tail.extend_from_slice(&buffer[..n]);
                            if tail.len() > 8192 { let excess = tail.len()-8192; tail.drain(..excess); }
                        }
                    }));
                }
                // Give scrcpy a moment to start — if it exits immediately, it's an error
                std::thread::sleep(std::time::Duration::from_millis(500));
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let reason = format!("{status}. {}", String::from_utf8_lossy(&self.recording_log.lock().unwrap()));
                        self.is_recording = false;
                        self.status_message = format!("Recording failed: {}", reason);
                        self.status_is_error = true;
                    }
                    Ok(None) | Err(_) => {
                        // Still running — success
                        self.is_recording = true;
                        self.recording_child = Some(child);
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
        if let Some(mut child) = self.recording_child.take() {
            // Keep ownership through graceful termination and reap off the UI thread.
            self.recording_cleanup.push(std::thread::spawn(move || {
                #[cfg(unix)]
                { let _ = std::process::Command::new("/bin/kill").args(["-INT", &child.id().to_string()]).status(); }
                #[cfg(not(unix))]
                { let _ = child.kill(); }
                let deadline = Instant::now() + Duration::from_secs(3);
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        _ if Instant::now() >= deadline => { let _ = child.kill(); let _ = child.wait(); break; }
                        _ => std::thread::sleep(Duration::from_millis(50)),
                    }
                }
            }));
        }
        self.is_recording = false;
        self.status_message = "Finishing the recording file…".into();
        self.status_is_error = false;
    }
}

// ─── Platform Paths ────────────────────────────────────────

fn adb_path() -> String {
    tools::path(tools::Tool::Adb)
}

fn scrcpy_path() -> String {
    tools::path(tools::Tool::Scrcpy)
}

/// Probe scrcpy with `--version`. Returns an actionable reason when it cannot run,
/// or an empty string when it launches cleanly. Catches broken installs early —
/// e.g. macOS dyld errors after a Homebrew ffmpeg major upgrade leave a *present*
/// scrcpy binary that crashes instantly on launch.
fn scrcpy_health() -> String {
    let scrcpy = scrcpy_path();
    let env = build_full_env();
    let result = std::process::Command::new(&scrcpy)
        .arg("--version")
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .output();

    match result {
        Ok(output) if output.status.success() => String::new(),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() { stderr } else { stdout };
            if detail.is_empty() {
                format!("scrcpy exited with code {}", output.status.code().unwrap_or(-1))
            } else {
                detail
            }
        }
        Err(e) => format!("{}", e),
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

    // Mirroring reads PHONEMIRROR_FFMPEG; point it at the detected or chosen copy.
    if let Some(ffmpeg) = tools::cached().and_then(|r| r.get(tools::Tool::Ffmpeg).path.clone()) {
        env_pairs.retain(|(k, _)| k != "PHONEMIRROR_FFMPEG");
        env_pairs.push(("PHONEMIRROR_FFMPEG".into(), ffmpeg));
    }

    env_pairs
}

impl Drop for PhoneMirrorApp {
    fn drop(&mut self) {
        self.embedded.stop();
        self.split.stop();
        self.embedded.shutdown_on_exit();
        self.split.shutdown_on_exit();
        if self.recording_child.is_some() { self.stop_recording(); }
        for cleanup in self.recording_cleanup.drain(..) { let _ = cleanup.join(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireless_serial_preserves_spaces() {
        let serial = "adb-RRCX107DMRP-ziYyJB (2)._adb-tls-connect._tcp";
        let mut app = PhoneMirrorApp::default();
        app.inventory.devices.push(devices::Device { id: "wifi".into(), name: "Phone".into(), serial: Some(serial.into()), avd: None, kind: devices::Kind::Wifi, state: devices::State::Online });
        app.sync_selection();
        assert_eq!(app.device_status.device_serial, serial);
    }
}
