//! Finds ADB, scrcpy and FFmpeg.
//!
//! Search order per tool: the custom path saved in settings, the `PHONEMIRROR_*`
//! environment variable, the Android SDK (ADB only), package-manager folders, then
//! `PATH`. Every existing candidate is run with its version flag and the first one
//! that works is used, so a broken copy early in the list never wins.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Adb,
    Scrcpy,
    Ffmpeg,
}

pub const ALL: [Tool; 3] = [Tool::Adb, Tool::Scrcpy, Tool::Ffmpeg];
const CUSTOM: &str = "your custom path";

impl Tool {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn key(self) -> &'static str {
        match self {
            Tool::Adb => "adb",
            Tool::Scrcpy => "scrcpy",
            Tool::Ffmpeg => "ffmpeg",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Tool::Adb => "ADB",
            Tool::Scrcpy => "scrcpy",
            Tool::Ffmpeg => "FFmpeg",
        }
    }
    pub fn purpose(self) -> &'static str {
        match self {
            Tool::Adb => "Finds and controls your devices",
            Tool::Scrcpy => "Streams and records the screen",
            Tool::Ffmpeg => "Decodes the live screen",
        }
    }
    pub fn env_var(self) -> &'static str {
        match self {
            Tool::Adb => "PHONEMIRROR_ADB",
            Tool::Scrcpy => "PHONEMIRROR_SCRCPY",
            Tool::Ffmpeg => "PHONEMIRROR_FFMPEG",
        }
    }
    fn version_arg(self) -> &'static str {
        match self {
            Tool::Adb => "version",
            Tool::Scrcpy => "--version",
            Tool::Ffmpeg => "-version",
        }
    }
    fn binary(self) -> String {
        if cfg!(windows) {
            format!("{}.exe", self.key())
        } else {
            self.key().into()
        }
    }
    pub fn install_command(self) -> &'static str {
        if cfg!(target_os = "macos") {
            match self {
                Tool::Adb => "brew install --cask android-platform-tools",
                Tool::Scrcpy => "brew install scrcpy",
                Tool::Ffmpeg => "brew install ffmpeg",
            }
        } else if cfg!(windows) {
            match self {
                Tool::Adb => "winget install Google.PlatformTools",
                Tool::Scrcpy => "winget install Genymobile.scrcpy",
                Tool::Ffmpeg => "winget install Gyan.FFmpeg",
            }
        } else {
            match self {
                Tool::Adb => "sudo apt install adb",
                Tool::Scrcpy => "sudo apt install scrcpy",
                Tool::Ffmpeg => "sudo apt install ffmpeg",
            }
        }
    }
}

// ─── Settings ──────────────────────────────────────────────

/// Custom tool paths, stored as `key = path` lines in tools.conf.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    custom: [Option<String>; 3],
}

impl Settings {
    pub fn custom(&self, tool: Tool) -> Option<&str> {
        self.custom[tool.index()].as_deref()
    }
    pub fn set(&mut self, tool: Tool, path: Option<String>) {
        self.custom[tool.index()] = path.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    }
    pub fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        for line in text.lines().map(str::trim) {
            if line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                if let Some(tool) = ALL.into_iter().find(|t| t.key() == key.trim()) {
                    settings.set(tool, Some(value.to_string()));
                }
            }
        }
        settings
    }
    pub fn render(&self) -> String {
        let mut out =
            String::from("# PhoneMirror tool paths. Remove a line to use automatic detection.\n");
        for tool in ALL {
            if let Some(path) = self.custom(tool) {
                out.push_str(&format!("{} = {}\n", tool.key(), path));
            }
        }
        out
    }
    pub fn load() -> Self {
        config_file()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<(), String> {
        let path =
            config_file().ok_or_else(|| "No home folder to save settings in.".to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let temporary = path.with_extension("conf.tmp");
        std::fs::write(&temporary, self.render()).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
    }
}

pub fn config_file() -> Option<PathBuf> {
    let var = |key: &str| std::env::var(key).ok().filter(|v| !v.is_empty());
    if cfg!(target_os = "macos") {
        var("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support/PhoneMirror/tools.conf"))
    } else if cfg!(windows) {
        var("APPDATA").map(|d| PathBuf::from(d).join("PhoneMirror").join("tools.conf"))
    } else {
        var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|d| d.join("phonemirror").join("tools.conf"))
    }
}

