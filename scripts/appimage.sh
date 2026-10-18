
#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

VERSION=$(cat VERSION)
NIGHTLY=$(cat NIGHTLY)
RELEASE_DIR="target/packaging"
STAGING_DIR="$RELEASE_DIR/archive"
APP_ID="org.squidowl.halloy"

if [[ "$VERSION" == "$NIGHTLY" ]]; then
    ARCHIVE_NAME="halloy-nightly-x86_64.AppImage"
else
    ARCHIVE_NAME="halloy-${VERSION}-x86_64.AppImage"
fi
ARCHIVE_PATH="$RELEASE_DIR/$ARCHIVE_NAME"

case "${1:-}" in
    archive_name) echo "$ARCHIVE_NAME"; exit 0 ;;
    archive_path) echo "$ARCHIVE_PATH"; exit 0 ;;
    package) ;;
    *) echo "Usage: $0 {package|archive_name|archive_path}" >&2; exit 1 ;;
esac

[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || {
    echo "AppImage packaging requires x86_64 Linux" >&2
    exit 1
}
[[ -x "$STAGING_DIR/bin/halloy" ]] || {
    echo "Run 'bash scripts/package-linux.sh package' first" >&2
    exit 1
}

# Pinned build tool and AppImage runtime (no mutable 'continuous' assets).
APPIMAGETOOL_VERSION="1.9.1"
APPIMAGETOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
RUNTIME_VERSION="20251108"
RUNTIME_SHA256="2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"

work_dir=$(mktemp -d)
trap 'rm -rf -- "$work_dir"' EXIT
appdir="$work_dir/Halloy.AppDir"
mkdir -p "$appdir/usr"
cp -a "$STAGING_DIR/." "$appdir/usr/"

ln -s usr/bin/halloy "$appdir/AppRun"
ln -s "usr/share/applications/$APP_ID.desktop" "$appdir/$APP_ID.desktop"
ln -s "usr/share/icons/hicolor/256x256/apps/$APP_ID.png" "$appdir/$APP_ID.png"
ln -s "$APP_ID.png" "$appdir/.DirIcon"

# Verify that all AppDir entry points resolve.
[[ -x "$appdir/AppRun" && -f "$appdir/$APP_ID.desktop" && -s "$appdir/.DirIcon" ]] || {
    echo "Incomplete AppDir: missing executable, desktop file or icon" >&2
    exit 1
}

download_verified() {
    local url="$1" sha256="$2" output="$3"
    curl -fL --retry 3 "$url" -o "$output"
    printf '%s  %s\n' "$sha256" "$output" | sha256sum --check --strict
}

download_verified \
    "https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_VERSION/appimagetool-x86_64.AppImage" \
    "$APPIMAGETOOL_SHA256" "$work_dir/appimagetool.AppImage"
download_verified \
    "https://github.com/AppImage/type2-runtime/releases/download/$RUNTIME_VERSION/runtime-x86_64" \
    "$RUNTIME_SHA256" "$work_dir/runtime-x86_64"
chmod +x "$work_dir/appimagetool.AppImage"

# The existing Halloy binary is unchanged; no libraries are bundled.
APPIMAGE_ABS_PATH="$PWD/$ARCHIVE_PATH"
ARCH=x86_64 VERSION="$VERSION" APPIMAGE_EXTRACT_AND_RUN=1 \
    "$work_dir/appimagetool.AppImage" \
    --runtime-file "$work_dir/runtime-x86_64" \
    "$appdir" "$APPIMAGE_ABS_PATH"

[[ -s "$APPIMAGE_ABS_PATH" ]] || { echo "AppImage not created" >&2; exit 1; }
chmod +x "$APPIMAGE_ABS_PATH"

# Check the embedded filesystem without requiring FUSE or a GUI.
(cd "$work_dir" && "$APPIMAGE_ABS_PATH" --appimage-extract >/dev/null)
[[ -x "$work_dir/squashfs-root/AppRun" ]] || {
    echo "AppImage extraction check failed" >&2
    exit 1
}

echo "Packaged AppImage: $ARCHIVE_PATH"
