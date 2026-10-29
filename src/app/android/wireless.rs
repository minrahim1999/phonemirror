//! Android 11+ TLS pairing. Pairing and connection endpoints are deliberately separate.
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct Service {
    pub name: String,
    pub kind: String,
    pub endpoint: String,
}

pub fn endpoint(value: &str) -> Result<String, String> {
    let address: SocketAddr = value.trim().parse().map_err(|_| {
        "Enter an IP address and port, e.g. 192.168.1.20:37123 (IPv6: [::1]:37123).".to_string()
    })?;
    if address.port() == 0 || address.ip().is_unspecified() || address.ip().is_multicast() {
        return Err("Use the phone's IP address and a port from 1 to 65535.".into());
    }
    Ok(address.to_string())
}

pub fn pairing_code(value: &str) -> Result<String, String> {
    let code = value.trim();
    if code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit()) {
        Ok(code.into())
    } else {
        Err("Enter the six-digit code from 'Pair device with pairing code'.".into())
    }
}

pub fn services(text: &str) -> Vec<Service> {
    text.lines()
        .filter_map(|line| {
            // Service names can contain spaces (e.g. duplicate-name suffixes).
            // AOSP emits instance<TAB>type<TAB>address. Parse from the right:
            // instance names may contain spaces or even "_adb-tls-" themselves.
            let (prefix, address) = line.trim().rsplit_once(char::is_whitespace)?;
            let address = endpoint(address).ok()?;
            let (name, kind) = prefix.trim_end().rsplit_once(char::is_whitespace)?;
            let kind = kind.trim_end_matches('.');
            let name = name.trim().to_string();
            if name.is_empty() {
                return None;
            }
            if kind != "_adb-tls-pairing._tcp" && kind != "_adb-tls-connect._tcp" {
                return None;
            }
            Some(Service {
                name,
                kind: kind.into(),
                endpoint: address,
            })
        })
        .collect()
}

pub struct QrSession {
    pub name: String,
    pub password: String,
    pub code: qrcode::QrCode,
}

impl QrSession {
    pub fn new() -> Result<Self, String> {
        let mut random = [0u8; 24];
        getrandom::fill(&mut random)
            .map_err(|_| "Secure random generation failed. Try pairing by code.".to_string())?;
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let name = format!("studio-{}", hex(&random[..8]));
        let password = hex(&random[8..]);
        let code = qrcode::QrCode::new(qr_payload(&name, &password))
            .map_err(|_| "Cannot generate pairing QR code.".to_string())?;
        Ok(Self {
            name,
            password,
            code,
        })
    }
}

fn qr_payload(name: &str, password: &str) -> String {
    format!("WIFI:T:ADB;S:{name};P:{password};;")
}

pub trait Adb {
    fn run(&mut self, args: &[&str], secret: Option<&str>) -> Result<String, String>;
    fn pause(&self) -> Result<(), String>;
}

pub struct LocalAdb {
    pub path: String,
    pub env: Vec<(String, String)>,
    pub cancel: Arc<AtomicBool>,
}

impl LocalAdb {
    fn check_cancel(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("Wireless operation cancelled.".into())
        } else {
            Ok(())
        }
    }
}

