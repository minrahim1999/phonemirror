# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in PhoneMirror, please report it by opening a [GitHub Security Advisory](https://github.com/minrahim1999/phonemirror/security/advisories/new).

**Please do not report security vulnerabilities through public GitHub issues.**

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

### Response Timeline

- **Acknowledgment:** Within 48 hours
- **Initial assessment:** Within 7 days
- **Fix:** Depends on severity, typically within 14 days

## Security Considerations

- PhoneMirror runs **ADB** and **scrcpy** locally — it does not expose any network services
- USB debugging must be enabled on the phone — this is an Android requirement, not a PhoneMirror issue
- All data (screenshots, recordings) stays on your local machine
- No telemetry, no analytics, no data sent anywhere