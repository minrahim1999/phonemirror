//! Native live-screen texture and focused device input. No subprocess runs on the UI thread.
use crate::app::mirror::{Config, Input, Session, Status};
use egui::{Color32, Pos2, Rect, Vec2};

#[derive(Default)]
pub(crate) struct MirrorUi {
    session: Option<Session>,
    retired: Vec<Session>,
    texture: Option<egui::TextureHandle>,
    dimensions: [u16; 2],
    touch: Option<(u32, u32)>,
    zoom: f32,
    keyboard_disabled: bool,
}

impl MirrorUi {
    #[cfg(test)]
    pub(crate) fn fixture(
        serial: &str,
        status: Status,
        frame: Option<crate::app::mirror::Frame>,
    ) -> Self {
        Self {
            session: Some(Session::fixture(serial, status, frame)),
            ..Self::default()
        }
    }
    pub fn start(&mut self, config: Config, ctx: egui::Context) {
        self.stop();
        self.session = Some(Session::start(config, ctx));
    }
    pub fn active(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| matches!(s.status(), Status::Starting | Status::Streaming))
    }
    pub fn diagnostic(&self) -> String {
        self.session.as_ref().map(|session| format!("{}: {:?}", session.serial(), session.status())).unwrap_or_else(|| "No session".into())
    }
    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }
    pub fn reconcile_target(&mut self, serial: &str) {
        if self.session.as_ref().is_some_and(|s| s.serial() != serial) {
            self.stop();
        }
    }
    pub fn poll(&mut self, ctx: &egui::Context) {
        self.retired
            .retain(|s| !s.wait_cleanup(std::time::Duration::ZERO));
        if let Some(session) = &self.session {
            if matches!(session.status(), Status::Stopped | Status::Failed(_)) {
                self.texture = None;
                self.touch = None;
                return;
            }
            if let Some(frame) = session.take_frame() {
                let dims = [frame.width as u16, frame.height as u16];
                if dims != self.dimensions {
                    self.dimensions = dims;
                    self.release_touch();
                }
                self.dimensions = dims;
                let image = egui::ColorImage::from_rgb(
                    [frame.width as usize, frame.height as usize],
                    &frame.rgb,
                );
                if let Some(texture) = &mut self.texture {
                    texture.set(image, egui::TextureOptions::LINEAR);
                } else {
                    self.texture = Some(ctx.load_texture(
                        "embedded-device-screen",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
        }
    }
    pub fn stop(&mut self) {
        self.release_touch();
        if let Some(session) = self.session.take() {
            session.stop();
            self.retired.push(session);
        }
        self.texture = None;
        self.dimensions = [0, 0];
    }
    pub fn shutdown_on_exit(&mut self) {
        self.stop();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(35);
        for session in self.retired.drain(..) {
            session.wait_cleanup(deadline.saturating_duration_since(std::time::Instant::now()));
        }
    }
    pub fn release_touch(&mut self) {
        if let Some((x, y)) = self.touch.take() {
            if !self.dimensions.contains(&0) {
                self.send_touch(
                    3,
                    x.min(u32::from(self.dimensions[0]) - 1),
                    y.min(u32::from(self.dimensions[1]) - 1),
                );
            }
        }
    }
    fn send_touch(&self, action: u8, x: u32, y: u32) {
        if let Some(session) = &self.session {
            session.send(Input::Touch {
                action,
                x,
                y,
                width: self.dimensions[0],
                height: self.dimensions[1],
            });
        }
    }
    fn key(&self, code: u32) {
        if let Some(session) = &self.session {
            session.send(Input::Key { code, down: true });
            session.send(Input::Key { code, down: false });
        }
    }

    pub fn streaming(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| s.status() == Status::Streaming)
            && self.texture.is_some()
    }
    /// Wake must not depend on a decoded texture: a sleeping display may have none yet.
    pub fn can_wake(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| s.status() == Status::Streaming)
    }
    pub fn wake_screen(&self) {
        if self.can_wake() {
            self.key(224); // Android KEYCODE_WAKEUP, never toggles an already-awake screen off.
        }
    }
    pub fn navigation_key(&self, code: u32) {
        if self.streaming() {
            self.key(code);
        }
    }
    pub fn keyboard_enabled(&self) -> bool {
        !self.keyboard_disabled
    }
    pub fn toggle_keyboard(&mut self) {
        self.keyboard_disabled = !self.keyboard_disabled;
        self.release_touch();
    }
    pub fn fit(&mut self) {
        self.release_touch();
        self.zoom = 0.0;
    }
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = ((if self.zoom == 0.0 { 1.0 } else { self.zoom }) * factor).clamp(0.5, 3.0);
        self.release_touch();
    }
    pub fn zoom_label(&self) -> String {
        if self.zoom == 0.0 {
            "Fit".into()
        } else {
            format!("{}%", (self.zoom * 100.0).round() as u32)
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let Some(status) = self.session.as_ref().map(|s| s.status()) else {
            return;
        };
        if let Some(texture_id) = self.texture.as_ref().map(|t| t.id()) {
            let source = Vec2::new(self.dimensions[0] as f32, self.dimensions[1] as f32);
            let available = ui.available_size().max(Vec2::splat(1.0));
            let fitted = fitted_size(
                source,
                (available - Vec2::splat(40.0)).max(Vec2::splat(1.0)),
            );
            let size = fitted * if self.zoom == 0.0 { 1.0 } else { self.zoom };
            // Zoom is relative to fit. Oversized screens pan in a clipped two-axis viewport.
            egui::ScrollArea::both().id_salt("screen-pan").drag_to_scroll(false).auto_shrink([false,false]).show(ui,|ui| {
                let extent=available.max(size+Vec2::splat(24.0));
                let (area,_)=ui.allocate_exact_size(extent,egui::Sense::hover());
                let rect=Rect::from_center_size(area.center(),size);
                let response=ui.interact(rect,ui.id().with("live-screen"),egui::Sense::click_and_drag());
                response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Image,ui.is_enabled(),"Live Android screen. Click to focus, drag to swipe, Esc to release keyboard."));
                // Device bezel: screen corners and bezel width scale with the rendered size.
                let screen_radius=(size.min_elem()*0.04).clamp(6.0,24.0);
                let bezel=(size.x*0.022).clamp(6.0,12.0);
                let shell=rect.expand(bezel);
                let shell_radius=screen_radius+bezel;
                let focus=ui.ctx().animate_bool_with_time(response.id.with("focus"),response.has_focus() && !self.keyboard_disabled,0.2);
                let painter=ui.painter();
                painter.add(egui::Shadow{offset:[0,18],blur:48,spread:0,color:Color32::from_black_alpha(150)}.as_shape(shell,shell_radius as u8));
                if focus>0.0 {
                    painter.add(egui::Shadow{offset:[0,0],blur:28,spread:2,color:crate::app::ACCENT.gamma_multiply(0.35*focus)}.as_shape(shell,shell_radius as u8));
                }
                painter.rect_filled(shell,shell_radius,Color32::from_rgb(9,10,13));
                painter.rect_stroke(shell,shell_radius,egui::Stroke::new(1.0,Color32::from_rgb(58,63,75).lerp_to_gamma(crate::app::ACCENT,focus)),egui::StrokeKind::Inside);
                painter.rect_stroke(shell.shrink(1.5),shell_radius-1.5,egui::Stroke::new(1.0,Color32::from_white_alpha(8)),egui::StrokeKind::Inside);
                egui::Image::from_texture(egui::load::SizedTexture::new(texture_id,size)).corner_radius(screen_radius as u8).paint_at(ui,rect);
                self.handle_input(ui,rect,&response);
            });
        } else {
            let available = ui.available_size();
            ui.allocate_ui_with_layout(
                available,
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.add_space((available.y * 0.35).max(0.0));
                    match status {
                        Status::Starting => {
                            ui.spinner();
                            ui.label("Connecting to device…");
                            crate::app::ui::components::muted(
                                ui,
                                "Starting screen capture and video decoder",
                            );
                        }
                        Status::Failed(error) => {
                            ui.label(
                                egui::RichText::new("Unable to mirror this device")
                                    .strong()
                                    .size(18.0),
                            );
                            ui.add(egui::Label::new(error).wrap());
                        }
                        _ => {
                            ui.label("Mirroring stopped");
                        }
                    }
                },
            );
        }
    }

    fn handle_input(&mut self, ui: &mut egui::Ui, rect: Rect, response: &egui::Response) {
        if !ui.is_enabled() {
            self.release_touch();
            return;
        }
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    ..
                } => {
                    if pressed
                        && response.contains_pointer()
                        && rect.contains(pos)
                        && ui.clip_rect().contains(pos)
                    {
                        response.request_focus();
                        if let Some((x, y)) = map_position(rect, pos, self.dimensions, false) {
                            self.send_touch(0, x, y);
                            self.touch = Some((x, y));
                        }
                    } else if !pressed && self.touch.is_some() {
                        if let Some((x, y)) = map_position(rect, pos, self.dimensions, true) {
                            self.send_touch(1, x, y);
                        }
                        self.touch = None;
                    }
                }
                egui::Event::PointerMoved(pos) if self.touch.is_some() => {
                    if let Some((x, y)) = map_position(rect, pos, self.dimensions, true) {
                        self.send_touch(2, x, y);
                        self.touch = Some((x, y));
                    }
                }
                egui::Event::PointerGone | egui::Event::WindowFocused(false) => {
                    self.release_touch()
                }
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } if response.has_focus() && !self.keyboard_disabled => {
                    self.release_touch();
                    response.surrender_focus();
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if response.has_focus()
                    && !self.keyboard_disabled
                    && !modifiers.command
                    && !modifiers.ctrl
                    && !modifiers.alt =>
                {
                    let code = match key {
                        egui::Key::Enter => Some(66),
                        egui::Key::Backspace => Some(67),
                        egui::Key::Delete => Some(112),
                        egui::Key::ArrowUp => Some(19),
                        egui::Key::ArrowDown => Some(20),
                        egui::Key::ArrowLeft => Some(21),
                        egui::Key::ArrowRight => Some(22),
                        _ => None,
                    };
                    if let Some(code) = code {
                        self.key(code);
                    }
                }
                egui::Event::Text(text) | egui::Event::Paste(text)
                    if response.has_focus() && !self.keyboard_disabled =>
                {
                    if let Some(session) = &self.session {
                        session.send(Input::Text(text));
                    }
                }
                _ => {}
            }
        }
        if self.touch.is_some()
            && (!ui.input(|i| i.pointer.primary_down()) || !ui.input(|i| i.focused))
        {
            self.release_touch();
        }
    }
}