impl Adb for LocalAdb {
    fn run(&mut self, args: &[&str], secret: Option<&str>) -> Result<String, String> {
        self.check_cancel()?;
        // Observe only new bytes for this attempt, never historical log failures.
        // server-status is host-local; no phone command is sent by this query.
        let pairing_address = if args.first() == Some(&"pair") {
            args.get(1).copied()
        } else {
            None
        };
        let mut pairing_log = pairing_address.and_then(|_| {
            let status = self.run(&["server-status"], None).ok()?;
            open_pairing_log(&status)
        });
        let mut child = Command::new(&self.path)
            .args(args)
            .envs(self.env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                format!("Cannot start ADB: {e}. Install current Android SDK Platform-Tools.")
            })?;
        // Drain both pipes concurrently so ADB cannot deadlock on a full pipe.
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let read = |pipe: Box<dyn Read + Send>| {
            std::thread::spawn(move || {
                let mut data = String::new();
                let _ = pipe.take(65536).read_to_string(&mut data);
                data
            })
        };
        let out = read(Box::new(stdout));
        let err = read(Box::new(stderr));
        if let Some(mut stdin) = child.stdin.take() {
            if let Some(secret) = secret {
                let _ = writeln!(stdin, "{secret}");
            }
        }
        let start = Instant::now();
        let status = loop {
            if let Err(message) = self.check_cancel() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(message);
            }
            if start.elapsed() > Duration::from_secs(20) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("ADB timed out. Check the phone's current IP/port, keep the pairing dialog open, disable VPN, and check Wi-Fi client isolation/firewall.".into());
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("Cannot wait for ADB: {e}"));
                }
            }
        };
        let mut text = format!(
            "{}
{}",
            out.join().unwrap_or_default(),
            err.join().unwrap_or_default()
        )
        .trim()
        .to_string();
        if let (Some(address), Some(log)) = (
            pairing_address.filter(|_| {
                !status.success() || text.contains("protocol fault") || text.contains("Failed:")
            }),
            pairing_log.as_mut(),
        ) {
            let mut recent = String::new();
            let _ = log.take(65536).read_to_string(&mut recent);
            if let Some(reason) = pairing_network_reason(&recent, address) {
                text.push_str(&format!("\nADB server reported for this pairing attempt: {reason}. This failed before code verification. Check the current phone IP/port, local-network permissions and Wi-Fi client isolation."));
            }
        }
        if let Some(secret) = secret {
            text = text.replace(secret, "[redacted]");
        }
        if !status.success() {
            return Err(format!("ADB failed: {text}. Check the current port/code and update Platform-Tools if pairing is unsupported."));
        }
        Ok(text)
    }

    fn pause(&self) -> Result<(), String> {
        for _ in 0..10 {
            self.check_cancel()?;
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }
}

/// Return an already-open regular log positioned at its current end. Symlinks,
/// non-absolute paths and non-ADB log names are ignored; diagnostics are optional.
fn open_pairing_log(status: &str) -> Option<std::fs::File> {
    let raw = status
        .lines()
        .find_map(|line| line.trim().strip_prefix("log_absolute_path: "))?;
    let path = std::path::Path::new(raw.trim().trim_matches('"'));
    let name = path.file_name()?.to_str()?;
    if !path.is_absolute() || !name.starts_with("adb.") || !name.ends_with(".log") {
        return None;
    }
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    file.seek(SeekFrom::End(0)).ok()?;
    Some(file)
}

/// Expose a fixed reason only, never raw logs, secrets, or other device addresses.
fn pairing_network_reason(recent: &str, address: &str) -> Option<&'static str> {
    let marker = format!("failed to connect to '{address}': ");
    recent
        .lines()
        .filter_map(|line| line.split_once(&marker).map(|(_, reason)| reason))
        .find_map(|reason| {
            if reason.starts_with("No route to host") {
                Some("No route to host")
            } else if reason.starts_with("Connection refused") {
                Some("Connection refused")
            } else if reason.starts_with("Operation timed out") {
                Some("Operation timed out")
            } else if reason.starts_with("Permission denied") {
                Some("Permission denied")
            } else {
                None
            }
        })
}

/// Disconnect exactly one wireless transport, preserving pairing and other connections.
pub fn disconnect(adb: &mut impl Adb, serial: &str) -> Result<String, String> {
    let mdns = serial
        .strip_suffix("._adb-tls-connect._tcp")
        .or_else(|| serial.strip_suffix("._adb-tls-connect._tcp."));
    if endpoint(serial).is_err()
        && !mdns.is_some_and(|name| name.starts_with("adb-") && name.len() > 4)
    {
        return Err("Select a wireless IP address or ADB connection service to disconnect.".into());
    }
    if serial.contains(['\n', '\r', '\0']) {
        return Err("Invalid wireless connection ID.".into());
    }
    let reply = adb.run(&["disconnect", serial], None)?;
    if !reply
        .lines()
        .any(|line| line.trim() == format!("disconnected {serial}"))
    {
        return Err(format!("ADB did not confirm disconnection: {reply}"));
    }
    Ok(format!("Disconnected {serial}. Pairing is preserved. ADB discovery may reconnect this service automatically."))
}

