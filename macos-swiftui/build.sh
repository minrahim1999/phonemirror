#!/bin/bash
# Build PhoneMirror macOS SwiftUI app
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="/Applications/PhoneMirror.app"

echo "🔨 Building PhoneMirror.app..."

# Create app bundle structure
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Compile
swiftc -parse-as-library \
    -o "$APP_DIR/Contents/MacOS/PhoneMirror" \
    "$SCRIPT_DIR/PhoneMirror.swift" \
    -framework Cocoa -framework SwiftUI

# Copy Info.plist
cp "$SCRIPT_DIR/Info.plist" "$APP_DIR/Contents/Info.plist"

# Generate icon
if [ -f "$SCRIPT_DIR/icon.png" ]; then
    echo "🎨 Generating app icon..."
    ICONSET="/tmp/PhoneMirror.iconset"
    mkdir -p "$ICONSET"
    sips -z 16 16     "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_16x16.png" -s format png 2>/dev/null
    sips -z 32 32     "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_16x16@2x.png" -s format png 2>/dev/null
    sips -z 32 32     "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_32x32.png" -s format png 2>/dev/null
    sips -z 64 64     "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_32x32@2x.png" -s format png 2>/dev/null
    sips -z 128 128   "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_128x128.png" -s format png 2>/dev/null
    sips -z 256 256   "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_128x128@2x.png" -s format png 2>/dev/null
    sips -z 256 256   "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_256x256.png" -s format png 2>/dev/null
    sips -z 512 512   "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_256x256@2x.png" -s format png 2>/dev/null
    sips -z 512 512   "$SCRIPT_DIR/icon.png" --out "$ICONSET/icon_512x512.png" -s format png 2>/dev/null
    cp "$SCRIPT_DIR/icon.png" "$ICONSET/icon_512x512@2x.png"
    iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/AppIcon.icns"
    rm -rf "$ICONSET"
fi

echo "✅ Built: $APP_DIR"
echo "   Binary: $APP_DIR/Contents/MacOS/PhoneMirror"
echo ""
echo "To install LaunchAgent (auto-start):"
echo "  cp $SCRIPT_DIR/com.muhaimin.phonemirror.plist ~/Library/LaunchAgents/"
echo "  launchctl load ~/Library/LaunchAgents/com.muhaimin.phonemirror.plist"