//! Embedded scrcpy 4.1 transport. All process and socket work belongs to the worker.
//! Wire reference: Genymobile/scrcpy v4.1 DesktopConnection.java and control_msg.c.
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct Config {
    pub serial: String,
    pub adb_path: String,
    pub scrcpy_path: String,
    pub env: Vec<(String, String)>,
}
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Starting,
    Streaming,
    Stopped,
    Failed(String),
}
pub enum Input {
    Touch {
        action: u8,
        x: u32,
        y: u32,
        width: u16,
        height: u16,
    },
    Key {
        code: u32,
        down: bool,
    },
    Text(String),
}
struct Shared {
    done: AtomicBool,
    cancel: AtomicBool,
    status: Mutex<Status>,
    frame: Mutex<Option<Frame>>,
    ctx: egui::Context,
}
impl Shared {
    fn status(&self, status: Status) {
        let mut current = self.status.lock().unwrap();
        if !(matches!(*current, Status::Failed(_))
            || self.cancel.load(Ordering::Relaxed) && status == Status::Streaming)
        {
            *current = status;
        }
        self.ctx.request_repaint();
    }
    fn fail(&self, error: String) {
        self.status(Status::Failed(error));
        self.cancel.store(true, Ordering::Relaxed);
    }
}
pub struct Session {
    serial: String,
    shared: Arc<Shared>,
    input: SyncSender<Vec<u8>>,
}
impl Session {
    pub fn start(config: Config, ctx: egui::Context) -> Self {
        let shared = Arc::new(Shared {
            done: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            status: Mutex::new(Status::Starting),
            frame: Mutex::new(None),
            ctx,
        });
        let (input, rx) = mpsc::sync_channel(128);
        let session = Self {
            serial: config.serial.clone(),
            shared: shared.clone(),
            input,
        };
        let worker = shared.clone();
        if let Err(e) = thread::Builder::new()
            .name("mirror-session".into())
            .spawn(move || {
                let mut resources = Resources::new(&config);
                let result = run(&config, &worker, rx, &mut resources);
                if let Err(error) = result {
                    if !worker.cancel.load(Ordering::Relaxed) {
                        worker.fail(format!("{error}{}", resources.diagnostics()));
                    }
                }
                worker.cancel.store(true, Ordering::Relaxed);
                resources.cleanup(&config);
                worker.done.store(true, Ordering::Release);
                if !matches!(*worker.status.lock().unwrap(), Status::Failed(_)) {
                    worker.status(Status::Stopped);
                }
            })
        {
            shared.fail(format!("Cannot start mirror worker: {e}"));
            shared.done.store(true, Ordering::Release);
        }
        session
    }
    pub fn serial(&self) -> &str {
        &self.serial
    }
    pub fn status(&self) -> Status {
        self.shared.status.lock().unwrap().clone()
    }
    pub fn take_frame(&self) -> Option<Frame> {
        if self.shared.cancel.load(Ordering::Relaxed) {
            return None;
        }
        self.shared.frame.lock().unwrap().take()
    }
    pub fn send(&self, input: Input) -> bool {
        if self.shared.cancel.load(Ordering::Relaxed) || self.status() != Status::Streaming {
            return false;
        }
        let Some(bytes) = encode(input) else {
            return false;
        };
        if self.input.try_send(bytes).is_err() {
            self.shared.fail("Mirror control queue overflow/disconnection; session stopped to avoid a stuck gesture.".into());
            return false;
        }
        true
    }
    /// Cancellation only: the worker closes sockets, reaps children, and cleans scoped ADB state.
    pub fn stop(&self) {
        self.shared.cancel.store(true, Ordering::Relaxed);
        self.shared.status(Status::Stopped);
    }
    /// Only for application exit, never call from a normal UI interaction.
    pub fn wait_cleanup(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while !self.shared.done.load(Ordering::Acquire) {
            if start.elapsed() >= timeout {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
        true
    }
    #[cfg(test)]
    pub(crate) fn fixture(serial: &str, status: Status, frame: Option<Frame>) -> Self {
        Self::fixture_with_input(serial, status, frame).0
    }
    #[cfg(test)]
    pub(crate) fn fixture_with_input(
        serial: &str,
        status: Status,
        frame: Option<Frame>,
    ) -> (Self, Receiver<Vec<u8>>) {
        let (input, rx) = mpsc::sync_channel(128);
        (
            Self {
                serial: serial.into(),
                shared: Arc::new(Shared {
                    done: AtomicBool::new(true),
                    cancel: AtomicBool::new(false),
                    status: Mutex::new(status),
                    frame: Mutex::new(frame),
                    ctx: egui::Context::default(),
                }),
                input,
            },
            rx,
        )
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

fn encode(input: Input) -> Option<Vec<u8>> {
    let mut b = Vec::new();
    match input {
        Input::Key { code, down } => {
            b.extend([0, if down { 0 } else { 1 }]);
            b.extend(code.to_be_bytes());
            b.extend([0; 8]);
        }
        Input::Touch {
            action,
            x,
            y,
            width,
            height,
        } => {
            if action > 3
                || width == 0
                || height == 0
                || x >= u32::from(width)
                || y >= u32::from(height)
            {
                return None;
            }
            b.extend([2, action]);
            b.extend((-2i64).to_be_bytes());
            b.extend(x.to_be_bytes());
            b.extend(y.to_be_bytes());
            b.extend(width.to_be_bytes());
            b.extend(height.to_be_bytes());
            b.extend(
                (if action == 0 || action == 2 {
                    u16::MAX
                } else {
                    0
                })
                .to_be_bytes(),
            );
            b.extend([0; 8]);
        }
        Input::Text(text) => {
            // One bounded queue item, split on UTF-8 boundaries, never log its contents.
            if text.len() > 16 * 1024 {
                return None;
            }
            let mut rest = text.as_str();
            while !rest.is_empty() {
                let mut n = rest.len().min(300);
                while !rest.is_char_boundary(n) {
                    n -= 1;
                }
                b.push(1);
                b.extend((n as u32).to_be_bytes());
                b.extend(&rest.as_bytes()[..n]);
                rest = &rest[n..];
            }
        }
    }
    Some(b)
}

const TAIL: usize = 8192;
struct Owned {
    child: Child,
    tail: Arc<Mutex<Vec<u8>>>,
    drains: Vec<JoinHandle<()>>,
}
impl Owned {
    fn spawn(command: &mut Command) -> Result<Self, String> {
        let mut child = command
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot start tool: {e}"))?;
        let tail = Arc::new(Mutex::new(Vec::new()));
        let drains = vec![drain(child.stderr.take().unwrap(), tail.clone())];
        Ok(Self {
            child,
            tail,
            drains,
        })
    }
    fn stdout_log(&mut self) {
        if let Some(out) = self.child.stdout.take() {
            self.drains.push(drain(out, self.tail.clone()));
        }
    }
    fn diagnostic(&self) -> String {
        String::from_utf8_lossy(&self.tail.lock().unwrap()).into_owned()
    }
    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for t in self.drains.drain(..) {
            let _ = t.join();
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.stop();
    }
}
fn drain(mut pipe: impl Read + Send + 'static, tail: Arc<Mutex<Vec<u8>>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0; 4096];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let mut t = tail.lock().unwrap();
                    t.extend_from_slice(&buf[..n]);
                    if t.len() > TAIL {
                        let excess = t.len() - TAIL;
                        t.drain(..excess);
                    }
                }
            }
        }
    })
}
fn command(config: &Config, path: &str) -> Command {
    let mut c = Command::new(path);
    c.envs(config.env.iter().cloned()).stdin(Stdio::null());
    c
}
fn bounded(
    config: &Config,
    path: &str,
    args: &[&str],
    cancel: &AtomicBool,
) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Mirror cancelled".into());
    }
    let mut child = Owned::spawn(command(config, path).args(args).stdout(Stdio::piped()))?;
    child.stdout_log();
    let start = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(15) {
            child.stop();
            return Err(format!(
                "Tool cancelled or timed out: {}",
                child.diagnostic()
            ));
        }
        match child.child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                child.stop();
                let out = child.diagnostic();
                return if status.success() {
                    Ok(out)
                } else {
                    Err(format!("Tool exited {status}: {out}"))
                };
            }
            None => thread::sleep(Duration::from_millis(20)),
        }
    }
}
fn adb(config: &Config, args: &[&str], cancel: &AtomicBool) -> Result<String, String> {
    let mut scoped = vec!["-s", config.serial.as_str()];
    scoped.extend_from_slice(args);
    bounded(config, &config.adb_path, &scoped, cancel)
}
fn env(config: &Config, name: &str) -> Option<String> {
    config
        .env
        .iter()
        .rev()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
        .or_else(|| std::env::var(name).ok())
}
fn executable(config: &Config, name: &str) -> Option<PathBuf> {
    let direct = PathBuf::from(name);
    if direct.is_file() {
        return Some(direct);
    }
    env(config, "PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|p| p.join(name))
            .find(|p| p.is_file())
    })
}
fn discover(config: &Config, cancel: &AtomicBool) -> Result<(String, String), String> {
    let version = bounded(config, &config.scrcpy_path, &["--version"], cancel)?;
    if !version.lines().any(|l| {
        let mut words = l.split_whitespace();
        words.next() == Some("scrcpy") && words.next() == Some("4.1")
    }) {
        return Err(format!("Embedded mirroring requires scrcpy 4.1 and its matching server; install/select 4.1. Reported: {version}"));
    }
    let server = if let Some(path) = env(config, "SCRCPY_SERVER_PATH") {
        let p = PathBuf::from(path);
        if !p.is_file() {
            return Err(
                "SCRCPY_SERVER_PATH must point to the matching scrcpy 4.1 server file.".into(),
            );
        }
        p
    } else {
        let client = executable(config, &config.scrcpy_path)
            .and_then(|p| p.canonicalize().ok())
            .unwrap_or_else(|| PathBuf::from(&config.scrcpy_path));
        let parent = client.parent().unwrap_or(Path::new("."));
        let mut paths = vec![
            parent.join("scrcpy-server"),
            parent.join("../share/scrcpy/scrcpy-server"),
        ];
        paths.extend(
            [
                "/opt/homebrew/share/scrcpy/scrcpy-server",
                "/usr/local/share/scrcpy/scrcpy-server",
                "/usr/share/scrcpy/scrcpy-server",
            ]
            .map(PathBuf::from),
        );
        paths.into_iter().find(|p| p.is_file()).ok_or(
            "Cannot find scrcpy 4.1 server. Set SCRCPY_SERVER_PATH to its scrcpy-server file.",
        )?
    };
    let ffmpeg = if let Some(p)=env(config,"PHONEMIRROR_FFMPEG") { executable(config,&p) } else {
        ["/opt/homebrew/bin/ffmpeg","/usr/local/bin/ffmpeg","/usr/bin/ffmpeg","ffmpeg","ffmpeg.exe"].iter().find_map(|p|executable(config,p))
    }.ok_or("Cannot find ffmpeg. Install it locally or set PHONEMIRROR_FFMPEG to its executable; nothing is installed automatically.")?;
    Ok((
        server.to_string_lossy().into_owned(),
        ffmpeg.to_string_lossy().into_owned(),
    ))
}

