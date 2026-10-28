#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
if [ "$(uname -s)" != Darwin ]; then
    echo "macOS is required to create this bundle." >&2
    exit 1
fi

echo "🏗️  Building PhoneMirror..."
cargo build --release --locked

APP_NAME="PhoneMirror"
BINARY="phonemirror"
VERSION="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json, sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "phonemirror"))')"

# Always stage and verify before touching the installed application.
mkdir -p "$ROOT/dist"
if [ -L "$ROOT/dist" ]; then
    echo "Refusing symlinked dist directory" >&2; exit 1
fi
STAGING="$(mktemp -d "$ROOT/dist/.bundle.XXXXXX")"
trap 'rm -rf "${STAGING:?}"' EXIT
BUNDLE="$STAGING/$APP_NAME.app"
echo "Creating staged bundle..."
mkdir -p "$BUNDLE/Contents/MacOS"
mkdir -p "$BUNDLE/Contents/Resources"

cp "target/release/$BINARY" "$BUNDLE/Contents/MacOS/$BINARY"

# Copy app icon into Resources (generate first with: python3 scripts/generate-icon.py)
if [ -f "assets/macos/AppIcon.icns" ]; then
    cp assets/macos/AppIcon.icns "$BUNDLE/Contents/Resources/AppIcon.icns"
    echo "🎨 App icon included"
else
    echo "⚠️  No AppIcon.icns found — run: python3 scripts/generate-icon.py"
fi

cat > "$BUNDLE/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${BINARY}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>com.muhaimin.phonemirror</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSLocalNetworkUsageDescription</key>
    <string>PhoneMirror discovers and connects to your Android device over your local Wi-Fi network for screen mirroring.</string>
    <key>NSBonjourServices</key>
    <array>
        <string>_adb-tls-pairing._tcp</string>
        <string>_adb-tls-connect._tcp</string>
    </array>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>CFBundleDocumentTypes</key>
    <array/>
</dict>
</plist>
PLIST

# Sign with entitlements for network and JIT access
if [ -f "packaging/macos/phonemirror.entitlements" ]; then
    echo "🔐 Signing with entitlements..."
    codesign --force --deep --sign - --entitlements packaging/macos/phonemirror.entitlements "$BUNDLE"
else
    echo "🔐 Signing ad-hoc..."
    codesign --force --deep --sign - "$BUNDLE"
fi


plutil -lint "$BUNDLE/Contents/Info.plist"
codesign --verify --deep --strict "$BUNDLE"

DEST="$ROOT/dist/$APP_NAME.app"
if [ -L "$DEST" ]; then echo "Refusing symlink: $DEST" >&2; exit 1; fi
rm -rf "$DEST"
mv "$BUNDLE" "$DEST"
echo "Bundle verified: $DEST ($VERSION)"

if [ "${LOCAL_BUILD:-0}" != "1" ]; then
    "$ROOT/scripts/install-macos.sh"
fi