/// Current custom paths as editable text, empty meaning automatic.
pub fn custom_inputs() -> [String; 3] {
    let settings = Settings::load();
    ALL.map(|tool| settings.custom(tool).unwrap_or_default().to_string())
}

// ─── Discovery ─────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub source: &'static str,
}

fn add(list: &mut Vec<Candidate>, path: PathBuf, source: &'static str) {
    if !list.iter().any(|c| c.path == path) {
        list.push(Candidate { path, source });
    }
}

/// Every place a tool may live, most specific first. Pure: `var` supplies the environment.
pub fn candidates(
    tool: Tool,
    custom: Option<&str>,
    var: &dyn Fn(&str) -> Option<String>,
) -> Vec<Candidate> {
    let get = |key: &str| var(key).filter(|v| !v.trim().is_empty());
    let bin = tool.binary();
    let home = get("HOME").or_else(|| get("USERPROFILE"));
    let expand = |p: &str| match (p.strip_prefix("~/"), &home) {
        (Some(rest), Some(h)) => PathBuf::from(h).join(rest),
        _ => PathBuf::from(p),
    };
    let mut list = Vec::new();
    if let Some(path) = custom.map(str::trim).filter(|p| !p.is_empty()) {
        add(&mut list, expand(path), CUSTOM);
    }
    if let Some(path) = get(tool.env_var()) {
        add(&mut list, expand(path.trim()), tool.env_var());
    }
    if tool == Tool::Adb {
        for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
            if let Some(root) = get(key) {
                add(
                    &mut list,
                    PathBuf::from(root).join("platform-tools").join(&bin),
                    key,
                );
            }
        }
        if let Some(h) = &home {
            for sdk in ["Library/Android/sdk", "Android/Sdk"] {
                let path = PathBuf::from(h).join(sdk).join("platform-tools").join(&bin);
                add(&mut list, path, "the Android SDK");
            }
        }
        if let Some(local) = get("LOCALAPPDATA") {
            let path = PathBuf::from(local)
                .join("Android")
                .join("Sdk")
                .join("platform-tools");
            add(&mut list, path.join(&bin), "the Android SDK");
        }
    }
    if cfg!(windows) {
        if let Some(h) = &home {
            add(
                &mut list,
                PathBuf::from(h).join("scoop").join("shims").join(&bin),
                "Scoop",
            );
            let winget = PathBuf::from(h).join("AppData/Local/Microsoft/WinGet/Links");
            add(&mut list, winget.join(&bin), "WinGet");
        }
        if tool == Tool::Scrcpy {
            add(
                &mut list,
                PathBuf::from(r"C:\Program Files\scrcpy").join(&bin),
                "Program Files",
            );
        }
    } else {
        let usr_local = if cfg!(target_os = "macos") {
            "Homebrew"
        } else {
            "/usr/local"
        };
        for (dir, source) in [
            ("/opt/homebrew/bin", "Homebrew"),
            ("/usr/local/bin", usr_local),
            ("/opt/local/bin", "MacPorts"),
            ("/usr/bin", "the system"),
            ("/snap/bin", "Snap"),
        ] {
            add(&mut list, Path::new(dir).join(&bin), source);
        }
    }
    if let Some(path) = get("PATH") {
        for dir in std::env::split_paths(&path) {
            add(&mut list, dir.join(&bin), "PATH");
        }
    }
    list
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolution {
    /// The working copy in use; None when nothing usable was found.
    pub path: Option<String>,
    pub source: String,
    pub version: String,
    /// Problems worth showing even when a working copy was found.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub tools: [Resolution; 3],
}

