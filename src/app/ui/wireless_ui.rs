use crate::app::wireless::{self, QrSession};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Code,
    Qr,
    Connect,
}

type WirelessJob = (mpsc::Receiver<Result<String, String>>, Arc<AtomicBool>);

#[derive(Default)]
pub struct WirelessUi {
    tab: Tab,
    pairing_address: String,
    code: String,
    connect_address: String,
    qr: Option<QrSession>,
    job: Option<WirelessJob>,
    message: String,
    error: bool,
    started: Option<std::time::Instant>,
    operation: String,
    show_code: bool,
}

impl Drop for WirelessUi {
    fn drop(&mut self) {
        if let Some((_, cancel)) = &self.job {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl WirelessUi {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    pub fn poll(&mut self) -> bool {
        let result = self.job.as_ref().and_then(|(rx, _)| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(
                "Wireless worker stopped unexpectedly. Try again.".into(),
            )),
        });
        if let Some(result) = result {
            self.job = None;
            self.started = None;
            self.qr = None;
            self.error = result.is_err();
            self.message = result.unwrap_or_else(|e| e);
            return true;
        }
        false
    }

    fn start(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce(&mut wireless::LocalAdb) -> Result<String, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut adb = wireless::LocalAdb {
            path: crate::app::adb_path(),
            env: crate::app::build_full_env(),
            cancel: cancel.clone(),
        };
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work(&mut adb));
            ctx.request_repaint();
        });
        self.job = Some((rx, cancel));
        self.started = Some(std::time::Instant::now());
        self.message.clear();
        self.error = false;
    }

    fn fail(&mut self, error: String) {
        self.message = error;
        self.error = true;
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        use crate::app::desktop_ui::{heading, muted, notice, primary};
        heading(
            ui,
            "Wi-Fi pairing",
            "Choose how to authorize or reconnect your phone.",
        );
        let busy = self.busy();
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (tab, label) in [
                    (Tab::Code, "Pairing code"),
                    (Tab::Qr, "QR code"),
                    (Tab::Connect, "Already paired"),
                ] {
                    ui.selectable_value(&mut self.tab, tab, label);
                }
            });
        });
        ui.add_space(8.0);
        ui.add_enabled_ui(!busy, |ui| {
            match self.tab {
                Tab::Code => {
                    instruction(ui, "1", "Open the pairing dialog", "Settings > Developer options > Wireless debugging > Pair device with pairing code. Leave it open.");
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("2  Enter the details shown on your phone").strong());
                    field(ui, "Pairing IP address & port", &mut self.pairing_address, "192.168.1.20:37123", false);
                    muted(ui, "Use the port inside the pairing dialog, not the main page.");
                    ui.label("Six-digit pairing code");
                    ui.add(egui::TextEdit::singleline(&mut self.code).id_salt("pair_secret").password(!self.show_code).hint_text("000000").font(egui::TextStyle::Monospace).margin(egui::vec2(12.0, 11.0)).desired_width(f32::INFINITY));
                    ui.checkbox(&mut self.show_code, "Show pairing code");
                    if primary(ui, "Pair this phone").clicked() {
                        let input = wireless::endpoint(&self.pairing_address).and_then(|address| wireless::pairing_code(&self.code).map(|code| (address, code)));
                        match input {
                            Ok((address, code)) => {
                                self.code.clear(); self.operation = "Authorizing this laptop".into();
                                self.start(ui.ctx(), move |adb| wireless::pair(adb, &address, &code));
                            }
                            Err(error) => self.fail(error),
                        }
                    }
                }
                Tab::Qr => {
                    instruction(ui, "1", "Open your phone's QR scanner", "Wireless debugging > Pair device with QR code. Use this scanner, not the Camera app.");
                    instruction(ui, "2", "Generate a QR, then scan it", "Your phone scans this laptop. Keep both devices on the same private Wi-Fi network.");
                    if primary(ui, "Generate pairing QR").clicked() {
                        match QrSession::new() {
                            Ok(qr) => {
                                let name = qr.name.clone(); let password = qr.password.clone();
                                self.qr = Some(qr); self.operation = "Waiting for the phone's pairing service".into();
                                self.start(ui.ctx(), move |adb| wireless::pair_qr(adb, &name, &password));
                            }
                            Err(error) => self.fail(error),
                        }
                    }
                }
                Tab::Connect => {
                    instruction(ui, "1", "Return to Wireless debugging", "Close the pairing dialog. Find IP address & port on the main Wireless debugging page.");
                    notice(ui, "Use the connection port", "This is different from the pairing port. Copy the current value; it can change when Wireless debugging restarts.", false);
                    field(ui, "Connection IP address & port", &mut self.connect_address, "192.168.1.20:40157", false);
                    if primary(ui, "Connect phone").clicked() {
                        match wireless::endpoint(&self.connect_address) {
                            Ok(address) => { self.operation = "Opening the wireless connection".into(); self.start(ui.ctx(), move |adb| wireless::connect(adb, &address)); }
                            Err(error) => self.fail(error),
                        }
                    }
                }
            }
        });
        if busy {
            ui.add_space(8.0);
            egui::Frame::new().fill(crate::app::INFO_FILL).corner_radius(10).inner_margin(12).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new(&self.operation).strong());
                    if ui.button("Cancel").clicked() {
                        if let Some((_, cancel)) = &self.job { cancel.store(true, Ordering::Relaxed); }
                        self.qr = None;
                    }
                });
                if let Some(started) = self.started {
                    let seconds = started.elapsed().as_secs();
                    muted(ui, &if self.qr.is_some() { format!("{} seconds remaining. No phone discovered yet; scanning must finish on the phone.", 120u64.saturating_sub(seconds)) } else { format!("Elapsed: {seconds}s. Network commands time out rather than freezing the app.") });
                }
            });
        }
        if let Some(qr) = &self.qr {
            ui.add_space(12.0);
            draw_qr(ui, &qr.code);
            ui.add_space(8.0);
            muted(
                ui,
                "Keep this QR private. Scanning authorizes debugging access to this laptop.",
            );
        }
        if !self.message.is_empty() {
            ui.add_space(12.0);
            let title = result_title(&self.message, self.error);
            let detail = result_guidance(&self.message, self.error);
            notice(ui, title, detail, self.error);
            if self.error {
                ui.add(
                    egui::Label::new(egui::RichText::new(&self.message).small().color(crate::app::RED))
                        .wrap(),
                );
            }
            ui.collapsing("Technical details", |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("wireless_details")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(&self.message).small()).wrap());
                    });
            });
            if ui.button("Copy details").clicked() {
                ui.ctx().copy_text(self.message.clone());
            }
            if self.error
                && self.tab == Tab::Qr
                && !busy
                && ui.button("Try pairing code instead").clicked()
            {
                self.tab = Tab::Code;
                self.message.clear();
            }
        }
        ui.add_space(12.0);
        ui.separator();
        if ui
            .add_enabled(!busy, egui::Button::new("Run connection diagnostics"))
            .clicked()
        {
            self.operation = "Checking ADB and local discovery".into();
            self.start(ui.ctx(), wireless::diagnose);
        }
        ui.collapsing("Samsung Galaxy setup & network help", |ui| {
            ui.label("On Galaxy: Settings > About phone > Software information > tap Build number seven times. Then Settings > Developer options > Wireless debugging. Accept the Wi-Fi prompt and keep the screen unlocked.");
            ui.label("Same Wi-Fi name is not proof of reachability. Routers can isolate clients even on the same SSID. A 5 GHz network is supported; the band alone is not an error.");
            ui.label("On macOS, review System Settings > Privacy & Security > Local Network. Allow PhoneMirror and the ADB host application if listed. Check VPN/firewall rules without disabling your firewall globally.");
            ui.label("No discovery? Pairing code uses a direct IP connection and does not require mDNS. Keep its dialog open and use a fresh code/port. USB is also available as a fallback.");
        });
    }
}

