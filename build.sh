#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$ROOT/release"
cd "$ROOT"

HOST="$(uname -s)"
case "$HOST" in
    Linux*) HOST_OS="linux" ;;
    Darwin*) HOST_OS="mac" ;;
    MINGW* | MSYS* | CYGWIN*) HOST_OS="windows" ;;
    *) HOST_OS="unknown" ;;
esac

DONE=()
SKIPPED=()
FAILED=()

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required tool: $1 ($2)"
        return 1
    fi
}

prepare() {
    need node "install from https://nodejs.org" || return 1
    need npm "install from https://nodejs.org" || return 1
    need cargo "install from https://rustup.rs" || return 1
    if [ ! -d node_modules ]; then
        npm install || return 1
    fi
}

VERSION="$(node -p "require('./src-tauri/tauri.conf.json').version" 2>/dev/null || echo "0.0.0")"

collect() {
    local label="$1"
    shift
    mkdir -p "$OUT/$label"
    local found=0
    for pattern in "$@"; do
        for file in $pattern; do
            if [ -f "$file" ]; then
                cp "$file" "$OUT/$label/"
                found=1
            fi
        done
    done
    [ "$found" -eq 1 ]
}

portable() {
    local source="$1"
    if [ -f "$source" ]; then
        mkdir -p "$OUT/windows"
        cp "$source" "$OUT/windows/VRCCreationApp-$VERSION-portable.exe"
    fi
}

checksums() {
    local label="$1"
    if command -v sha256sum >/dev/null 2>&1 && [ -d "$OUT/$label" ]; then
        (cd "$OUT/$label" && rm -f SHA256SUMS.txt && sha256sum -- * > SHA256SUMS.txt)
    fi
}

build_linux() {
    if [ "$HOST_OS" != "linux" ]; then
        SKIPPED+=("Linux: must be built on a Linux machine")
        return
    fi
    if npm run tauri build -- --bundles appimage,deb; then
        local base="src-tauri/target/release/bundle"
        if collect linux "$base/appimage/*.AppImage" "$base/deb/*.deb"; then
            checksums linux
            DONE+=("Linux: $OUT/linux")
        else
            FAILED+=("Linux: build finished but no artifacts were found")
        fi
    else
        FAILED+=("Linux: build failed")
    fi
}

build_mac() {
    if [ "$HOST_OS" != "mac" ]; then
        SKIPPED+=("Mac: must be built on a Mac (Apple toolchain and signing are required)")
        return
    fi
    if npm run tauri build -- --bundles dmg; then
        local base="src-tauri/target/release/bundle"
        if collect mac "$base/dmg/*.dmg"; then
            checksums mac
            DONE+=("Mac: $OUT/mac")
        else
            FAILED+=("Mac: build finished but no artifacts were found")
        fi
    else
        FAILED+=("Mac: build failed")
    fi
}

build_windows() {
    case "$HOST_OS" in
        windows)
            if npm run tauri build -- --bundles nsis,msi; then
                local base="src-tauri/target/release"
                portable "$base/vrccreationapp.exe"
                if collect windows "$base/bundle/nsis/*.exe" "$base/bundle/msi/*.msi"; then
                    checksums windows
                    DONE+=("Windows: $OUT/windows (NSIS installer, MSI and portable exe)")
                else
                    FAILED+=("Windows: build finished but no artifacts were found")
                fi
            else
                FAILED+=("Windows: build failed")
            fi
            ;;
        linux | mac)
            local missing=0
            need cargo-xwin "cargo install --locked cargo-xwin" || missing=1
            need makensis "install NSIS from your package manager" || missing=1
            need clang "install LLVM and clang from your package manager" || missing=1
            if [ "$missing" -eq 1 ]; then
                SKIPPED+=("Windows: cross-compiling from $HOST_OS needs the missing tools listed above (the MSI can only be built on Windows)")
                return
            fi
            rustup target add x86_64-pc-windows-msvc >/dev/null 2>&1
            if npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis; then
                local base="src-tauri/target/x86_64-pc-windows-msvc/release"
                portable "$base/vrccreationapp.exe"
                if collect windows "$base/bundle/nsis/*.exe"; then
                    checksums windows
                    DONE+=("Windows: $OUT/windows (cross-compiled, NSIS installer and portable exe)")
                else
                    FAILED+=("Windows: build finished but no artifacts were found")
                fi
            else
                FAILED+=("Windows: cross-compile failed")
            fi
            ;;
        *)
            SKIPPED+=("Windows: unsupported host system")
            ;;
    esac
}

echo "Select the platform to compile for:"
echo "  1) Windows"
echo "  2) Mac"
echo "  3) Linux"
echo "  4) All of the above"
read -r -p "Choice [1-4]: " CHOICE

case "$CHOICE" in
    1 | 2 | 3 | 4) ;;
    *)
        echo "Invalid choice: $CHOICE"
        exit 1
        ;;
esac

prepare || exit 1

case "$CHOICE" in
    1) build_windows ;;
    2) build_mac ;;
    3) build_linux ;;
    4)
        build_linux
        build_mac
        build_windows
        ;;
esac

echo
echo "Build summary"
for line in "${DONE[@]+"${DONE[@]}"}"; do echo "  done:    $line"; done
for line in "${SKIPPED[@]+"${SKIPPED[@]}"}"; do echo "  skipped: $line"; done
for line in "${FAILED[@]+"${FAILED[@]}"}"; do echo "  failed:  $line"; done

[ "${#FAILED[@]}" -eq 0 ] && [ "${#DONE[@]}" -gt 0 ]
