//! Read-only, bounded logcat snapshots. No subprocess work runs on the UI thread.
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_BYTES: usize = 256 * 1024;
const DEADLINE: Duration = Duration::from_secs(5);
const TICK: Duration = Duration::from_millis(10);

type Snapshot = Result<String, String>;

struct Job {
    serial: String,
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Snapshot>,
    worker: JoinHandle<()>,
}

#[derive(Default)]
pub struct Reader {
    job: Option<Job>,
}

impl Reader {
    /// Starts one snapshot; repeated requests while busy are ignored.
    /// The result carries the original serial so callers can discard stale selections.
    pub fn request(
        &mut self,
        ctx: &egui::Context,
        adb: String,
        serial: String,
        env: Vec<(String, String)>,
    ) {
        if self.busy() {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker_serial = serial.clone();
        let ctx = ctx.clone();
        let (tx, result) = mpsc::channel();
        let worker = thread::spawn(move || {
            let snapshot = snapshot(&adb, &worker_serial, &env, &worker_cancel, DEADLINE);
            let _ = tx.send(snapshot);
            ctx.request_repaint();
        });
        self.job = Some(Job {
            serial,
            cancel,
            result,
            worker,
        });
    }

    pub fn poll(&mut self) -> Option<(String, Result<String, String>)> {
        let job = self.job.as_ref()?;
        let result = match job.result.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Logcat worker stopped unexpectedly.".into())
            }
        };
        let job = self.job.take().unwrap();
        // Never wait on the UI thread. A sent result means child cleanup is complete.
        if job.worker.is_finished() {
            let _ = job.worker.join();
        }
        Some((job.serial, result))
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
            // Worker owns the child, including kill/wait. Bound shutdown waiting even
            // if the OS stalls; a detached worker still retains cleanup ownership.
            let until = Instant::now() + Duration::from_millis(100);
            while !job.worker.is_finished() && Instant::now() < until {
                thread::sleep(Duration::from_millis(1));
            }
            if job.worker.is_finished() {
                let _ = job.worker.join();
            }
        }
    }
}

// This guard is used only by the worker, never by Reader/UI Drop.
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn drain(
    mut pipe: impl Read + Send + 'static,
    remaining: Arc<AtomicUsize>,
    truncated: Arc<AtomicBool>,
) -> JoinHandle<Result<Vec<u8>, String>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            let count = match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(format!("Cannot read logcat output: {error}")),
            };
            let before = remaining
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                    Some(left.saturating_sub(count))
                })
                .unwrap();
            let keep = count.min(before);
            bytes.extend_from_slice(&chunk[..keep]);
            if keep != count {
                truncated.store(true, Ordering::Relaxed);
            }
            // Keep draining after the cap: closing a pipe early can deadlock/fail ADB.
        }
        Ok(bytes)
    })
}

fn snapshot(
    adb: &str,
    serial: &str,
    env: &[(String, String)],
    cancel: &AtomicBool,
    deadline: Duration,
) -> Snapshot {
    let start = Instant::now();
    if cancel.load(Ordering::Relaxed) {
        return Err("Logcat cancelled.".into());
    }
    if serial.is_empty() {
        return Err("Select a device before reading logcat.".into());
    }
    let mut child = OwnedChild(
        Command::new(adb)
            .args(["-s", serial, "logcat", "-d", "-t", "500", "-v", "brief"])
            .envs(env.iter().cloned())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("Cannot start ADB for logcat: {error}"))?,
    );
    let remaining = Arc::new(AtomicUsize::new(MAX_BYTES));
    let truncated = Arc::new(AtomicBool::new(false));
    let out = drain(
        child.0.stdout.take().unwrap(),
        remaining.clone(),
        truncated.clone(),
    );
    let err = drain(child.0.stderr.take().unwrap(), remaining, truncated.clone());
    let mut status = None;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Logcat cancelled.".into());
        }
        if start.elapsed() >= deadline {
            // The guard kills and reaps before the worker sends this result. Pipe
            // readers are detached on failure: descendants holding inherited pipes
            // must not extend the worker's deadline; they still drain until EOF.
            return Err("Logcat timed out (5 second limit).".into());
        }
        if status.is_none() {
            status = child
                .0
                .try_wait()
                .map_err(|error| format!("Cannot wait for logcat: {error}"))?;
        }
        if status.is_some() && out.is_finished() && err.is_finished() {
            break;
        }
        thread::sleep(TICK);
    }
    let out = out
        .join()
        .map_err(|_| "Logcat output reader stopped.".to_string())??;
    let err = err
        .join()
        .map_err(|_| "Logcat error reader stopped.".to_string())??;
    let mut bytes = out;
    if !bytes.is_empty() && !err.is_empty() && bytes.len() + err.len() < MAX_BYTES {
        bytes.push(b'\n');
    }
    bytes.extend(err);
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    // Lossy UTF-8 expansion must not turn the byte budget into a larger UI payload.
    let marker = "
