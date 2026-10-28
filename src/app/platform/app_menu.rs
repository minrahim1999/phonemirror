//! Native macOS menu bar. Menu items only post an [`Action`]; the app applies it on
//! the UI thread with the same guards as the toolbar, so the menu never bypasses them.

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use std::sync::mpsc::{channel, Receiver};

pub const HELP_URL: &str = "https://github.com/minrahim1999/phonemirror";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Close,
    Refresh,
    PairWifi,
    Screenshot,
    Record,
    Mirror,
    Fit,
    ZoomIn,
    ZoomOut,
    DeviceInfo,
    Keyboard,
    Nav(u32),
    Help,
}

type Shortcut = Option<(Modifiers, Code)>;

const NAV_KEYS: [(&str, u32, Shortcut); 6] = [
    ("Back", 4, Some((Modifiers::META, Code::BracketLeft))),
    (
        "Home",
        3,
        Some((Modifiers::META.union(Modifiers::SHIFT), Code::KeyH)),
    ),
    ("Recent apps", 187, None),
    ("Power", 26, None),
    ("Volume up", 24, None),
    ("Volume down", 25, None),
];

fn id(action: Action) -> String {
    match action {
        Action::Nav(code) => format!("nav:{code}"),
        other => format!("{other:?}"),
    }
}

pub fn parse(id: &str) -> Option<Action> {
    if let Some(code) = id.strip_prefix("nav:") {
        return code.parse().ok().map(Action::Nav);
    }
    use Action::*;
    [
        Close, Refresh, PairWifi, Screenshot, Record, Mirror, Fit, ZoomIn, ZoomOut, DeviceInfo,
        Keyboard, Help,
    ]
    .into_iter()
    .find(|a| format!("{a:?}") == id)
}

/// Facts the menu reflects; it is re-synced only when they change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuState {
    pub can_mirror: bool,
    pub mirroring: bool,
    pub can_screenshot: bool,
    pub can_record: bool,
    pub recording: bool,
    pub streaming: bool,
    pub keyboard: bool,
    pub info: bool,
    pub can_refresh: bool,
}

pub struct AppMenu {
    _menu: Menu,
    rx: Receiver<Action>,
    mirror: MenuItem,
    screenshot: MenuItem,
    record: MenuItem,
    refresh: MenuItem,
    zoom: [MenuItem; 3],
    nav: Vec<MenuItem>,
    keyboard: CheckMenuItem,
    info: CheckMenuItem,
    last: Option<MenuState>,
}

fn item(text: &str, action: Action, keys: Shortcut) -> MenuItem {
    MenuItem::with_id(
        id(action),
        text,
        true,
        keys.map(|(m, c)| Accelerator::new(m, c)),
    )
}
const CMD: Modifiers = Modifiers::META;
const CMD_SHIFT: Modifiers = Modifiers::META.union(Modifiers::SHIFT);