impl Report {
    pub fn get(&self, tool: Tool) -> &Resolution {
        &self.tools[tool.index()]
    }
    pub fn missing(&self) -> bool {
        self.tools.iter().any(|r| r.path.is_none())
    }
}

/// Picks the first candidate that exists and runs. ADB keeps probing so differing
/// installed versions, which restart each other's server, can be reported.
pub fn choose(
    tool: Tool,
    candidates: Vec<Candidate>,
    exists: impl Fn(&Path) -> bool,
    mut probe: impl FnMut(&Path) -> Result<String, String>,
) -> Resolution {
    let mut found = Resolution::default();
    let mut working: Vec<(String, String)> = Vec::new();
    let mut seen = HashSet::new();
    for candidate in candidates {
        let shown = candidate.path.display().to_string();
        if !exists(&candidate.path) {
            if candidate.source == CUSTOM {
                found.warnings.push(format!(
                    "{shown} doesn't exist or isn't a program. PhoneMirror is using automatic detection instead."
                ));
            }
            continue;
        }
        let identity = candidate
            .path
            .canonicalize()
            .unwrap_or_else(|_| candidate.path.clone());
        if !seen.insert(identity) {
            continue;
        }
        match probe(&candidate.path) {
            Ok(version) => {
                if found.path.is_none() {
                    found.path = Some(shown.clone());
                    found.source = candidate.source.to_string();
                    found.version = version.clone();
                }
                working.push((shown, version));
                if tool != Tool::Adb {
                    break;
                }
            }
            Err(error) if candidate.source == CUSTOM => found.warnings.push(format!(
                "{shown} didn't run ({error}). PhoneMirror is using automatic detection instead."
            )),
            Err(_) => {}
        }
    }
    if working.iter().any(|(_, v)| *v != found.version) {
        let list: Vec<_> = working.iter().map(|(p, v)| format!("{v} at {p}")).collect();
        found.warnings.push(format!(
            "Different ADB versions are installed: {}. They can restart each other's server and drop device connections. PhoneMirror uses {}; point Android Studio and your terminal at the same copy, or choose one here.",
            list.join("; "),
            found.path.as_deref().unwrap_or_default(),
        ));
    }
    found
}

fn version_line(tool: Tool, output: &str) -> String {
    let first = output
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let version = match tool {
        Tool::Adb => output
            .lines()
            .find_map(|l| l.trim().strip_prefix("Version "))
            .map(|v| v.split('-').next().unwrap_or(v)),
        Tool::Scrcpy => first
            .strip_prefix("scrcpy ")
            .and_then(|r| r.split_whitespace().next()),
        Tool::Ffmpeg => first
            .strip_prefix("ffmpeg version ")
            .and_then(|r| r.split_whitespace().next()),
    };
    version.unwrap_or(first).chars().take(40).collect()
}