[Logcat output truncated at 256 KiB]";
    if truncated.load(Ordering::Relaxed) || text.len() > MAX_BYTES {
        let mut end = (MAX_BYTES - marker.len()).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str(marker);
    }
    if status.unwrap().success() {
        Ok(text)
    } else {
        Err(format!(
            "ADB logcat failed ({}): {}",
            status.unwrap(),
            text.trim()
        ))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(script: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "phonemirror-logcat-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&dir).unwrap();
            let fixture = Self(dir);
            fs::write(fixture.path(), format!("#!/bin/sh\n{script}\n")).unwrap();
            fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o700)).unwrap();
            fixture
        }
        fn wait_started(&self) {
            let until = Instant::now() + Duration::from_secs(3);
            while !self.0.join("pid").exists() {
                assert!(Instant::now() < until, "fixture did not start");
                thread::sleep(TICK);
            }
        }
        fn assert_reaped(&self) {
            let pid = fs::read_to_string(self.0.join("pid")).unwrap();
            assert!(
                !Command::new("/bin/kill")
                    .args(["-0", pid.trim()])
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success(),
                "fixture child still exists: {pid}"
            );
        }
        fn path(&self) -> PathBuf {
            self.0.join("fake adb with spaces")
        }
        fn run(&self, deadline: Duration) -> Snapshot {
            snapshot(
                self.path().to_str().unwrap(),
                "serial with spaces",
                &[(
                    "LOGCAT_PID".into(),
                    self.0.join("pid").to_string_lossy().into(),
                )],
                &AtomicBool::new(false),
                deadline,
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn exact_arguments_environment_and_async_result() {
        let fixture = Fixture::new("printf '<%s>\n' \"$@\"; printf '%s' \"$LOGCAT_FIXTURE\"");
        let mut reader = Reader::default();
        let ctx = egui::Context::default();
        reader.request(
            &ctx,
            fixture.path().to_string_lossy().into(),
            "serial with spaces".into(),
            vec![("LOGCAT_FIXTURE".into(), "environment value".into())],
        );
        assert!(reader.busy());
        reader.request(&ctx, "not-an-executable".into(), "ignored".into(), vec![]);
        let until = Instant::now() + Duration::from_secs(2);
        let (serial, result) = loop {
            if let Some(result) = reader.poll() {
                break result;
            }
            assert!(Instant::now() < until);
            thread::sleep(TICK);
        };
        assert_eq!(serial, "serial with spaces");
        assert_eq!(
            result.unwrap(),
            "<-s>
<serial with spaces>
<logcat>
<-d>
<-t>
<500>
<-v>
<brief>
environment value"
        );
        assert!(!reader.busy());
        assert!(reader.poll().is_none());
    }

    #[test]
    fn reports_exit_and_spawn_errors() {
        let fixture = Fixture::new("printf 'permission denied' >&2; exit 7");
        let error = fixture.run(DEADLINE).unwrap_err();
        assert!(error.contains("permission denied") && error.contains('7'));
        assert!(snapshot(
            "/nonexistent/phonemirror-logcat-fixture",
            "fixture",
            &[],
            &AtomicBool::new(false),
            DEADLINE
        )
        .unwrap_err()
        .contains("Cannot start ADB"));
    }

    #[test]
    fn combined_cap_drains_both_streams_and_bounds_invalid_utf8() {
        let fixture = Fixture::new("i=0; while [ $i -lt 20000 ]; do printf '01234567890123456789'; printf 'abcdefghijabcdefghij' >&2; i=$((i+1)); done");
        let text = fixture.run(DEADLINE).unwrap();
        assert!(text.len() <= MAX_BYTES);
        assert!(text.ends_with("[Logcat output truncated at 256 KiB]"));
        let remaining = Arc::new(AtomicUsize::new(7));
        let truncated = Arc::new(AtomicBool::new(false));
        let bytes = drain(
            std::io::Cursor::new(vec![255; 100]),
            remaining.clone(),
            truncated.clone(),
        )
        .join()
        .unwrap()
        .unwrap();
        assert_eq!(bytes.len(), 7);
        assert_eq!(remaining.load(Ordering::Relaxed), 0);
        assert!(truncated.load(Ordering::Relaxed));
    }

    #[test]
    fn timeout_and_cancellation_reap_owned_child() {
        let fixture = Fixture::new("printf '%s' \"$$\" > \"$LOGCAT_PID\"; exec /bin/sleep 30");
        let start = Instant::now();
        assert!(fixture
            .run(Duration::from_millis(500))
            .unwrap_err()
            .contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(2));
        fixture.assert_reaped();
        let fixture = Fixture::new("printf '%s' \"$$\" > \"$LOGCAT_PID\"; exec /bin/sleep 30");
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let path = fixture.path();
        let env = vec![(
            "LOGCAT_PID".into(),
            fixture.0.join("pid").to_string_lossy().into(),
        )];
        let worker = thread::spawn(move || {
            snapshot(path.to_str().unwrap(), "fixture", &env, &flag, DEADLINE)
        });
        fixture.wait_started();
        cancel.store(true, Ordering::Relaxed);
        assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
        fixture.assert_reaped();
    }

    #[test]
    fn cancelled_before_spawn_and_reader_drop_are_bounded() {
        assert!(snapshot(
            "/never-spawn",
            "fixture",
            &[],
            &AtomicBool::new(true),
            DEADLINE
        )
        .unwrap_err()
        .contains("cancelled"));
        let fixture = Fixture::new("printf '%s' \"$$\" > \"$LOGCAT_PID\"; exec /bin/sleep 30");
        let mut reader = Reader::default();
        reader.request(
            &egui::Context::default(),
            fixture.path().to_string_lossy().into(),
            "fixture".into(),
            vec![(
                "LOGCAT_PID".into(),
                fixture.0.join("pid").to_string_lossy().into(),
            )],
        );
        fixture.wait_started();
        let start = Instant::now();
        drop(reader);
        assert!(start.elapsed() < Duration::from_secs(1));
        fixture.assert_reaped();
    }
}