struct Resources {
    remote: String,
    pushed: bool,
    port: Option<u16>,
    server: Option<Owned>,
    decoder: Option<Decoder>,
    sockets: Vec<TcpStream>,
    threads: Vec<JoinHandle<()>>,
}
impl Resources {
    fn new(_: &Config) -> Self {
        Self {
            remote: String::new(),
            pushed: false,
            port: None,
            server: None,
            decoder: None,
            sockets: Vec::new(),
            threads: Vec::new(),
        }
    }
    fn diagnostics(&self) -> String {
        let mut s = String::new();
        for (name, p) in [
            ("scrcpy server", self.server.as_ref()),
            ("ffmpeg", self.decoder.as_ref().map(|d| &d.process)),
        ] {
            if let Some(p) = p {
                s.push_str(&format!(
                    "
{name}: {}",
                    p.diagnostic()
                ));
            }
        }
        s
    }
    fn cleanup(&mut self, config: &Config) {
        for socket in self.sockets.drain(..) {
            let _ = socket.shutdown(Shutdown::Both);
        }
        if let Some(p) = self.decoder.as_mut() {
            p.stop();
        }
        if let Some(p) = self.server.as_mut() {
            p.stop();
        }
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        // Fresh flag: cancellation of the session must not cancel its cleanup.
        let fresh = AtomicBool::new(false);
        if let Some(port) = self.port.take() {
            let _ = adb(
                config,
                &["forward", "--remove", &format!("tcp:{port}")],
                &fresh,
            );
        }
        if self.pushed {
            let _ = adb(config, &["shell", "rm", "-f", &self.remote], &fresh);
            self.pushed = false;
        }
    }
}
fn connect(port: u16, shared: &Shared, deadline: Instant) -> Result<TcpStream, String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    loop {
        if shared.cancel.load(Ordering::Relaxed) {
            return Err("Mirror cancelled".into());
        }
        match TcpStream::connect_timeout(&address, Duration::from_millis(200)) {
            Ok(s) => return Ok(s),
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(30)),
            Err(e) => return Err(format!("Cannot connect scrcpy forward: {e}")),
        }
    }
}
// Keep server launch options independently testable without contacting a device.
fn server_args<'a>(serial: &'a str, classpath: &'a str, id: &'a str) -> Vec<&'a str> {
    vec![
        "-s",
        serial,
        "shell",
        classpath,
        "app_process",
        "/",
        "com.genymobile.scrcpy.Server",
        "4.1",
        id,
        "tunnel_forward=true",
        "audio=false",
        "control=true",
        // Session-scoped activity pulses work on battery/Wi-Fi, without changing settings.
        "keep_active=true",
        "power_on=true",
        "video_codec=h264",
        "max_size=1280",
        "max_fps=30",
        "send_device_meta=false",
        "send_frame_meta=true",
        "send_stream_meta=false",
        "send_dummy_byte=true",
        "clipboard_autosync=false",
        "log_level=info",
        "cleanup=false",
    ]
}

