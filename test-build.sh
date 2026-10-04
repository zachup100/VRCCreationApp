#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

if [ "$(uname -s)" != "Linux" ]; then
    echo "This script is for Linux development machines."
    exit 1
fi

for tool in node npm cargo; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Missing required tool: $tool"
        exit 1
    fi
done

if [ ! -d node_modules ]; then
    npm install
fi

exec npm run tauri dev