/// Builds the menu bar and replaces winit's default one. Call once the app is running.
pub fn install(ctx: &egui::Context) -> Result<AppMenu, muda::Error> {
    let sep = PredefinedMenuItem::separator;
    let about = AboutMetadata {
        name: Some("PhoneMirror".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        comments: Some("Mirror and control Android devices.".into()),
        website: Some(HELP_URL.into()),
        ..Default::default()
    };
    let quit = item("Quit PhoneMirror", Action::Close, Some((CMD, Code::KeyQ)));
    let app = Submenu::with_items(
        "PhoneMirror",
        true,
        &[
            &PredefinedMenuItem::about(Some("About PhoneMirror"), Some(about)),
            &sep(),
            &PredefinedMenuItem::services(None),
            &sep(),
            &PredefinedMenuItem::hide(Some("Hide PhoneMirror")),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &sep(),
            &quit,
        ],
    )?;

    let refresh = item("Refresh devices", Action::Refresh, Some((CMD, Code::KeyR)));
    let screenshot = item(
        "Save screenshot",
        Action::Screenshot,
        Some((CMD, Code::KeyS)),
    );
    let record = item(
        "Record screen",
        Action::Record,
        Some((CMD_SHIFT, Code::KeyR)),
    );
    let file = Submenu::with_items(
        "File",
        true,
        &[
            &refresh,
            &item(
                "Pair a Wi-Fi device…",
                Action::PairWifi,
                Some((CMD_SHIFT, Code::KeyP)),
            ),
            &sep(),
            &screenshot,
            &record,
            &sep(),
            &item("Close window", Action::Close, Some((CMD, Code::KeyW))),
        ],
    )?;

    let zoom = [
        item("Fit screen", Action::Fit, Some((CMD, Code::Digit0))),
        item("Zoom in", Action::ZoomIn, Some((CMD, Code::Equal))),
        item("Zoom out", Action::ZoomOut, Some((CMD, Code::Minus))),
    ];
    let info = CheckMenuItem::with_id(
        id(Action::DeviceInfo),
        "Device information",
        true,
        false,
        Some(Accelerator::new(CMD, Code::KeyI)),
    );
    let view = Submenu::with_items(
        "View",
        true,
        &[
            &zoom[0],
            &zoom[1],
            &zoom[2],
            &sep(),
            &info,
            // macOS appends its own "Enter Full Screen" (Ctrl+Cmd+F) here.
        ],
    )?;

    let mirror = item("Start mirroring", Action::Mirror, Some((CMD, Code::Enter)));
    let keyboard = CheckMenuItem::with_id(
        id(Action::Keyboard),
        "Forward keyboard input",
        true,
        false,
        Some(Accelerator::new(CMD, Code::KeyK)),
    );
    let nav: Vec<_> = NAV_KEYS
        .iter()
        .map(|&(label, code, keys)| item(label, Action::Nav(code), keys))
        .collect();
    let device = Submenu::with_items("Device", true, &[&mirror, &keyboard, &sep()])?;
    for nav_item in &nav {
        device.append(nav_item)?;
    }

    let window = Submenu::with_items(
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(Some("Zoom")),
            &sep(),
            &PredefinedMenuItem::bring_all_to_front(None),
        ],
    )?;
    let help = Submenu::with_items(
        "Help",
        true,
        &[&item("PhoneMirror on GitHub", Action::Help, None)],
    )?;

    let menu = Menu::with_items(&[&app, &file, &view, &device, &window, &help])?;
    menu.init_for_nsapp();
    window.set_as_windows_menu_for_nsapp();
    help.set_as_help_menu_for_nsapp();

    // Menu clicks arrive outside egui's input; wake the UI so they apply immediately.
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if let Some(action) = parse(&event.id.0) {
            let _ = tx.send(action);
            ctx.request_repaint();
        }
    }));

    Ok(AppMenu {
        _menu: menu,
        rx,
        mirror,
        screenshot,
        record,
        refresh,
        zoom,
        nav,
        keyboard,
        info,
        last: None,
    })
}

impl AppMenu {
    pub fn actions(&mut self) -> Vec<Action> {
        let actions: Vec<_> = self.rx.try_iter().collect();
        if !actions.is_empty() {
            // Check items toggle themselves natively; force a resync to the real state.
            self.last = None;
        }
        actions
    }

    pub fn sync(&mut self, state: MenuState) {
        if self.last == Some(state) {
            return;
        }
        self.last = Some(state);
        self.mirror.set_text(if state.mirroring {
            "Stop mirroring"
        } else {
            "Start mirroring"
        });
        self.mirror.set_enabled(state.can_mirror);
        self.screenshot.set_enabled(state.can_screenshot);
        self.record.set_text(if state.recording {
            "Stop recording"
        } else {
            "Record screen"
        });
        self.record.set_enabled(state.can_record);
        self.refresh.set_enabled(state.can_refresh);
        for item in self.zoom.iter().chain(&self.nav) {
            item.set_enabled(state.streaming);
        }
        self.keyboard.set_enabled(state.streaming);
        self.keyboard.set_checked(state.streaming && state.keyboard);
        self.info.set_checked(state.info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_round_trip_to_actions() {
        use Action::*;
        for action in [
            Close,
            Refresh,
            PairWifi,
            Screenshot,
            Record,
            Mirror,
            Fit,
            ZoomIn,
            ZoomOut,
            DeviceInfo,
            Keyboard,
            Help,
            Nav(4),
            Nav(187),
        ] {
            assert_eq!(parse(&id(action)), Some(action));
        }
        assert_eq!(parse("nav:x"), None);
        assert_eq!(parse("unknown"), None);
    }
}
