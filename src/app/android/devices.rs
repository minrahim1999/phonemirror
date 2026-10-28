//! Device inventory and emulator lifecycle. Call blocking APIs from a background worker.
use crate::app::wireless::{Adb, LocalAdb};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Emulator,
    Usb,
    Wifi,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Online,
    Offline,
    Unauthorized,
    Stopped,
}
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub serial: Option<String>,
    pub avd: Option<String>,
    pub kind: Kind,
    pub state: State,
}
#[derive(Default)]
pub struct Inventory {
    pub devices: Vec<Device>,
    pub emulator_path: Option<String>,
    pub emulator_health: String,
    pub adb_health: String,
}

/// Blocking: the supplied ADB implementation must bound each command (LocalAdb: 20s).
/// SDK enumeration failure does not hide attached transports.
pub fn discover(adb: &mut impl Adb) -> Result<Inventory, String> {
    let env = crate::app::build_full_env();
    let path = emulator_path(&env);
    let listing = match path.as_ref() {
        Some(path) => {
            let mut runner = LocalAdb {
                path: path.clone(), env,
                cancel: Arc::new(AtomicBool::new(false)),
            };
            runner.run(&["-list-avds"], None).map_err(|e|
                format!("Cannot list Android virtual devices with {path} (20s limit): {e}. Check Android Studio Device Manager and the SDK emulator installation."))
        }
        None => Err("Android emulator not found. Install Android Emulator in Android Studio SDK Manager; set ANDROID_HOME or ANDROID_SDK_ROOT, or add the SDK emulator directory to PATH.".into()),
    };
    discover_with(adb, path, listing)
}

fn discover_with(
    adb: &mut impl Adb,
    path: Option<String>,
    listing: Result<String, String>,
) -> Result<Inventory, String> {
    let (text, adb_health) = match adb.run(&["devices", "-l"], None) {
        Ok(text) => (text, String::new()),
        Err(error) => (
            String::new(),
            format!("Cannot discover ADB devices: {error}"),
        ),
    };
    let (avds, health) = match listing {
        Ok(text) => {
            let names = avd_names(&text);
            let health = String::new();
            (names, health)
        }
        Err(e) => (Vec::new(), e),
    };
    let mut devices = transports(&text);
    let mut matched = HashSet::new();
    for device in &mut devices {
        if device.kind != Kind::Emulator || device.state != State::Online {
            continue;
        }
        let serial = device.serial.as_deref().unwrap();
        if let Ok(text) = adb.run(&["-s", serial, "emu", "avd", "name"], None) {
            if let Some(name) = resolved_avd(&text) {
                device.name = name.clone();
                device.avd = Some(name.clone());
                // Never collapse distinct running transports, even if their reported AVD agrees.
                if matched.insert(name.clone()) {
                    device.id = format!("avd:{name}");
                }
            }
        }
    }
    for name in avds {
        if matched.insert(name.clone()) {
            devices.push(Device {
                id: format!("avd:{name}"),
                name: name.clone(),
                serial: None,
                avd: Some(name),
                kind: Kind::Emulator,
                state: State::Stopped,
            });
        }
    }
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Inventory {
        devices,
        emulator_path: path,
        emulator_health: health,
        adb_health,
    })
}

fn valid_avd(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}
fn avd_names(text: &str) -> Vec<String> {
    let mut names: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|s| valid_avd(s))
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    names
}
fn resolved_avd(text: &str) -> Option<String> {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "OK");
    let name = lines.next()?;
    (valid_avd(name) && lines.next().is_none()).then(|| name.to_owned())
}
fn emulator_serial(serial: &str) -> bool {
    serial
        .strip_prefix("emulator-")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}
