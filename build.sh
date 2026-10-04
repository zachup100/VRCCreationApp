#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$ROOT/release/windows"
cd "$ROOT"

HOST="$(uname -s)"
case "$HOST" in
    Linux*) HOST_OS="linux" ;;
    Darwin*) HOST_OS="mac" ;;
    MINGW* | MSYS* | CYGWIN*) HOST_OS="windows" ;;
    *) HOST_OS="unknown" ;;
esac

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
    mkdir -p "$OUT"
    local found=0
    for pattern in "$@"; do
        for file in $pattern; do
            if [ -f "$file" ]; then
                cp "$file" "$OUT/"
                found=1
            fi
        done
    done
    [ "$found" -eq 1 ]
}

portable() {
    local source="$1"
    if [ -f "$source" ]; then
        mkdir -p "$OUT"
        cp "$source" "$OUT/VRCCreationApp-$VERSION-portable.exe"
    fi
}

checksums() {
    if command -v sha256sum >/dev/null 2>&1; then
        (cd "$OUT" && sha256sum * > SHA256SUMS.txt)
    fi
}

build_windows() {
    case "$HOST_OS" in
        windows)
            if npm run tauri build -- --bundles nsis,msi; then
                local base="src-tauri/target/release"
                portable "$base/vrccreationapp.exe"
                collect "$base/bundle/nsis/*.exe" "$base/bundle/msi/*.msi"
            else
                return 1
            fi
            ;;
        linux | mac)
            local missing=0
            need cargo-xwin "cargo install --locked cargo-xwin" || missing=1
            need makensis "install NSIS from your package manager" || missing=1
            need clang "install LLVM and clang from your package manager" || missing=1
            if [ "$missing" -eq 1 ]; then
                echo "Cross-compiling from $HOST_OS needs the tools above. The MSI installer can only be built on Windows."
                return 1
            fi
            rustup target add x86_64-pc-windows-msvc >/dev/null 2>&1
            if npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis; then
                local base="src-tauri/target/x86_64-pc-windows-msvc/release"
                portable "$base/vrccreationapp.exe"
                collect "$base/bundle/nsis/*.exe"
            else
                return 1
            fi
            ;;
        *)
            echo "Unsupported host system for building."
            return 1
            ;;
    esac
}

prepare || exit 1

if build_windows; then
    checksums
    echo
    echo "Windows build finished: $OUT"
    ls -1 "$OUT"
else
    echo
    echo "Windows build failed."
    exit 1
fi
