# VRCCreationApp

An open-source desktop app, published by zachLabs, for managing VRChat Unity projects and VPM packages. Its layout and flows follow the VRChat Creator Companion, and it is built on [vrc-get](https://github.com/vrc-get/vrc-get) and [Tauri](https://tauri.app).

It is not made or endorsed by VRChat. VRChat and its logos belong to VRChat Inc. This app uses its own name, icon and artwork.

## Features

- Create projects from World and Avatar templates, optionally with a preset of packages.
- Add, duplicate, back up, restore, migrate and open projects. Import the project list from the Creator Companion.
- Install, upgrade, downgrade, remove, resolve and reinstall packages with a review step before anything changes.
- Manage repositories, including private ones that need authentication headers, and hide packages you never use.
- Dark, OLED and light themes.

## Install (Windows 10 and 11)

Download the latest release and pick one:

- The installer (`.exe`) installs for the current user and needs no administrator rights.
- The MSI installer, for managed machines.
- The portable executable, a single file you can run from anywhere.

The app needs the Microsoft WebView2 Runtime, which is already part of current Windows 10 and 11. The installers fetch it if it is missing. If you run the portable file without it, the app shows a message with the official download link.

The files are not code signed yet, so Windows SmartScreen may warn you the first time. Each release includes a `SHA256SUMS.txt` file so you can check your download.

Unity is not bundled. Install Unity Hub and the Unity version your projects use (2022.3.22f1 for current VRChat projects). The app finds editors that Unity Hub installed. The Unity page can also install the recommended editors (2022.3.22f1 and 2019.4.31f1, with optional Android and iOS support) through Unity Hub, and on Windows it can download and install Unity Hub itself after you confirm. The installer is checked against Unity's published checksum and signature before it runs, and Windows asks for permission first. While an install runs, projects cannot be opened or changed. Editor downloads can be limited to a chosen speed (Settings or the install dialog); Unity Hub does the downloading, so the app routes it through a small local proxy that only forwards to Unity's own hosts.

## Where your data is

Everything is stored in `%APPDATA%\net.zachlabs.vrccreationapp.main`:

- `projects.json`, `presets.json` and `preferences.json` for your lists and settings.
- `vpm\` for repository settings and the package cache. Authentication headers you give a repository are saved here in plain text, the same way the Creator Companion stores them, so keep this folder private. The Logs page hides the query string and any credentials in repository addresses.
- `Backups\` for project backups, unless you pick another folder in Settings.

The app keeps its own data and never writes to the Creator Companion's files. The import buttons only read from them.

Unity projects with very long folder paths can hit the Windows 260 character limit. Keep projects close to the drive root, or turn on long path support in Windows.

## Build from source

You need Node.js 22 or newer, a current stable Rust toolchain, and on Windows the Visual Studio C++ build tools.

```
npm install
npm run tauri build -- --bundles nsis,msi
```

The installers appear under `src-tauri/target/release/bundle`. On Windows with Git Bash, `./build.sh` runs the same build and gathers the files, plus a portable executable and checksums, in `release/windows`. On Linux it can cross-compile the NSIS installer with `cargo-xwin`, `makensis` and `clang`.

For development on Linux or macOS, `./test-build.sh` starts the app with hot reload.

## Tests

```
npm test
cd src-tauri && cargo test
```

## Licenses and credits

- Package handling comes from `vrc-get-vpm`, and the project templates come from ALCOM, both MIT licensed. See `THIRD-PARTY-NOTICES.md`.
- `THIRD-PARTY-LICENSES.txt` lists every Rust crate and npm package in the build with its license text. The same file is shown in the app under Settings, About. Regenerate it with `python3 scripts/generate-licenses.py` after changing dependencies.