fn transports(text: &str) -> Vec<Device> {
    let mut seen = HashSet::new();
    text.lines()
        .filter_map(|line| {
            // Serial names from mDNS may contain spaces. Locate the state boundary
            // rather than truncating the serial at the first whitespace.
            let line = line.trim();
            let (serial, rest) = if let Some(parts) = line.split_once('\t') {
                parts
            } else {
                let boundary = [" device", " offline", " unauthorized"]
                    .iter()
                    .filter_map(|state| {
                        line.find(state).filter(|&i| {
                            line[i + state.len()..]
                                .chars()
                                .next()
                                .is_none_or(char::is_whitespace)
                        })
                    })
                    .min()?;
                (&line[..boundary], &line[boundary..])
            };
            let serial = serial.trim();
            if serial.is_empty() {
                return None;
            }
            let mut fields = rest.split_whitespace();
            let state = match fields.next()? {
                "device" => State::Online,
                "offline" => State::Offline,
                "unauthorized" => State::Unauthorized,
                _ => return None,
            };
            if !seen.insert(serial.to_owned()) {
                return None;
            }
            let kind = if emulator_serial(serial) {
                Kind::Emulator
            } else if serial.parse::<SocketAddr>().is_ok()
                || serial.contains("._adb-tls-connect._tcp")
            {
                Kind::Wifi
            } else {
                Kind::Usb
            };
            let model = fields
                .find_map(|s| s.strip_prefix("model:"))
                .filter(|s| !s.is_empty());
            Some(Device {
                id: format!("adb:{serial}"),
                name: model
                    .map(|s| s.replace('_', " "))
                    .unwrap_or_else(|| serial.to_owned()),
                serial: Some(serial.to_owned()),
                avd: None,
                kind,
                state,
            })
        })
        .collect()
}
fn executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}
fn emulator_path(env: &[(String, String)]) -> Option<String> {
    let get = |key: &str| env.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    let binary = if cfg!(windows) {
        "emulator.exe"
    } else {
        "emulator"
    };
    let mut candidates = Vec::new();
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(root) = get(key).filter(|s| !s.is_empty()) {
            candidates.push(PathBuf::from(root).join("emulator").join(binary));
        }
    }
    if let Some(home) = get("HOME") {
        for sdk in ["Library/Android/sdk", "Android/Sdk"] {
            candidates.push(PathBuf::from(home).join(sdk).join("emulator").join(binary));
        }
    }
    if let Some(local) = get("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Android/Sdk/emulator/emulator.exe"));
    }
    if let Some(path) = get("PATH") {
        candidates.extend(std::env::split_paths(path).map(|p| p.join(binary)));
    }
    candidates
        .into_iter()
        .find(|p| executable(p))
        .map(|p| p.to_string_lossy().into_owned())
}