fn instruction(ui: &mut egui::Ui, number: &str, title: &str, body: &str) {
    ui.label(egui::RichText::new(format!("{number}  {title}")).strong());
    crate::app::desktop_ui::muted(ui, body);
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, password: bool) {
    ui.label(label);
    ui.add(
        egui::TextEdit::singleline(value)
            .id_salt(label)
            .password(password)
            .hint_text(hint)
            .font(egui::TextStyle::Monospace)
            .margin(egui::vec2(12.0, 11.0))
            .desired_width(f32::INFINITY),
    );
}
fn result_title(message: &str, error: bool) -> &'static str {
    if message.contains("No route to host") {
        "The phone is not reachable from this laptop"
    } else if message.contains("Connection refused") {
        "The phone is not accepting this port"
    } else if message.contains("protocol fault") {
        "ADB protocol connection ended unexpectedly"
    } else if message.contains("QR pairing timed out") {
        "Your phone wasn't discovered"
    } else if message.contains("Paired successfully") {
        "Paired · connection may still be needed"
    } else if message.contains("wireless service(s)") {
        "Connection diagnostics"
    } else if error {
        "Connection needs attention"
    } else {
        "Connection update"
    }
}
fn result_guidance(message: &str, error: bool) -> &'static str {
    if message.contains("No route to host") {
        "The laptop could not reach the phone before code verification. Recheck the phone IP, macOS Local Network permissions, and Wi-Fi client isolation. A fresh code cannot fix a blocked network route."
    } else if message.contains("Connection refused") {
        "The address was reached but this port refused the connection. Keep the pairing dialog open and use its fresh port; do not use the main connection port for pairing."
    } else if message.contains("protocol fault") {
        "ADB closed the request before reporting a pairing result. This can mask a network failure. Run diagnostics, check macOS Local Network access, and retry with the current pairing-dialog IP and port."
    } else if message.contains("QR pairing timed out") {
        "The phone's pairing service never reached this laptop. Try pairing by code. Review local-network permissions, VPN and router client isolation if that also fails."
    } else if message.contains("Paired successfully") {
        "Your laptop is authorized. If no device appears in Your session, choose Already paired and enter the connection port from the phone's main Wireless debugging page."
    } else if message.contains("0 wireless service(s)") {
        "ADB is responding but no wireless services are visible. Check the phone's Wireless debugging screen and local-network permissions. Direct code pairing bypasses discovery."
    } else if message.contains("wireless service(s)") {
        "The checks finished. Expand Technical details for ADB and discovery results."
    } else if error {
        "Review the exact error in Technical details. Keep the phone's pairing dialog open and use its current port and code. Pairing and connection use different ports."
    } else {
        "Review Technical details for the result. Session controls unlock when ADB reports an online device."
    }
}

