use std::process::Command;
use std::path::PathBuf;
use chrono::Local;

// ─── Platform-specific paths ──────────────────────────────

fn adb_path() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());

    if cfg!(target_os = "windows") {
        let path = format!("{}\\AppData\\Local\\Android\\Sdk\\platform-tools\\adb.exe", home);
        if PathBuf::from(&path).exists() { return path; }
        "adb.exe".to_string()
    } else if cfg!(target_os = "macos") {
        let path = format!("{}/Library/Android/sdk/platform-tools/adb", home);
        if PathBuf::from(&path).exists() { return path; }
        "adb".to_string()
    } else {
        for candidate in &["/usr/bin/adb", "/usr/local/bin/adb"] {
            if PathBuf::from(candidate).exists() { return candidate.to_string(); }
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
            if PathBuf::from(candidate).exists() { return candidate.to_string(); }
        }
        "scrcpy.exe".to_string()
    } else if cfg!(target_os = "macos") {
        let path = "/opt/homebrew/bin/scrcpy".to_string();
        if PathBuf::from(&path).exists() { return path; }
        "scrcpy".to_string()
    } else {
        for candidate in &["/usr/bin/scrcpy", "/usr/local/bin/scrcpy", "/snap/bin/scrcpy"] {
            if PathBuf::from(candidate).exists() { return candidate.to_string(); }
        }
        "scrcpy".to_string()
    }
}

fn screenshots_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());

    if cfg!(target_os = "windows") {
        let path = format!("{}\\Pictures\\PhoneMirror", home);
        std::fs::create_dir_all(&path).ok();
        path
    } else if cfg!(target_os = "macos") {
        format!("{}/Pictures", home)
    } else {
        let path = format!("{}/Pictures/PhoneMirror", home);
        std::fs::create_dir_all(&path).ok();
        path
    }
}

fn recordings_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string());

    if cfg!(target_os = "windows") {
        let path = format!("{}\\Videos\\PhoneMirror", home);
        std::fs::create_dir_all(&path).ok();
        path
    } else if cfg!(target_os = "macos") {
        format!("{}/Movies", home)
    } else {
        let path = format!("{}/Videos/PhoneMirror", home);
        std::fs::create_dir_all(&path).ok();
        path
    }
}

// ─── Safe command execution ────────────────────────────────
// All commands use Command::new() with .args() to avoid shell injection.

fn adb_output(args: &[&str]) -> String {
    let adb = adb_path();
    Command::new(&adb)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn adb_run(args: &[&str]) -> bool {
    let adb = adb_path();
    Command::new(&adb).args(args).spawn().is_ok()
}

fn notify(title: &str, body: &str) {
    if cfg!(target_os = "macos") {
        // Escape double-quotes in body for AppleScript safety
        let safe_body = body.replace('"', "\\\"");
        let safe_title = title.replace('"', "\\\"");
        let script = format!("display notification \"{}\" with title \"{}\"", safe_body, safe_title);
        let _ = Command::new("osascript").args(["-e", &script]).spawn();
    } else if cfg!(target_os = "linux") {
        let _ = Command::new("notify-send").args([title, body]).spawn();
    }
    // Windows: print for now (can add toast later)
    println!("[{}] {}", title, body);
}

// ─── Core actions ──────────────────────────────────────────

fn check_device() -> (bool, String) {
    let output = adb_output(&["devices"]);
    for line in output.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.contains("device") && !trimmed.contains("daemon") && !trimmed.is_empty() {
            let id = trimmed.split_whitespace().next().unwrap_or("unknown").to_string();
            return (true, id);
        }
    }
    (false, "No device".to_string())
}

fn take_screenshot() {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let dir = screenshots_dir();
    let sep = if cfg!(target_os = "windows") { "\\" } else { "/" };
    let filename = format!("{}{}phone_screenshot_{}.png", dir, sep, timestamp);
    let remote_path = "/sdcard/phone_screenshot.png";

    adb_run(&["shell", "screencap", "-p", remote_path]);
    adb_run(&["pull", remote_path, &filename]);
    adb_run(&["shell", "rm", remote_path]);
    notify("📸 Screenshot Saved", &filename);
}

fn start_mirror() {
    let scrcpy = scrcpy_path();
    if cfg!(target_os = "macos") {
        let _ = Command::new(&scrcpy)
            .arg("--shortcut-mod=lctrl")
            .spawn();
    } else {
        let _ = Command::new(&scrcpy).spawn();
    }
    notify("📱 Mirror Started", "scrcpy window opened");
}

fn start_recording() {
    let scrcpy = scrcpy_path();
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let dir = recordings_dir();
    let sep = if cfg!(target_os = "windows") { "\\" } else { "/" };
    let filename = format!("{}{}phone_recording_{}.mp4", dir, sep, timestamp);

    let _ = Command::new(&scrcpy)
        .args(["--record", &filename, "--no-playback", "--no-audio", "--no-window"])
        .spawn();
    notify("🎬 Recording Started", &format!("Saving to: {}", filename));
}

