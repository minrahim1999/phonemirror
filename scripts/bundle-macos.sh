#!/bin/bash
set -e

echo "🏗️  Building PhoneMirror..."
cargo build --release

APP_NAME="PhoneMirror"
BINARY="phonemirror"
BUNDLE="/Applications/${APP_NAME}.app"
VERSION="2.0.0"

echo "📦 Creating .app bundle..."
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

echo "✅ Installed to $BUNDLE"
echo "   Binary: $(du -h "$BUNDLE/Contents/MacOS/$BINARY" | cut -f1)"
echo ""
echo "Run: open $BUNDLE"
