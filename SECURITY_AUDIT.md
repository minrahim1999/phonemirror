# Security Audit — PhoneMirror v1.0.0

## Summary

Security audit performed on 2026-05-14. Found **3 medium** and **2 low** issues.

## Findings

### 🔴 MEDIUM: Command Injection via `shell()` / `run_bg()` (Rust & Swift)

**Location:** `src/main.rs` — `shell()`, `run_bg()` functions; `macos-swiftui/PhoneMirror.swift` — `shell()`, `runSilent()` functions

**Issue:** Both versions pass commands through `sh -c` which could be vulnerable if any user-derived data (like device IDs from ADB) contained shell metacharacters. The timestamp-based filenames are safe (controlled format), but the `shell()` and `run_bg()` wrappers are inherently risky patterns.

**Risk:** A malicious device name or ADB output containing shell metacharacters (`;`, `|`, `$()`, etc.) could potentially execute arbitrary commands.

**Mitigation:** In practice, the risk is low because:
- Phone device IDs are typically alphanumeric (e.g., `RRCX107DMRP`)
- The ADB path and scrcpy path are hardcoded/detected, not user-supplied
- The shell scripts are local-only, no network input

**Recommendation:** Use `Command::new()` with `.args()` instead of `sh -c` for all invocations. This is the preferred fix for future versions.

---

### 🔴 MEDIUM: `osascript` Notification Injection (Swift)

**Location:** `macos-swiftui/PhoneMirror.swift` — `notify()` function

**Issue:** The notification title and body are interpolated into an AppleScript string without escaping quotes. If a filename contained a double-quote, it could break the AppleScript.

**Risk:** Low — filenames are timestamp-based (`phone_screenshot_20260514_150000.png`) which cannot contain quotes.

**Recommendation:** Escape double-quotes in the title/body before passing to osascript.

---

### 🔴 MEDIUM: Unmaintained Dependencies (Rust)

**Location:** `Cargo.lock` — `ansi_term` v0.12.1, `atty` v0.2.14

**Issue:** `cargo audit` reports 3 warnings:
- RUSTSEC-2021-0139: `ansi_term` is unmaintained (via `ksni` → `clap v2`)
- RUSTSEC-2024-0375: `atty` is unmaintained (via `ksni` → `clap v2`)  
- RUSTSEC-2021-0145: `atty` has a potential unaligned read (via `ksni` → `clap v2`)

These are transitive dependencies from the `tray-item` crate's `ksni` feature (Linux only).

**Risk:** Low — these are only used on Linux via the `ksni` feature, and the unmaintained crates are not security-critical (terminal color and TTY detection). The unaligned read issue in `atty` is low severity.

**Recommendation:** Monitor `tray-item` for updates. Consider switching to `ksni` directly or using `libappindicator` feature instead.

---

### 🟡 LOW: No TLS/Network Security Concerns

**Finding:** PhoneMirror does **not** make any network connections. It only runs local commands (`adb`, `scrcpy`, `pkill`, `osascript`, `notify-send`). No data is sent anywhere.

**Status:** ✅ No action needed.

---

### 🟡 LOW: File Permissions

**Location:** Screenshots saved to `~/Pictures/`, recordings to `~/Movies/`

**Issue:** Files are created with default umask permissions (typically `644`). In a multi-user environment, other local users could read screenshots.

**Risk:** Very low — typical home directory permissions prevent access.

**Recommendation:** Consider setting `0600` on created files if screenshots may contain sensitive data.

---

## ✅ Positive Findings

1. **No network communication** — all operations are local
2. **No secrets/credentials stored** — no API keys, tokens, or passwords
3. **Hardcoded tool paths** — paths are detected from known locations, not user input
4. **Timestamp filenames** — cannot contain shell metacharacters
5. **ADB commands are deterministic** — no arbitrary command execution from user input
6. **LaunchAgent runs as current user** — no privilege escalation

---

## Dependency Audit (cargo audit)

```
Crate:     ansi_term 0.12.1  — unmaintained (RUSTSEC-2021-0139)
Crate:     atty 0.2.14       — unmaintained (RUSTSEC-2024-0375), unaligned read (RUSTSEC-2021-0145)
```

**0 critical vulnerabilities, 0 high vulnerabilities, 3 warnings (all from transitive Linux deps)**

---

*Audit performed by ClawNovaX on 2026-05-14*