fn run(
    config: &Config,
    shared: &Arc<Shared>,
    input: Receiver<Vec<u8>>,
    r: &mut Resources,
) -> Result<(), String> {
    let (server_path, ffmpeg) = discover(config, &shared.cancel)?;
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|e| format!("Cannot generate session identity: {e}"))?;
    let scid = u32::from_be_bytes(random[..4].try_into().unwrap()) & 0x7fffffff;
    let scid = format!("{scid:08x}");
    r.remote = format!(
        "/data/local/tmp/phonemirror-{}.jar",
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    // Mark before push: a failed/interrupted push may have created a partial file.
    r.pushed = true;
    adb(config, &["push", &server_path, &r.remote], &shared.cancel)?;
    let forward = adb(
        config,
        &["forward", "tcp:0", &format!("localabstract:scrcpy_{scid}")],
        &shared.cancel,
    )?;
    let port = forward
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|p| *p != 0)
        .ok_or_else(|| format!("ADB did not return an allocated TCP port: {forward}"))?;
    r.port = Some(port);
    let classpath = format!("CLASSPATH={}", r.remote);
    let id = format!("scid={scid}");
    let args = server_args(&config.serial, &classpath, &id);
    let mut server = Owned::spawn(
        command(config, &config.adb_path)
            .args(args)
            .stdout(Stdio::piped()),
    )?;
    server.stdout_log();
    r.server = Some(server);
    let deadline = Instant::now() + Duration::from_secs(30);
    // ADB accepts TCP before the remote socket exists; retry until the dummy byte validates it.
    let video = loop {
        if Instant::now() >= deadline {
            return Err("scrcpy readiness timed out; verify matching 4.1 server".into());
        }
        let mut socket = connect(port, shared, deadline)?;
        socket
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| e.to_string())?;
        let mut byte = [0];
        let ready = loop {
            match socket.read_exact(&mut byte) {
                Ok(()) if byte[0] == 0 => break true,
                Ok(()) => return Err("Invalid scrcpy readiness byte (expected zero)".into()),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) && Instant::now() < deadline
                        && !shared.cancel.load(Ordering::Relaxed) =>
                {
                    continue
                }
                Err(_) => break false,
            }
        };
        if ready {
            break socket;
        }
        if shared.cancel.load(Ordering::Relaxed) {
            return Err("Mirror cancelled".into());
        }
        if let Some(status) = r
            .server
            .as_mut()
            .unwrap()
            .child
            .try_wait()
            .map_err(|e| e.to_string())?
        {
            return Err(format!("scrcpy server exited before readiness: {status}"));
        }
        thread::sleep(Duration::from_millis(30));
    };
    let mut control = connect(port, shared, deadline)?;
    control
        .set_write_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    control
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    r.sockets
        .push(video.try_clone().map_err(|e| e.to_string())?);
    r.sockets
        .push(control.try_clone().map_err(|e| e.to_string())?);
    let (events, rx) = mpsc::sync_channel::<String>(4);
    let first = Arc::new(AtomicBool::new(false));
    let (handoff, replacements) = mpsc::sync_channel(1);
    let (restart, rotations) = mpsc::sync_channel(1);
    let (decoder, stdin) = Decoder::start(
        config,
        &ffmpeg,
        shared.clone(),
        first.clone(),
        events.clone(),
    )?;
    r.decoder = Some(decoder);
    let s = shared.clone();
    let errors = events.clone();
    r.threads.push(thread::spawn(move || {
        if let Err(e) = feed_video(video, stdin, &s, &restart, &replacements) {
            let _ = errors.try_send(e);
        }
    }));
    // Consume server-to-client notifications even with clipboard autosync disabled.
    let mut incoming = control.try_clone().map_err(|e| e.to_string())?;
    let s = shared.clone();
    let events_for_restart = events.clone();
    let errors = events;
    r.threads.push(thread::spawn(move || {
        let mut buf = [0; 4096];
        while !s.cancel.load(Ordering::Relaxed) {
            match incoming.read(&mut buf) {
                Ok(0) => {
                    let _ = errors.try_send("Control transport disconnected".into());
                    break;
                }
                Ok(_) => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) => {}
                Err(e) => {
                    let _ = errors.try_send(format!("Control transport failed: {e}"));
                    break;
                }
            }
        }
    }));
    let mut streaming = false;
    while !shared.cancel.load(Ordering::Relaxed) {
        if rotations.try_recv().is_ok() {
            r.decoder.as_mut().unwrap().stop();
            let (decoder, stdin) = Decoder::start(
                config,
                &ffmpeg,
                shared.clone(),
                first.clone(),
                events_for_restart.clone(),
            )?;
            r.decoder = Some(decoder);
            handoff
                .send(stdin)
                .map_err(|_| "Video feeder stopped during rotation")?;
        }
        if let Ok(error) = rx.try_recv() {
            return Err(error);
        }
        for (name, p) in [
            ("scrcpy server", r.server.as_mut().unwrap()),
            ("ffmpeg", &mut r.decoder.as_mut().unwrap().process),
        ] {
            if let Some(status) = p.child.try_wait().map_err(|e| e.to_string())? {
                return Err(format!("{name} exited unexpectedly: {status}"));
            }
        }
        if first.load(Ordering::Relaxed) && !streaming {
            shared.status(Status::Streaming);
            streaming = true;
        }
        if !streaming && Instant::now() > deadline {
            return Err("No decoded frame within 30 seconds. Check matching scrcpy server and ffmpeg H264 support.".into());
        }
        // Bound work per cycle so continuous input cannot starve cancellation/monitoring.
        for _ in 0..16 {
            match input.try_recv() {
                Ok(bytes) => control
                    .write_all(&bytes)
                    .map_err(|e| format!("Control write failed: {e}"))?,
                Err(_) => break,
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

const DECODE_ARGS: &[&str] = &[
    "-hide_banner",
    "-loglevel",
    "error",
    "-flags",
    "low_delay",
    "-threads",
    "1",
    "-probesize",
    "32",
    "-analyzeduration",
    "0",
    "-f",
    "h264",
    "-i",
    "pipe:0",
    "-an",
    "-fps_mode",
    "passthrough",
    "-noautoscale",
    "-f",
    "image2pipe",
    "-c:v",
    "ppm",
    "-pix_fmt",
    "rgb24",
    "-flush_packets",
    "1",
    "pipe:1",
];
struct Decoder {
    process: Owned,
    reader: Option<JoinHandle<()>>,
    retiring: Arc<AtomicBool>,
}
impl Decoder {
    fn start(
        config: &Config,
        ffmpeg: &str,
        shared: Arc<Shared>,
        first: Arc<AtomicBool>,
        events: SyncSender<String>,
    ) -> Result<(Self, std::process::ChildStdin), String> {
        let mut process = Owned::spawn(
            command(config, ffmpeg)
                .args(DECODE_ARGS)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped()),
        )?;
        let stdin = process.child.stdin.take().unwrap();
        let stdout = process.child.stdout.take().unwrap();
        let retiring = Arc::new(AtomicBool::new(false));
        let done = retiring.clone();
        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while !shared.cancel.load(Ordering::Relaxed) && !done.load(Ordering::Relaxed) {
                match ppm(&mut reader) {
                    Ok(frame) => {
                        if !done.load(Ordering::Relaxed) {
                            *shared.frame.lock().unwrap() = Some(frame);
                            first.store(true, Ordering::Relaxed);
                            shared.ctx.request_repaint();
                        }
                    }
                    Err(e) => {
                        if !done.load(Ordering::Relaxed) {
                            let _ = events.try_send(format!("ffmpeg PPM output failed: {e}"));
                        }
                        break;
                    }
                }
            }
        });
        Ok((
            Self {
                process,
                reader: Some(reader),
                retiring,
            },
            stdin,
        ))
    }
    fn stop(&mut self) {
        self.retiring.store(true, Ordering::Relaxed);
        self.process.stop();
        if let Some(t) = self.reader.take() {
            let _ = t.join();
        }
    }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        self.stop();
    }
}
// https://raw.githubusercontent.com/Genymobile/scrcpy/v4.1/server/src/main/java/com/genymobile/scrcpy/device/Streamer.java
// Config flag bit 62; session bit 63 is disabled by send_stream_meta=false.
// Frame metadata is stripped here; ffmpeg still receives only raw H264.
fn feed_video(
    mut video: TcpStream,
    mut stdin: std::process::ChildStdin,
    shared: &Shared,
    restart: &SyncSender<()>,
    replacements: &Receiver<std::process::ChildStdin>,
) -> Result<(), String> {
    let mut previous = Vec::new();
    while !shared.cancel.load(Ordering::Relaxed) {
        let mut header = [0; 12];
        socket_exact(&mut video, &mut header, shared)?;
        let flags = u64::from_be_bytes(header[..8].try_into().unwrap());
        let len = u32::from_be_bytes(header[8..].try_into().unwrap()) as usize;
        if flags & (1 << 63) != 0 || len == 0 || len > 8 * 1024 * 1024 {
            return Err("Invalid scrcpy video packet header/size".into());
        }
        let mut packet = vec![0; len];
        socket_exact(&mut video, &mut packet, shared)?;
        if flags & (1 << 62) != 0 {
            if !previous.is_empty() && previous != packet {
                restart
                    .send(())
                    .map_err(|_| "Decoder restart disconnected")?;
                loop {
                    match replacements.recv_timeout(Duration::from_millis(100)) {
                        Ok(next) => {
                            stdin = next;
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout)
                            if !shared.cancel.load(Ordering::Relaxed) => {}
                        Err(_) => return Err("Decoder restart cancelled".into()),
                    }
                }
            }
            previous = packet.clone();
        }
        stdin
            .write_all(&packet)
            .map_err(|e| format!("ffmpeg H264 input failed: {e}"))?;
    }
    Ok(())
}
fn socket_exact(
    socket: &mut TcpStream,
    mut buffer: &mut [u8],
    shared: &Shared,
) -> Result<(), String> {
    while !buffer.is_empty() {
        if shared.cancel.load(Ordering::Relaxed) {
            return Err("Mirror cancelled".into());
        }
        match socket.read(buffer) {
            Ok(0) => return Err("Video transport disconnected".into()),
            Ok(n) => buffer = &mut buffer[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(e) => return Err(format!("Video transport failed: {e}")),
        }
    }
    Ok(())
}

/// P6 framing consumes exactly one raster delimiter, never skips pixel whitespace.
fn ppm(reader: &mut impl BufRead) -> io::Result<Frame> {
    let mut budget = 4096usize;
    fn token(r: &mut impl BufRead, budget: &mut usize) -> io::Result<Vec<u8>> {
        let mut token = Vec::new();
        let mut comment = false;
        loop {
            if *budget == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "PPM header too large",
                ));
            }
            *budget -= 1;
            let mut b = [0];
            r.read_exact(&mut b)?;
            let c = b[0];
            if comment {
                if c == b'\n' {
                    comment = false;
                }
                continue;
            }
            if token.is_empty() && c == b'#' {
                comment = true;
                continue;
            }
            if c.is_ascii_whitespace() {
                if !token.is_empty() {
                    return Ok(token);
                }
                continue;
            }
            token.push(c);
            if token.len() > 32 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "PPM token too large",
                ));
            }
        }
    }
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid P6 PPM dimensions/format",
        )
    };
    if token(reader, &mut budget)? != b"P6" {
        return Err(invalid());
    }
    let w = token(reader, &mut budget)?;
    let h = token(reader, &mut budget)?;
    let max = token(reader, &mut budget)?;
    let number = |b: &[u8]| {
        std::str::from_utf8(b)
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
    };
    let width = number(&w).ok_or_else(invalid)?;
    let height = number(&h).ok_or_else(invalid)?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 || max != b"255" {
        return Err(invalid());
    }
    let pixels = width
        .checked_mul(height)
        .filter(|n| *n <= 4096 * 4096)
        .ok_or_else(invalid)?;
    let mut rgb = vec![0; pixels as usize * 3];
    reader.read_exact(&mut rgb)?;
    Ok(Frame { width, height, rgb })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_keeps_active_without_changing_android_settings() {
        let args = server_args("fixture-serial", "CLASSPATH=/fixture.jar", "scid=12345678");
        assert_eq!(
            &args[..4],
            &["-s", "fixture-serial", "shell", "CLASSPATH=/fixture.jar"]
        );
        assert!(args.contains(&"keep_active=true"));
        assert!(args.contains(&"power_on=true"));
        assert!(args.contains(&"cleanup=false"));
        assert!(!args
            .iter()
            .any(|arg| arg.starts_with("stay_awake=") || arg.starts_with("screen_off_timeout=")));
    }

    #[test]
    fn key_layout() {
        assert_eq!(
            encode(Input::Key {
                code: 0x12345678,
                down: true
            })
            .unwrap(),
            vec![0, 0, 0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            encode(Input::Key {
                code: 4,
                down: false
            })
            .unwrap()[1],
            1
        );
    }
    #[test]
    fn touch_layout() {
        for action in 0..4 {
            let b = encode(Input::Touch {
                action,
                x: 12,
                y: 34,
                width: 100,
                height: 200,
            })
            .unwrap();
            assert_eq!(b.len(), 32);
            assert_eq!(&b[..2], &[2, action]);
            assert_eq!(&b[2..10], &(-2i64).to_be_bytes());
            assert_eq!(&b[10..14], &12u32.to_be_bytes());
            assert_eq!(&b[14..18], &34u32.to_be_bytes());
            assert_eq!(&b[18..22], &[0, 100, 0, 200]);
            assert_eq!(
                &b[22..24],
                if action == 0 || action == 2 {
                    &[255, 255]
                } else {
                    &[0, 0]
                }
            );
            assert_eq!(&b[24..], &[0; 8]);
        }
        for (action, x, width) in [(4, 0, 10), (0, 10, 10), (0, 0, 0)] {
            assert!(encode(Input::Touch {
                action,
                x,
                y: 0,
                width,
                height: 10
            })
            .is_none());
        }
    }
    #[test]
    fn text_chunks() {
        let text = "é🙂".repeat(150);
        let bytes = encode(Input::Text(text.clone())).unwrap();
        let mut rest = bytes.as_slice();
        let mut decoded = String::new();
        while !rest.is_empty() {
            assert_eq!(rest[0], 1);
            let n = u32::from_be_bytes(rest[1..5].try_into().unwrap()) as usize;
            assert!(n <= 300);
            decoded.push_str(std::str::from_utf8(&rest[5..5 + n]).unwrap());
            rest = &rest[5 + n..];
        }
        assert_eq!(decoded, text);
        assert!(encode(Input::Text("a".repeat(16385))).is_none());
    }
    #[test]
    fn ppm_rotation_and_pixels() {
        let mut data = b"P6\n# comment\n2 1\n255\n".to_vec();
        data.extend([10, 32, 35, 0, 255, 1]);
        data.extend(b"P6\n1 2\n255\n");
        data.extend([1, 2, 3, 4, 5, 6]);
        let mut reader = io::Cursor::new(data);
        let a = ppm(&mut reader).unwrap();
        assert_eq!((a.width, a.height), (2, 1));
        assert_eq!(a.rgb, [10, 32, 35, 0, 255, 1]);
        let b = ppm(&mut reader).unwrap();
        assert_eq!((b.width, b.height), (1, 2));
    }
    #[test]
    fn ppm_bad_inputs() {
        for bytes in [
            b"P3\n1 1\n255\nabc".as_slice(),
            b"P6\n1 1\n255\nab",
            b"P6\n0 1\n255\n",
            b"P6\n4097 1\n255\n",
            b"P6\n1 1\n256\nabc",
            b"P6\n999999999999999999999 1\n255\n",
        ] {
            assert!(ppm(&mut io::Cursor::new(bytes)).is_err());
        }
        let bytes = format!("P6\n#{}\n1 1\n255\nabc", "a".repeat(5000));
        assert!(ppm(&mut io::Cursor::new(bytes)).is_err());
    }
    #[test]
    fn cancelled_frame_and_input_failure() {
        let session = Session::fixture(
            "fixture",
            Status::Streaming,
            Some(Frame {
                width: 1,
                height: 1,
                rgb: vec![0; 3],
            }),
        );
        assert!(!session.send(Input::Key {
            code: 4,
            down: false
        }));
        assert!(matches!(session.status(), Status::Failed(_)));
        assert!(session.take_frame().is_none());
        session.stop();
        assert!(matches!(session.status(), Status::Failed(_)));
        assert!(session.wait_cleanup(Duration::ZERO));
    }
    #[test]
    fn cancellation_before_spawn() {
        let c = Config {
            serial: "fixture-only".into(),
            adb_path: "DO-NOT-RUN-ADB".into(),
            scrcpy_path: String::new(),
            env: vec![],
        };
        assert!(adb(
            &c,
            &["forward", "--remove", "tcp:1234"],
            &AtomicBool::new(true)
        )
        .unwrap_err()
        .contains("cancelled"));
    }
}
#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn synthetic_h264_live_frames_and_rotated_decoder() {
        let config = Config {
            serial: "host-fixture".into(),
            adb_path: String::new(),
            scrcpy_path: String::new(),
            env: std::env::vars().collect(),
        };
        let Some(ffmpeg) = [
            "/opt/homebrew/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "ffmpeg",
        ]
        .iter()
        .find_map(|p| executable(&config, p)) else {
            eprintln!("ffmpeg unavailable: host synthetic decoder test skipped");
            return;
        };
        let ffmpeg = ffmpeg.to_string_lossy();
        // Each changed config starts a new decoder, whose PPM encoder adopts that geometry.
        for (width, height) in [(64, 32), (32, 64)] {
            let output = Command::new(ffmpeg.as_ref())
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    &format!("color=size={width}x{height}:rate=10"),
                    "-frames:v",
                    "3",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "ultrafast",
                    "-tune",
                    "zerolatency",
                    "-f",
                    "h264",
                    "pipe:1",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "Synthetic H264 encoder: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let session = Session::fixture("host", Status::Starting, None);
            let shared = session.shared.clone();
            let seen = Arc::new(AtomicBool::new(false));
            let (events, _) = mpsc::sync_channel(4);
            let (mut decoder, mut stdin) =
                Decoder::start(&config, &ffmpeg, shared.clone(), seen.clone(), events).unwrap();
            stdin.write_all(&output.stdout).unwrap();
            stdin.flush().unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while !seen.load(Ordering::Relaxed) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            let frame = shared.frame.lock().unwrap().take();
            // Kill before closing stdin; receiving output at EOF would not prove a live stream.
            decoder.stop();
            drop(stdin);
            let frame = frame.expect("No live PPM frame before stdin EOF");
            assert_eq!((frame.width, frame.height), (width, height));
            assert_eq!(frame.rgb.len(), width as usize * height as usize * 3);
        }
    }
}
#[cfg(all(test, unix))]
mod packet_tests {
    use super::*;
    fn capture() -> (Owned, std::process::ChildStdin) {
        let mut process = Owned::spawn(
            Command::new("/bin/cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped()),
        )
        .unwrap();
        let stdin = process.child.stdin.take().unwrap();
        (process, stdin)
    }
    fn sockets() -> (TcpStream, TcpStream) {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let server = listener.accept().unwrap().0;
        server
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        (client, server)
    }
    fn packet(flags: u64, data: &[u8]) -> Vec<u8> {
        let mut p = flags.to_be_bytes().to_vec();
        p.extend((data.len() as u32).to_be_bytes());
        p.extend(data);
        p
    }
    #[test]
    fn strips_headers_and_restarts_only_changed_bit62_config() {
        let (mut wire, video) = sockets();
        let session = Session::fixture("loopback", Status::Starting, None);
        let shared = session.shared.clone();
        let (mut a, stdin_a) = capture();
        let (mut b, stdin_b) = capture();
        let (restart, rotations) = mpsc::sync_channel(1);
        let (handoff, replacements) = mpsc::sync_channel(1);
        let feeder =
            thread::spawn(move || feed_video(video, stdin_a, &shared, &restart, &replacements));
        // Identical config, key-frame and timestamp packets must all stay on decoder A.
        let mut bytes = packet(1 << 62, b"config-a");
        bytes.extend(packet(1 << 62, b"config-a"));
        bytes.extend(packet((1 << 61) | 42, b"key"));
        bytes.extend(packet(123, b"frame"));
        bytes.extend(packet(1 << 62, b"config-b"));
        bytes.extend(packet(456, b"rotated"));
        wire.write_all(&bytes).unwrap();
        wire.shutdown(Shutdown::Write).unwrap();
        let request = rotations.recv_timeout(Duration::from_secs(3));
        if request.is_ok() {
            handoff.send(stdin_b).unwrap();
        } else {
            drop(stdin_b);
            session.stop();
        }
        let result = feeder.join().unwrap();
        // Let cat drain its tiny stdin payload after feeder drops the pipe.
        // Owned guards still kill/reap both children if an assertion unwinds.
        for process in [&mut a, &mut b] {
            let deadline = Instant::now() + Duration::from_secs(3);
            while process.child.try_wait().unwrap().is_none() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            process.stop();
        }
        assert!(request.is_ok(), "No changed-config restart");
        assert!(result.unwrap_err().contains("disconnected"));
        assert!(
            rotations.try_recv().is_err(),
            "Unexpected extra decoder restart"
        );
        let mut output_a = Vec::new();
        a.child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut output_a)
            .unwrap();
        let mut output_b = Vec::new();
        b.child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut output_b)
            .unwrap();
        assert_eq!(output_a, b"config-aconfig-akeyframe");
        assert_eq!(output_b, b"config-brotated");
    }
    #[test]
    fn rejects_session_bit63_and_oversized_payload_before_reading_body() {
        for (flags, len) in [(1u64 << 63, 1u32), (0, 8 * 1024 * 1024 + 1), (0, 0)] {
            let (mut wire, video) = sockets();
            let session = Session::fixture("loopback", Status::Starting, None);
            let (mut child, stdin) = capture();
            let (restart, _) = mpsc::sync_channel(1);
            let (_, replacements) = mpsc::sync_channel(1);
            let mut header = flags.to_be_bytes().to_vec();
            header.extend(len.to_be_bytes());
            wire.write_all(&header).unwrap();
            wire.shutdown(Shutdown::Write).unwrap();
            let result = feed_video(video, stdin, &session.shared, &restart, &replacements);
            child.stop();
            assert!(result
                .unwrap_err()
                .contains("Invalid scrcpy video packet header/size"));
            let mut output = Vec::new();
            child
                .child
                .stdout
                .take()
                .unwrap()
                .read_to_end(&mut output)
                .unwrap();
            assert!(output.is_empty());
        }
    }
}