fn probe(tool: Tool, path: &Path, env: &[(String, String)]) -> Result<String, String> {
    let mut child = Command::new(path)
        .arg(tool.version_arg())
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    while child.try_wait().map_err(|e| e.to_string())?.is_none() {
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("it did not respond within 5 seconds".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        return Ok(version_line(tool, &stdout));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(stderr
        .lines()
        .chain(stdout.lines())
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("it exited with an error")
        .chars()
        .take(160)
        .collect())
}

fn is_executable(path: &Path) -> bool {
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

static REPORT: Mutex<Option<Report>> = Mutex::new(None);

pub fn cached() -> Option<Report> {
    REPORT.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Probes every tool and caches the result. Runs subprocesses: call off the UI thread.
pub fn rescan(env: &[(String, String)]) -> Report {
    let settings = Settings::load();
    let var = |key: &str| std::env::var(key).ok();
    let report = Report {
        tools: ALL.map(|tool| {
            choose(
                tool,
                candidates(tool, settings.custom(tool), &var),
                is_executable,
                |p: &Path| probe(tool, p, env),
            )
        }),
    };
    *REPORT.lock().unwrap_or_else(|e| e.into_inner()) = Some(report.clone());
    report
}

/// Path to run for a tool. Before the first scan, the first existing candidate.
pub fn path(tool: Tool) -> String {
    if let Some(path) = cached().and_then(|r| r.get(tool).path.clone()) {
        return path;
    }
    let settings = Settings::load();
    candidates(tool, settings.custom(tool), &|k| std::env::var(k).ok())
        .into_iter()
        .find(|c| is_executable(&c.path))
        .map(|c| c.path.display().to_string())
        .unwrap_or_else(|| tool.binary())
}

/// Native file picker (macOS); None when cancelled or unsupported.
pub fn choose_file(tool: Tool) -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let prompt = format!(
        "return POSIX path of (choose file with prompt \"Choose the {} program\")",
        tool.label()
    );
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", "activate", "-e", &prompt])
        .output()
        .ok()?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !path.is_empty()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }
    fn candidate(path: &str, source: &'static str) -> Candidate {
        Candidate {
            path: path.into(),
            source,
        }
    }

    #[cfg(unix)]
    #[test]
    fn adb_search_prefers_user_choices_then_sdk_then_packages_then_path() {
        let env = [
            ("HOME", "/Users/me"),
            ("PHONEMIRROR_ADB", "/env/adb"),
            ("ANDROID_HOME", "/sdk"),
            ("PATH", "/opt/homebrew/bin:/custom/bin"),
        ];
        let list = candidates(Tool::Adb, Some(" ~/tools/adb "), &vars(&env));
        let paths: Vec<_> = list.iter().map(|c| c.path.display().to_string()).collect();
        assert_eq!(
            paths[..4],
            [
                "/Users/me/tools/adb",
                "/env/adb",
                "/sdk/platform-tools/adb",
                "/Users/me/Library/Android/sdk/platform-tools/adb",
            ]
        );
        assert_eq!(list[0].source, CUSTOM);
        assert!(
            paths.contains(&"/opt/local/bin/adb".to_string()),
            "MacPorts"
        );
        let homebrew = paths
            .iter()
            .filter(|p| *p == "/opt/homebrew/bin/adb")
            .count();
        assert_eq!(homebrew, 1, "PATH duplicates are skipped");
        assert_eq!(paths.last().map(String::as_str), Some("/custom/bin/adb"));
    }

    #[test]
    fn only_adb_searches_the_android_sdk() {
        let env = [
            ("ANDROID_HOME", "/sdk"),
            ("ANDROID_SDK_ROOT", "/sdk-root"),
            ("HOME", "/Users/me"),
            ("LOCALAPPDATA", "/local"),
        ];
        let sdk_paths: Vec<_> = candidates(Tool::Adb, None, &vars(&env))
            .into_iter()
            .filter(|c| matches!(c.source, "ANDROID_HOME" | "ANDROID_SDK_ROOT" | "the Android SDK"))
            .map(|c| c.path.parent().unwrap().to_path_buf())
            .collect();
        assert_eq!(sdk_paths.len(), 5, "all explicit and default SDK roots are covered");
        for tool in [Tool::Scrcpy, Tool::Ffmpeg] {
            let list = candidates(tool, None, &vars(&env));
            assert!(list.iter().all(|c| {
                !matches!(c.source, "ANDROID_HOME" | "ANDROID_SDK_ROOT" | "the Android SDK")
                    && sdk_paths.iter().all(|sdk| !c.path.starts_with(sdk))
            }), "{tool:?} must not search SDK platform-tools: {list:?}");
            // Home-directory package managers are valid tool sources on Windows.
            #[cfg(windows)]
            for (relative, source) in [
                ("scoop/shims", "Scoop"),
                ("AppData/Local/Microsoft/WinGet/Links", "WinGet"),
            ] {
                let expected = Path::new("/Users/me").join(relative).join(tool.binary());
                assert!(list.iter().any(|c| c.path == expected && c.source == source));
            }
        }
    }

    #[test]
    fn first_working_copy_wins_and_bad_custom_path_falls_back() {
        let list = vec![
            candidate("/custom/adb", CUSTOM),
            candidate("/broken/adb", "Homebrew"),
            candidate("/sdk/adb", "the Android SDK"),
            candidate("/other/adb", "PATH"),
        ];
        let found = choose(
            Tool::Adb,
            list,
            |p: &Path| p != Path::new("/custom/adb"),
            |p: &Path| match p.to_str().unwrap() {
                "/broken/adb" => Err("dyld: library not loaded".into()),
                "/sdk/adb" => Ok("37.0.1".into()),
                _ => Ok("37.0.0".into()),
            },
        );
        assert_eq!(found.path.as_deref(), Some("/sdk/adb"));
        assert_eq!(
            (found.source.as_str(), found.version.as_str()),
            ("the Android SDK", "37.0.1")
        );
        assert_eq!(found.warnings.len(), 2);
        assert!(found.warnings[0].contains("/custom/adb doesn't exist"));
        assert!(found.warnings[1].contains("37.0.0 at /other/adb"));
    }

    #[test]
    fn broken_custom_program_is_reported_and_missing_tool_has_no_path() {
        let found = choose(
            Tool::Ffmpeg,
            vec![
                candidate("/mine/ffmpeg", CUSTOM),
                candidate("/usr/bin/ffmpeg", "the system"),
            ],
            |_: &Path| true,
            |p: &Path| {
                if p.starts_with("/mine") {
                    Err("permission denied".into())
                } else {
                    Ok("8.0".into())
                }
            },
        );
        assert_eq!(found.path.as_deref(), Some("/usr/bin/ffmpeg"));
        assert!(found.warnings[0].contains("didn't run (permission denied)"));

        let same = choose(
            Tool::Adb,
            vec![candidate("/a/adb", "Homebrew"), candidate("/b/adb", "PATH")],
            |_: &Path| true,
            |_: &Path| Ok("37.0.1".into()),
        );
        assert!(same.warnings.is_empty(), "same version is not a conflict");

        let none = choose(
            Tool::Scrcpy,
            vec![candidate("/x", "PATH")],
            |_: &Path| false,
            |_: &Path| Ok(String::new()),
        );
        assert_eq!(none, Resolution::default());
        let report = Report {
            tools: [Resolution::default(), none.clone(), none],
        };
        assert!(report.missing());
    }

    #[test]
    fn settings_round_trip_and_ignore_unknown_lines() {
        let settings =
            Settings::parse("# note\nadb = /a b/adb \nscrcpy=\nunknown = x\nffmpeg=/f=1/ffmpeg\n");
        assert_eq!(settings.custom(Tool::Adb), Some("/a b/adb"));
        assert_eq!(settings.custom(Tool::Scrcpy), None);
        assert_eq!(settings.custom(Tool::Ffmpeg), Some("/f=1/ffmpeg"));
        assert_eq!(Settings::parse(&settings.render()), settings);
    }

    #[test]
    fn versions_are_read_from_each_tools_output() {
        let adb =
            "Android Debug Bridge version 1.0.41\nVersion 37.0.1-15733141\nInstalled as /x/adb";
        assert_eq!(version_line(Tool::Adb, adb), "37.0.1");
        assert_eq!(
            version_line(
                Tool::Scrcpy,
                "scrcpy 4.1 <https://github.com/Genymobile/scrcpy>\n"
            ),
            "4.1"
        );
        assert_eq!(
            version_line(Tool::Ffmpeg, "ffmpeg version 8.0.1 Copyright (c) 2000"),
            "8.0.1"
        );
        assert_eq!(
            version_line(Tool::Ffmpeg, "\nsomething else\n"),
            "something else"
        );
    }
}
