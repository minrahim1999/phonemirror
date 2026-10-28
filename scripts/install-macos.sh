#!/bin/bash
# Install only a verified staged bundle; retain the previous app for rollback.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$ROOT/dist/PhoneMirror.app"
DEST="/Applications/PhoneMirror.app"
BACKUP="/Applications/PhoneMirror.previous.app"
if [ "$(uname -s)" != Darwin ]; then echo "macOS required" >&2; exit 1; fi
for path in "$SOURCE" "$DEST" "$BACKUP"; do
    if [ -L "$path" ]; then echo "Refusing symlink: $path" >&2; exit 1; fi
done
if pgrep -x phonemirror >/dev/null; then
    echo "Quit PhoneMirror before replacing the installed app." >&2; exit 1
fi
codesign --verify --deep --strict "$SOURCE"
plutil -lint "$SOURCE/Contents/Info.plist"
if [ -e "$BACKUP" ]; then
    echo "Previous backup exists: $BACKUP. Move it aside before installing." >&2; exit 1
fi
STAGING="$(mktemp -d /Applications/.phonemirror-install.XXXXXX)"
trap 'rm -rf "${STAGING:?}"' EXIT
ditto "$SOURCE" "$STAGING/PhoneMirror.app"
codesign --verify --deep --strict "$STAGING/PhoneMirror.app"
diff -rq "$SOURCE" "$STAGING/PhoneMirror.app"
if [ -e "$DEST" ]; then mv "$DEST" "$BACKUP"; fi
if ! mv "$STAGING/PhoneMirror.app" "$DEST"; then
    if [ -e "$BACKUP" ]; then mv "$BACKUP" "$DEST"; fi
    exit 1
fi
codesign --verify --deep --strict "$DEST"
diff -rq "$SOURCE" "$DEST"
echo "Installed verified bundle: $DEST"
if [ -e "$BACKUP" ]; then echo "Rollback copy: $BACKUP"; fi