/// Discover connection endpoints only; never pair or connect an arbitrary phone.
pub fn discover_connections(adb: &mut impl Adb) -> Result<Vec<Service>, String> {
    let text = adb.run(&["mdns", "services"], None)?;
    let mut found = services(&text);
    found.retain(|service| service.kind == "_adb-tls-connect._tcp");
    found.sort_by(|a, b| a.endpoint.cmp(&b.endpoint).then(a.name.cmp(&b.name)));
    found.dedup_by(|a, b| a.endpoint == b.endpoint);
    Ok(found)
}

pub fn connect(adb: &mut impl Adb, address: &str) -> Result<String, String> {
    let address = endpoint(address)?;
    let text = adb.run(&["connect", &address], None)?;
    // adb connect can exit zero even when the connection failed.
    if !text.lines().any(|line| {
        let line = line.trim();
        line == format!("connected to {address}")
            || line == format!("already connected to {address}")
    }) {
        return Err(format!("Connection failed: {text}. Use the IP & port on the main Wireless debugging page, NOT the pairing dialog. Check VPN, firewall, and guest Wi-Fi isolation."));
    }
    let state = adb.run(&["-s", &address, "get-state"], None)?;
    if state.trim() != "device" {
        return Err(format!("ADB connected but device is not ready: {state}"));
    }
    Ok(format!("Wireless device connected at {address}. You can now start mirroring. In VS Code, reload the window and select the IP-address device if the service-name device appears unsupported."))
}

pub fn pair(adb: &mut impl Adb, address: &str, password: &str) -> Result<String, String> {
    let address = endpoint(address)?;
    let text = adb.run(&["pair", &address], Some(password))?;
    // ADB prints the stdin prompt without a newline, and failures may contain
    // the words "Successfully paired". Only accept an actual success response.
    let response = text
        .trim()
        .strip_prefix("Enter pairing code: ")
        .unwrap_or(text.trim());
    if !response.starts_with("Successfully paired to ") {
        return Err(format!(
            "Pairing failed: {text}. Open a fresh pairing dialog and use its port and code."
        ));
    }
    // Auto-connect only a TLS connection service on the paired phone's IP.
    // Never reuse the pairing port or blindly connect every discovered phone.
    for _ in 0..5 {
        let discovered = match adb.run(&["mdns", "services"], None) {
            Ok(text) => services(&text),
            Err(_) => break,
        };
        let host = address.parse::<SocketAddr>().unwrap().ip();
        let candidates: Vec<_> = discovered
            .iter()
            .filter(|s| {
                s.kind == "_adb-tls-connect._tcp"
                    && s.endpoint.parse::<SocketAddr>().unwrap().ip() == host
            })
            .collect();
        if candidates.len() == 1 {
            return Ok(match connect(adb, &candidates[0].endpoint) {
                Ok(message) => message,
                Err(error) => format!("Paired successfully, but not connected. {error}"),
            });
        }
        adb.pause()?;
    }
    Ok("Paired successfully. If the device does not appear, close the phone's pairing dialog and enter the IP & port from the main Wireless debugging page in Connect (a different port).".into())
}

pub fn pair_qr(adb: &mut impl Adb, name: &str, password: &str) -> Result<String, String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(120) {
        let text = adb.run(&["mdns", "services"], None)?;
        let matches: Vec<_> = services(&text)
            .into_iter()
            .filter(|s| s.kind == "_adb-tls-pairing._tcp" && s.name == name)
            .collect();
        if matches.len() == 1 {
            return pair(adb, &matches[0].endpoint, password);
        }
        adb.pause()?;
    }
    Err("QR pairing timed out: no matching phone discovered. Scan using Wireless debugging > Pair device with QR code. If multicast discovery is blocked, use Pairing code instead.".into())
}