fn fitted_size(source: Vec2, available: Vec2) -> Vec2 {
    if source.x <= 0.0 || source.y <= 0.0 {
        return Vec2::ZERO;
    }
    source
        * (available.x / source.x)
            .min(available.y / source.y)
            .max(0.0)
}
fn map_position(rect: Rect, pos: Pos2, dims: [u16; 2], clamp: bool) -> Option<(u32, u32)> {
    if dims.contains(&0) || !rect.is_positive() || (!clamp && !rect.contains(pos)) {
        return None;
    }
    let normalized = (pos - rect.min) / rect.size();
    Some((
        (normalized.x.clamp(0.0, 1.0) * dims[0] as f32)
            .floor()
            .min((dims[0] - 1) as f32) as u32,
        (normalized.y.clamp(0.0, 1.0) * dims[1] as f32)
            .floor()
            .min((dims[1] - 1) as f32) as u32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toolbar_zoom_is_relative_to_fit_and_bounded() {
        let mut mirror = MirrorUi::default();
        assert_eq!(mirror.zoom_label(), "Fit");
        mirror.zoom_by(1.25);
        assert_eq!(mirror.zoom_label(), "125%");
        mirror.zoom_by(10.0);
        assert_eq!(mirror.zoom_label(), "300%");
        mirror.zoom_by(0.01);
        assert_eq!(mirror.zoom_label(), "50%");
        mirror.fit();
        assert_eq!(mirror.zoom_label(), "Fit");
        assert!(mirror.keyboard_enabled());
        mirror.toggle_keyboard();
        assert!(!mirror.keyboard_enabled());
    }

    #[test]
    fn typing_requires_focus_and_drag_release_is_sent() {
        let ctx = egui::Context::default();
        let (session, rx) = Session::fixture_with_input("fixture", Status::Streaming, None);
        let mut mirror = MirrorUi {
            session: Some(session),
            dimensions: [100, 200],
            ..Default::default()
        };
        for (frame, events) in [
            vec![egui::Event::Text("unfocused".into())],
            vec![egui::Event::Text("focused".into())],
            vec![egui::Event::Text("disabled".into())],
        ]
        .into_iter()
        .enumerate()
        {
            if frame == 2 {
                mirror.toggle_keyboard();
            }
            let _ = ctx.run(
                egui::RawInput {
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let rect =
                            Rect::from_min_size(Pos2::new(30.0, 30.0), Vec2::new(100.0, 200.0));
                        let response = ui.interact(
                            rect,
                            ui.id().with("screen-test"),
                            egui::Sense::click_and_drag(),
                        );
                        mirror.handle_input(ui, rect, &response);
                        if frame == 0 {
                            response.request_focus();
                        }
                    });
                },
            );
            if frame == 0 {
                assert!(
                    rx.try_recv().is_err(),
                    "Unfocused typing must stay on desktop"
                );
            }
        }
        let packet = rx.try_recv().expect("Focused text must be forwarded");
        assert_eq!(packet[0], 1);
        assert_eq!(&packet[5..], b"focused");
        assert!(
            rx.try_recv().is_err(),
            "Disabled keyboard input must not be forwarded"
        );
        mirror.touch = Some((90, 180));
        mirror.dimensions = [200, 100];
        mirror.release_touch();
        let packet = rx.try_recv().expect("Release is never silently omitted");
        assert_eq!(&packet[..2], &[2, 3]);
        assert_eq!(u16::from_be_bytes(packet[18..20].try_into().unwrap()), 200);
        assert_eq!(u16::from_be_bytes(packet[20..22].try_into().unwrap()), 100);
        assert_eq!(u32::from_be_bytes(packet[14..18].try_into().unwrap()), 99);
    }

    #[test]
    fn wake_screen_sends_wakeup_without_a_frame_or_keyboard_focus() {
        let (session, rx) = Session::fixture_with_input("phone", Status::Streaming, None);
        let mut mirror = MirrorUi {
            session: Some(session),
            keyboard_disabled: true,
            ..Default::default()
        };
        assert!(!mirror.streaming());
        assert!(mirror.can_wake());
        mirror.wake_screen();
        for action in [0, 1] {
            assert_eq!(
                rx.try_recv().unwrap(),
                vec![0, action, 0, 0, 0, 224, 0, 0, 0, 0, 0, 0, 0, 0]
            );
        }
        mirror.reconcile_target("other-phone");
        assert!(!mirror.can_wake());
        mirror.wake_screen();
        assert!(rx.try_recv().is_err());
        for status in [
            Status::Starting,
            Status::Stopped,
            Status::Failed("offline".into()),
        ] {
            let (session, rx) = Session::fixture_with_input("phone", status, None);
            let mirror = MirrorUi {
                session: Some(session),
                ..Default::default()
            };
            assert!(!mirror.can_wake());
            mirror.wake_screen();
            assert!(rx.try_recv().is_err());
        }
    }

    #[test]
    fn fixture_frame_upload_and_target_switch_clear_screen() {
        let ctx = egui::Context::default();
        let mut mirror = MirrorUi::fixture(
            "phone-a",
            Status::Streaming,
            Some(crate::app::mirror::Frame {
                width: 2,
                height: 3,
                rgb: vec![90; 18],
            }),
        );
        mirror.poll(&ctx);
        assert!(mirror.texture.is_some());
        assert_eq!(mirror.dimensions, [2, 3]);
        mirror.reconcile_target("phone-b");
        assert!(!mirror.active());
        assert!(!mirror.has_session());
        assert!(mirror.texture.is_none());
        assert_eq!(mirror.dimensions, [0, 0]);
    }
    #[test]
    fn fixture_failed_state_never_displays_stale_screen() {
        let ctx = egui::Context::default();
        let mut mirror = MirrorUi::fixture(
            "phone",
            Status::Failed("Disconnected".into()),
            Some(crate::app::mirror::Frame {
                width: 2,
                height: 3,
                rgb: vec![90; 18],
            }),
        );
        mirror.poll(&ctx);
        assert!(!mirror.active());
        assert!(mirror.texture.is_none());
    }
    #[test]
    fn fitted_screen_preserves_portrait_and_landscape_aspect() {
        assert_eq!(
            fitted_size(Vec2::new(1080.0, 1920.0), Vec2::new(500.0, 600.0)),
            Vec2::new(337.5, 600.0)
        );
        assert_eq!(
            fitted_size(Vec2::new(1920.0, 1080.0), Vec2::new(600.0, 500.0)),
            Vec2::new(600.0, 337.5)
        );
    }
    #[test]
    fn mapping_rejects_letterbox_and_clamps_drag_release() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(200.0, 400.0));
        assert_eq!(
            map_position(rect, Pos2::new(200.0, 250.0), [1000, 2000], false),
            Some((500, 1000))
        );
        assert_eq!(
            map_position(rect, Pos2::new(50.0, 250.0), [1000, 2000], false),
            None
        );
        assert_eq!(
            map_position(rect, Pos2::new(500.0, 900.0), [1000, 2000], true),
            Some((999, 1999))
        );
        assert_eq!(map_position(rect, rect.min, [0, 0], false), None);
    }
}