fn draw_qr(ui: &mut egui::Ui, code: &qrcode::QrCode) {
    // Integer physical pixels and a four-module quiet zone keep it scannable on Retina displays.
    let modules = code.width() + 8;
    let pixels_per_point = ui.ctx().pixels_per_point();
    let scale = ((ui.available_width().min(260.0) * pixels_per_point) / modules as f32)
        .floor()
        .max(1.0)
        / pixels_per_point;
    let size = modules as f32 * scale;
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let origin = egui::pos2(
            (rect.left() * pixels_per_point).round() / pixels_per_point,
            (rect.top() * pixels_per_point).round() / pixels_per_point,
        );
        ui.painter().rect_filled(
            egui::Rect::from_min_size(origin, egui::vec2(size, size)),
            0.0,
            egui::Color32::WHITE,
        );
        for y in 0..code.width() {
            for x in 0..code.width() {
                if code[(x, y)] == qrcode::Color::Dark {
                    let min = origin + egui::vec2((x + 4) as f32 * scale, (y + 4) as f32 * scale);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(min, egui::vec2(scale, scale)),
                        0.0,
                        egui::Color32::BLACK,
                    );
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_tabs_render_at_minimum_width() {
        let ctx = egui::Context::default();
        let mut panel = WirelessUi::default();
        for tab in [Tab::Code, Tab::Qr, Tab::Connect] {
            panel.tab = tab;
            if panel.tab == Tab::Qr {
                panel.qr = Some(QrSession::new().unwrap());
            } else {
                panel.qr = None;
            }
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(292.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
                },
            );
        }
    }
    #[test]
    fn actionable_error_titles_do_not_claim_pairing_success() {
        assert_eq!(
            result_title("ADB failed: protocol fault", true),
            "ADB protocol connection ended unexpectedly"
        );
        assert_eq!(
            result_title("No route to host", true),
            "The phone is not reachable from this laptop"
        );
        assert!(result_guidance("No route to host", true).contains("before code verification"));
    }
    #[test]
    fn worker_result_clears_qr_and_busy_state() {
        let (tx, rx) = mpsc::channel();
        let mut panel = WirelessUi::default();
        panel.qr = Some(QrSession::new().unwrap());
        panel.job = Some((rx, Arc::new(AtomicBool::new(false))));
        tx.send(Err("test failure".into())).unwrap();
        assert!(panel.poll());
        assert!(!panel.busy());
        assert!(panel.qr.is_none());
        assert!(panel.error);
        assert_eq!(panel.message, "test failure");
    }
}