fn close_mirror() {
    if cfg!(target_os = "windows") {
        let _ = Command::new("taskkill").args(["/F", "/IM", "scrcpy.exe"]).spawn();
    } else {
        let _ = Command::new("pkill").args(["-x", "scrcpy"]).spawn();
    }
    notify("📵 Mirror Closed", "scrcpy window closed");
}

// ─── Main ───────────────────────────────────────────────────

fn main() {
    #[cfg(target_os = "macos")]
    {
        use tray_item::{TrayItem, IconSource};

        let icon_data: Vec<u8> = create_phone_icon();
        let icon = IconSource::Data { height: 16, width: 16, data: icon_data };

        let mut tray = TrayItem::new("PhoneMirror", icon)
            .expect("Failed to create tray item");

        tray.add_menu_item("📱 Mirror Screen", || { start_mirror(); }).unwrap();
        tray.add_menu_item("📸 Screenshot", || { take_screenshot(); }).unwrap();
        tray.add_menu_item("🎬 Record Video", || { start_recording(); }).unwrap();
        tray.add_menu_item("📵 Close Mirror", || { close_mirror(); }).unwrap();
        tray.add_label("").unwrap();
        tray.add_menu_item("🔄 Refresh", || {
            let (connected, id) = check_device();
            println!("Device: {} ({})", id, if connected { "connected" } else { "disconnected" });
        }).unwrap();
        tray.add_menu_item("Quit PhoneMirror", || {
            close_mirror();
            std::process::exit(0);
        }).unwrap();

        std::thread::spawn(|| {
            let mut was_connected = false;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let (connected, _) = check_device();
                if connected && !was_connected {
                    notify("PhoneMirror", "📱 Phone connected!");
                } else if !connected && was_connected {
                    notify("PhoneMirror", "📵 Phone disconnected");
                }
                was_connected = connected;
            }
        });

        tray.inner_mut().display();
    }

    #[cfg(target_os = "windows")]
    {
        use tray_item::{TrayItem, IconSource};

        let icon = IconSource::Resource("");
        let mut tray = TrayItem::new("PhoneMirror", icon)
            .expect("Failed to create tray item");

        tray.add_menu_item("📱 Mirror Screen", || { start_mirror(); }).unwrap();
        tray.add_menu_item("📸 Screenshot", || { take_screenshot(); }).unwrap();
        tray.add_menu_item("🎬 Record Video", || { start_recording(); }).unwrap();
        tray.add_menu_item("📵 Close Mirror", || { close_mirror(); }).unwrap();
        tray.add_separator().unwrap();
        tray.add_menu_item("🔄 Refresh", || {
            let (connected, id) = check_device();
            println!("Device: {} ({})", id, if connected { "connected" } else { "disconnected" });
        }).unwrap();
        tray.add_menu_item("Quit PhoneMirror", || {
            close_mirror();
            std::process::exit(0);
        }).unwrap();

        std::thread::spawn(|| {
            let mut was_connected = false;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let (connected, _) = check_device();
                if connected && !was_connected {
                    notify("PhoneMirror", "📱 Phone connected!");
                } else if !connected && was_connected {
                    notify("PhoneMirror", "📵 Phone disconnected");
                }
                was_connected = connected;
            }
        });

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    #[cfg(target_os = "linux")]
    {
        use tray_item::{TrayItem, IconSource};

        let icon_data: Vec<u8> = create_phone_icon();
        let icon = IconSource::Data { height: 16, width: 16, data: icon_data };

        let mut tray = TrayItem::new("PhoneMirror", icon)
            .expect("Failed to create tray item");

        tray.add_menu_item("📱 Mirror Screen", || { start_mirror(); }).unwrap();
        tray.add_menu_item("📸 Screenshot", || { take_screenshot(); }).unwrap();
        tray.add_menu_item("🎬 Record Video", || { start_recording(); }).unwrap();
        tray.add_menu_item("📵 Close Mirror", || { close_mirror(); }).unwrap();
        tray.add_separator().unwrap();
        tray.add_menu_item("🔄 Refresh", || {
            let (connected, id) = check_device();
            println!("Device: {} ({})", id, if connected { "connected" } else { "disconnected" });
        }).unwrap();
        tray.add_menu_item("Quit PhoneMirror", || {
            close_mirror();
            std::process::exit(0);
        }).unwrap();

        std::thread::spawn(|| {
            let mut was_connected = false;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let (connected, _) = check_device();
                if connected && !was_connected {
                    notify("PhoneMirror", "📱 Phone connected!");
                } else if !connected && was_connected {
                    notify("PhoneMirror", "📵 Phone disconnected");
                }
                was_connected = connected;
            }
        });

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}

/// Create a simple 16x16 RGBA phone icon
fn create_phone_icon() -> Vec<u8> {
    let mut data = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16u8 {
        for x in 0..16u8 {
            let on_border = (3..=12).contains(&x) && (1..=14).contains(&y);
            let on_screen = (4..=11).contains(&x) && (3..=12).contains(&y);
            let on_home = (6..=9).contains(&x) && y == 13;

            if on_screen {
                data.extend_from_slice(&[200, 230, 255, 255]);
            } else if on_home {
                data.extend_from_slice(&[180, 200, 220, 255]);
            } else if on_border {
                data.extend_from_slice(&[30, 120, 220, 255]);
            } else {
                data.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    data
}