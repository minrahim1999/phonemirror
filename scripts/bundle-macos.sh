#!/bin/bash
set -e

echo "🏗️  Building PhoneMirror..."
cargo build --release

APP_NAME="PhoneMirror"
BINARY="phonemirror"
VERSION="2.1.0"

# Support LOCAL_BUILD=1 to build .app in project dir instead of /Applications
if [ "${LOCAL_BUILD:-0}" = "1" ]; then
    BUNDLE="$(pwd)/${APP_NAME}.app"
else
    BUNDLE="/Applications/${APP_NAME}.app"
fi

echo "📦 Creating .app bundle at $BUNDLE..."
rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS"
mkdir -p "$BUNDLE/Contents/Resources"

cp "target/release/$BINARY" "$BUNDLE/Contents/MacOS/$BINARY"

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
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
PLIST

# Sign with entitlements for network and JIT access
if [ -f "phonemirror.entitlements" ]; then
    echo "🔐 Signing with entitlements..."
    codesign --force --deep --sign - --entitlements phonemirror.entitlements "$BUNDLE"
else
    echo "🔐 Signing ad-hoc..."
    codesign --force --deep --sign - "$BUNDLE"
fi

echo "✅ Bundle created at $BUNDLE"
echo "   Binary: $(du -h "$BUNDLE/Contents/MacOS/$BINARY" | cut -f1)"

if [ "${LOCAL_BUILD:-0}" != "1" ]; then
    echo ""
    echo "Run: open $BUNDLE"
fi