/// Blocking startup probe (500ms), not proof of completed Android boot.
/// After success a dedicated waiter retains and reaps the emulator child.
pub fn launch(emulator_path: &str, avd: &str) -> Result<(), String> {
    if !valid_avd(avd) {
        return Err("Invalid AVD name. Select a registered AVD name containing only letters, digits, dots, underscores or hyphens, not an option.".into());
    }
    if emulator_path.trim().is_empty() {
        return Err(
            "Android emulator path is empty. Install or select the SDK emulator executable.".into(),
        );
    }
    let mut child = Command::new(emulator_path)
        .args(["-avd", avd])
        .envs(crate::app::build_full_env())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            format!(
                "Cannot launch AVD {avd}: {e}. Check the emulator executable and SDK installation."
            )
        })?;
    std::thread::sleep(Duration::from_millis(500));
    match child.try_wait() {
        Ok(Some(status)) => Err(format!("AVD {avd} exited immediately ({status}). Check Android Studio Device Manager, its system image, virtualization support and whether it is already running.")),
        Ok(None) => { std::thread::spawn(move || { let _ = child.wait(); }); Ok(()) }
        Err(e) => { let _ = child.kill(); let _ = child.wait(); Err(format!("Cannot check startup of AVD {avd}: {e}")) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        devices: &'static str,
        names: Vec<(&'static str, &'static str)>,
        calls: Vec<Vec<String>>,
    }
    impl Adb for Fake {
        fn run(&mut self, args: &[&str], _: Option<&str>) -> Result<String, String> {
            self.calls
                .push(args.iter().map(|s| s.to_string()).collect());
            if args == ["devices", "-l"] {
                return Ok(self.devices.into());
            }
            assert_eq!(args.len(), 5);
            assert_eq!(&args[2..], &["emu", "avd", "name"]);
            self.names
                .iter()
                .find(|(s, _)| *s == args[1])
                .map(|(_, n)| n.to_string())
                .ok_or("unresolved".into())
        }
        fn pause(&self) -> Result<(), String> {
            Ok(())
        }
    }
    #[test]
    fn multiple_phones_and_all_states_and_transports() {
        let d = transports(
            "List of devices attached
A device model:Pixel_9
B unauthorized usb:1
C offline
192.168.1.4:4444 device model:Galaxy_S24
[::1]:5555 offline
adb-X._adb-tls-connect._tcp device
emulator-no device
emulator-5554 offline
A device
",
        );
        assert_eq!(d.len(), 8);
        assert_eq!(d[0].name, "Pixel 9");
        assert_eq!(d[1].state, State::Unauthorized);
        assert_eq!(d[2].state, State::Offline);
        assert!(d[3..6].iter().all(|d| d.kind == Kind::Wifi));
        assert_eq!(d[6].kind, Kind::Usb);
        assert_eq!(d[7].kind, Kind::Emulator);
    }
    #[test]
    fn merge_only_resolved_online_emulators_and_keep_stopped() {
        let mut adb = Fake {
            devices: "phone device model:Phone
emulator-5554 device
emulator-5556 device
emulator-5558 offline
emulator-5560 unauthorized",
            names: vec![
                (
                    "emulator-5554",
                    "Pixel_API_35
OK",
                ),
                (
                    "emulator-5556",
                    "ERROR: unavailable
OK",
                ),
            ],
            calls: vec![],
        };
        let inv = discover_with(
            &mut adb,
            Some("emulator".into()),
            Ok("Pixel_API_35
Tablet
Tablet
"
            .into()),
        )
        .unwrap();
        assert_eq!(inv.devices.len(), 6);
        let pixel = inv
            .devices
            .iter()
            .find(|d| d.id == "avd:Pixel_API_35")
            .unwrap();
        assert_eq!(pixel.serial.as_deref(), Some("emulator-5554"));
        assert_eq!(pixel.name, "Pixel_API_35");
        assert_eq!(
            inv.devices
                .iter()
                .find(|d| d.id == "avd:Tablet")
                .unwrap()
                .state,
            State::Stopped
        );
        assert_eq!(
            inv.devices
                .iter()
                .find(|d| d.id == "adb:emulator-5556")
                .unwrap()
                .avd,
            None
        );
        assert_eq!(adb.calls.len(), 3);
    }
    #[test]
    fn missing_sdk_retains_physical_and_unknown_emulator() {
        let mut adb = Fake {
            devices: "one unauthorized
emulator-5554 device",
            names: vec![],
            calls: vec![],
        };
        let inv = discover_with(&mut adb, None, Err("Install emulator".into())).unwrap();
        assert_eq!(inv.devices.len(), 2);
        assert_eq!(inv.emulator_health, "Install emulator");
        assert!(inv.devices.iter().all(|d| d.id.starts_with("adb:")));
    }
    #[test]
    fn mdns_serial_with_spaces_is_not_truncated() {
        let serial = "adb-RRCX107DMRP-ziYyJB (2)._adb-tls-connect._tcp";
        for separator in ["\t", " "] {
            let devices = transports(&format!(
                "{serial}{separator}device product:p model:Pixel_9 transport_id:3"
            ));
            assert_eq!(devices.len(), 1);
            assert_eq!(devices[0].serial.as_deref(), Some(serial));
            assert_eq!(devices[0].kind, Kind::Wifi);
            assert_eq!(devices[0].name, "Pixel 9");
        }
    }

    #[test]
    fn adb_failure_does_not_hide_installed_emulators() {
        struct Failed;
        impl Adb for Failed {
            fn run(&mut self, _: &[&str], _: Option<&str>) -> Result<String, String> {
                Err("ADB unavailable".into())
            }
            fn pause(&self) -> Result<(), String> {
                Ok(())
            }
        }
        let inventory = discover_with(
            &mut Failed,
            Some("emulator".into()),
            Ok("Pixel_API_35".into()),
        )
        .unwrap();
        assert_eq!(inventory.devices.len(), 1);
        assert_eq!(inventory.devices[0].state, State::Stopped);
        assert!(inventory.adb_health.contains("ADB unavailable"));
    }

    #[test]
    fn strict_names_and_duplicate_running_identity() {
        for name in [
            "",
            "-wipe-data",
            "a b",
            "x
y",
            "../x",
        ] {
            assert!(!valid_avd(name));
        }
        assert_eq!(
            resolved_avd(
                "Pixel
OK
"
            ),
            Some("Pixel".into())
        );
        assert_eq!(
            resolved_avd(
                "Pixel
Other
OK"
            ),
            None
        );
        let mut adb = Fake {
            devices: "emulator-5554 device
emulator-5556 device",
            names: vec![("emulator-5554", "Pixel"), ("emulator-5556", "Pixel")],
            calls: vec![],
        };
        let inv = discover_with(&mut adb, None, Ok("Pixel".into())).unwrap();
        assert_eq!(inv.devices.len(), 2);
        assert_ne!(inv.devices[0].id, inv.devices[1].id);
    }
}