pub fn diagnose(adb: &mut impl Adb) -> Result<String, String> {
    let version = adb.run(&["version"], None)?;
    let check = adb.run(&["mdns", "check"], None).unwrap_or_else(|e| e);
    let server = adb.run(&["server-status"], None).unwrap_or_else(|e| e);
    let discovery = adb.run(&["mdns", "services"], None)?;
    let count = services(&discovery).len();
    Ok(format!("{version}

ADB server:
{server}

{check}
{count} wireless service(s) found.
{discovery}

Android 11+: enable Developer options > Wireless debugging, accept this Wi-Fi network, and keep the phone unlocked. Pairing code: keep its dialog open. QR: scan this laptop's QR from the phone. Pairing and connection ports differ and can change.

No services? Same Wi-Fi name does not guarantee reachability. Avoid guest Wi-Fi/client isolation, temporarily disconnect VPN, allow ADB/local network access in the firewall, then try a direct IP & port connection after pairing. Manual code pairing does not require mDNS.

Protocol fault / No route to host: pairing can fail before code verification. Check the current phone IP and pairing port, router client isolation, and macOS Privacy & Security > Local Network for the application hosting ADB. Do not disable the firewall globally or delete ADB trust keys. Compare ADB client and server paths above if multiple tool installations are present."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Fake {
        replies: VecDeque<(&'static str, &'static str)>,
        calls: Vec<String>,
    }
    impl Adb for Fake {
        fn run(&mut self, args: &[&str], _secret: Option<&str>) -> Result<String, String> {
            let call = args.join(" ");
            self.calls.push(call.clone());
            let (expected, reply) = self.replies.pop_front().expect("unexpected ADB command");
            assert_eq!(call, expected);
            Ok(reply.into())
        }
        fn pause(&self) -> Result<(), String> {
            Ok(())
        }
    }
    fn fake(replies: Vec<(&'static str, &'static str)>) -> Fake {
        Fake {
            replies: replies.into(),
            calls: vec![],
        }
    }
    #[test]
    fn discovery_lists_only_unique_connection_endpoints_without_connecting() {
        let mut adb = fake(vec![("mdns services", "phone (2)\t_adb-tls-connect._tcp.\t192.168.1.192:36801\nphone\t_adb-tls-connect._tcp\t192.168.1.192:36801\npair\t_adb-tls-pairing._tcp\t192.168.1.192:40000\nother\t_adb-tls-connect._tcp\t[::1]:1234")]);
        let found = discover_connections(&mut adb).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|s| s.endpoint == "192.168.1.192:36801"));
        assert!(found.iter().all(|s| s.kind == "_adb-tls-connect._tcp"));
        assert_eq!(adb.calls, ["mdns services"]);
    }

    #[test]
    fn disconnect_preserves_exact_spaced_id_and_never_disconnects_all() {
        let serial = "adb-phone (2)._adb-tls-connect._tcp";
        let mut adb = fake(vec![(
            "disconnect adb-phone (2)._adb-tls-connect._tcp",
            "disconnected adb-phone (2)._adb-tls-connect._tcp",
        )]);
        assert!(disconnect(&mut adb, serial)
            .unwrap()
            .contains("Pairing is preserved"));
        assert_eq!(adb.calls.len(), 1);
        for invalid in [
            "",
            "usb-serial",
            "192.168.1.20",
            "adb-bad\n._adb-tls-connect._tcp",
        ] {
            assert!(disconnect(&mut adb, invalid).is_err());
        }
        assert_eq!(adb.calls.len(), 1);
    }

    #[test]
    fn disconnect_ip_and_reject_unconfirmed_reply() {
        let mut adb = fake(vec![(
            "disconnect 192.168.1.20:1234",
            "error: no such device",
        )]);
        assert!(disconnect(&mut adb, "192.168.1.20:1234").is_err());
        let mut adb = fake(vec![(
            "disconnect 192.168.1.20:1234",
            "disconnected 192.168.1.20:1234",
        )]);
        assert!(disconnect(&mut adb, "192.168.1.20:1234").is_ok());
    }

    #[test]
    fn discovery_empty_is_not_a_connection_success() {
        let mut adb = fake(vec![("mdns services", "List of discovered mdns services")]);
        assert!(discover_connections(&mut adb).unwrap().is_empty());
    }

    #[test]
    fn pairing_failure_containing_success_words_is_rejected() {
        // This exact failure is emitted by AOSP adb_wifi_pair_device.
        let mut adb = fake(vec![("pair 127.0.0.1:1234", "Enter pairing code: Failed: Successfully paired but server returned unknown response=0")]);
        assert!(pair(&mut adb, "127.0.0.1:1234", "012345").is_err());
        assert_eq!(adb.calls.len(), 1);
    }
    #[test]
    fn service_name_can_contain_service_type_text() {
        let found = services("phone_adb-tls-backup (2)\t_adb-tls-connect._tcp.\t127.0.0.1:4321");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "phone_adb-tls-backup (2)");
        assert_eq!(
            services("  phone (2)   _adb-tls-connect._tcp.   127.0.0.1:4321  ")[0].name,
            "phone (2)"
        );
    }
    #[test]
    fn unknown_response_is_not_a_connection_success() {
        let mut adb = fake(vec![(
            "connect 127.0.0.1:1234",
            "not connected to 127.0.0.1:1234",
        )]);
        assert!(connect(&mut adb, "127.0.0.1:1234").is_err());
        assert_eq!(adb.calls.len(), 1);
    }
    #[test]
    fn pairing_log_reads_new_bytes_not_historical_failures() {
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "phonemirror-log-test-{}-{:x}",
            std::process::id(),
            u64::from_ne_bytes(random)
        ));
        std::fs::create_dir(&directory).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(directory.clone());
        let path = directory.join("adb.fixture.log");
        let address = "192.168.1.20:44907";
        std::fs::write(
            &path,
            format!("failed to connect to '{address}': No route to host\n"),
        )
        .unwrap();
        let status = format!("log_absolute_path: \"{}\"", path.display());
        let mut observed = open_pairing_log(&status).unwrap();
        let mut fresh = String::new();
        observed.read_to_string(&mut fresh).unwrap();
        assert!(
            fresh.is_empty(),
            "historical failures must never be attributed to a fresh attempt"
        );
        let mut writer = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(
            writer,
            "failed to connect to '192.168.1.99:44907': No route to host"
        )
        .unwrap();
        writeln!(
            writer,
            "failed to connect to '{address}': Connection refused"
        )
        .unwrap();
        writer.flush().unwrap();
        observed.read_to_string(&mut fresh).unwrap();
        assert_eq!(
            pairing_network_reason(&fresh, address),
            Some("Connection refused")
        );
        #[cfg(unix)]
        {
            let link = directory.join("adb.link.log");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(
                open_pairing_log(&format!("log_absolute_path: \"{}\"", link.display())).is_none()
            );
        }
    }
    #[test]
    fn pairing_log_exposes_only_exact_endpoint_network_reason() {
        let fresh = "network.cpp:149 failed to connect to '192.168.1.20:44907': No route to host\nprivate unrelated text";
        assert_eq!(
            pairing_network_reason(fresh, "192.168.1.20:44907"),
            Some("No route to host")
        );
        assert_eq!(pairing_network_reason(fresh, "192.168.1.2:44907"), None);
        assert_eq!(pairing_network_reason(fresh, "192.168.1.20:4490"), None);
        assert_eq!(pairing_network_reason("", "192.168.1.20:44907"), None);
        assert_eq!(
            pairing_network_reason(
                "failed to connect to '192.168.1.20:44907': secret-value",
                "192.168.1.20:44907"
            ),
            None
        );
        assert!(open_pairing_log("log_absolute_path: \"relative/adb.1.log\"").is_none());
        assert!(open_pairing_log("log_absolute_path: \"/etc/passwd\"").is_none());
    }
    #[test]
    fn validates_inputs() {
        for input in [
            "",
            "host:123",
            "1.2.3.4",
            "1.2.3.4:0",
            "1.2.3.4:65536",
            "0.0.0.0:5",
            "1.2.3.4:5;rm",
        ] {
            assert!(endpoint(input).is_err(), "{input}");
        }
        assert_eq!(endpoint(" 192.168.1.2:1234 ").unwrap(), "192.168.1.2:1234");
        assert!(endpoint("[fe80::1]:1234").is_ok());
        assert_eq!(pairing_code(" 012345 ").unwrap(), "012345");
        for input in ["12345", "1234567", "abcdef", "１２３４５６"] {
            assert!(pairing_code(input).is_err());
        }
    }
    #[test]
    fn parses_services_with_spaces_and_trailing_dots() {
        let found = services(
            "List of discovered mdns services
adb-phone (2)	_adb-tls-connect._tcp.	192.168.1.2:4321
studio-test	_adb-tls-pairing._tcp	[::1]:1234
bad _adb-tls-connect._tcp bogus
",
        );
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "adb-phone (2)");
        assert_eq!(found[1].kind, "_adb-tls-pairing._tcp");
    }
    #[test]
    fn qr_payload_and_random_sessions() {
        assert_eq!(
            qr_payload("studio-test", "secret"),
            "WIFI:T:ADB;S:studio-test;P:secret;;"
        );
        let a = QrSession::new().unwrap();
        let b = QrSession::new().unwrap();
        assert_ne!(a.name, b.name);
        assert_ne!(a.password, b.password);
        assert!(a.code.width() > 20);
    }
    #[test]
    fn pair_uses_separate_connect_port_and_checks_state() {
        let mut adb = fake(vec![
            (
                "pair 192.168.1.2:1234",
                "Enter pairing code: Successfully paired to 192.168.1.2:1234 [guid=phone]",
            ),
            (
                "mdns services",
                "phone _adb-tls-connect._tcp 192.168.1.2:5678",
            ),
            ("connect 192.168.1.2:5678", "connected to 192.168.1.2:5678"),
            ("-s 192.168.1.2:5678 get-state", "device"),
        ]);
        assert!(pair(&mut adb, "192.168.1.2:1234", "012345")
            .unwrap()
            .contains("connected"));
        assert!(adb.replies.is_empty());
    }
    #[test]
    fn zero_exit_connection_failure_is_not_success() {
        let mut adb = fake(vec![(
            "connect 192.168.1.2:1234",
            "failed to connect to 192.168.1.2:1234",
        )]);
        assert!(connect(&mut adb, "192.168.1.2:1234").is_err());
    }
    #[test]
    fn failed_pair_does_not_connect() {
        let mut adb = fake(vec![("pair 192.168.1.2:1234", "Failed: wrong password")]);
        assert!(pair(&mut adb, "192.168.1.2:1234", "012345").is_err());
        assert_eq!(adb.calls.len(), 1);
    }
    #[test]
    fn missing_discovery_reports_paired_not_connected() {
        let mut replies = vec![(
            "pair 192.168.1.2:1234",
            "Successfully paired to 192.168.1.2:1234",
        )];
        replies.extend(vec![
            (
                "mdns services",
                "other _adb-tls-connect._tcp 192.168.1.99:5678"
            );
            5
        ]);
        let mut adb = fake(replies);
        assert!(pair(&mut adb, "192.168.1.2:1234", "012345")
            .unwrap()
            .starts_with("Paired successfully."));
        assert!(!adb.calls.iter().any(|c| c.starts_with("connect")));
    }
    #[test]
    fn qr_selects_only_exact_session_name() {
        let mut adb = fake(vec![
            (
                "mdns services",
                "studio-other _adb-tls-pairing._tcp 192.168.1.99:1111
studio-mine _adb-tls-pairing._tcp 192.168.1.2:1234",
            ),
            ("pair 192.168.1.2:1234", "wrong password"),
        ]);
        assert!(pair_qr(&mut adb, "studio-mine", "secret").is_err());
        assert!(adb.replies.is_empty());
    }
    #[test]
    #[cfg(unix)]
    fn runner_supplies_secret_via_stdin_and_redacts_output() {
        let mut adb = LocalAdb {
            path: "/bin/sh".into(),
            env: vec![],
            cancel: Arc::new(AtomicBool::new(false)),
        };
        let output = adb
            .run(
                &[
                    "-c",
                    r#"read password; printf '%s' "$password"; printf 'fixture error' >&2; exit 1"#,
                ],
                Some("012345"),
            )
            .unwrap_err();
        assert!(output.contains("[redacted]"));
        assert!(output.contains("fixture error"));
        assert!(!output.contains("012345"));
    }
    #[test]
    #[cfg(unix)]
    fn runner_cancels_inflight_process() {
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker = std::thread::spawn(move || {
            let mut adb = LocalAdb {
                path: "/bin/sh".into(),
                env: vec![],
                cancel: worker_cancel,
            };
            adb.run(&["-c", "exec /bin/sleep 30"], None)
        });
        std::thread::sleep(Duration::from_millis(100));
        cancel.store(true, Ordering::Relaxed);
        assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
    }
    #[test]
    fn cancelled_runner_never_spawns() {
        let mut adb = LocalAdb {
            path: "nonexistent".into(),
            env: vec![],
            cancel: Arc::new(AtomicBool::new(true)),
        };
        assert!(adb
            .run(&["version"], None)
            .unwrap_err()
            .contains("cancelled"));
    }
}
