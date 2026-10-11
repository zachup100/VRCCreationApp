# VRCCreationApp

An open-source desktop app, published by zachLabs, for managing VRChat Unity projects and VPM packages. Its layout and flows follow the VRChat Creator Companion, and it is built on [vrc-get](https://github.com/vrc-get/vrc-get) and [Tauri](https://tauri.app).

It is not made or endorsed by VRChat. VRChat and its logos belong to VRChat Inc. This app uses its own name, icon and artwork.

## Features

- Create projects from World and Avatar templates, optionally with a preset of packages.
- Add, duplicate, back up, restore, migrate and open projects. Import the project list from the Creator Companion.
- Install, upgrade, downgrade, remove, resolve and reinstall packages with a review step before anything changes.
- Manage repositories, including private ones that need authentication headers, and hide packages you never use.
- Dark, OLED and light themes.
- Tells you when a newer version is released on GitHub. The app asks GitHub for the latest release when it starts (at most once a day) and from Settings, About. Nothing is downloaded or installed for you, and the automatic check can be turned off in the same place. GitHub sees the request the way any website sees a visit.

## Install (Windows 10 and 11)

Download the latest release and pick one:

- The installer (`.exe`) installs for the current user and needs no administrator rights.
- The MSI installer, for managed machines.
- The portable executable, a single file you can run from anywhere.

The app needs the Microsoft WebView2 Runtime, which is already part of current Windows 10 and 11. The installers fetch it if it is missing. If you run the portable file without it, the app shows a message with the official download link.

The files are not code signed yet, so Windows SmartScreen may warn you the first time. Each release includes `SHA256SUMS-windows.txt` and `SHA256SUMS-linux.txt` files so you can check your download.

Linux builds (an AppImage and a `.deb`) are attached to each release too. Windows is the supported platform, so the Linux builds are best effort: installing Unity Hub from the app is Windows only there.

Unity is not bundled. Install Unity Hub and the Unity version your projects use (2022.3.22f1 for current VRChat projects). The app finds editors that Unity Hub installed. The Unity page can also install the recommended editors (2022.3.22f1 and 2019.4.31f1, with optional Android and iOS support) through Unity Hub, and on Windows it can download and install Unity Hub itself after you confirm. The installer is checked against Unity's published checksum and signature before it runs, and Windows asks for permission first. While an install runs, projects cannot be opened or changed. Editor downloads can be limited to a chosen speed (Settings or the install dialog); Unity Hub does the downloading, so the app routes it through a small local proxy that only forwards to Unity's own hosts.

## Where your data is

Everything is stored in `%APPDATA%\net.zachlabs.vrccreationapp.main`:

- `projects.json`, `presets.json` and `preferences.json` for your lists and settings (preferences also remember when the last update check ran and the newest release it saw).
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

## Releasing (maintainers)

```
python scripts/release.py 1.0.1 "Optional description for the release notes"
```

The script checks that your work is committed and up to date, bumps the version everywhere, adds the changelog entry, commits, tags `v1.0.1` and pushes. Add `--dry-run` to see the notes and the plan first. GitHub Actions then builds the Windows and Linux files and publishes the release, with your description and one line per commit since the last release as its notes. The app's update check picks the release up once it is published. The script's own checks run with `cd scripts && python -m unittest test_release`.

## Tests

```
npm test
cd src-tauri && cargo test
```

## Licenses and credits

- Package handling comes from `vrc-get-vpm`, and the project templates come from ALCOM, both MIT licensed. See `THIRD-PARTY-NOTICES.md`.
- `THIRD-PARTY-LICENSES.txt` lists every Rust crate and npm package in the build with its license text. The same file is shown in the app under Settings, About. Regenerate it with `python3 scripts/generate-licenses.py` after changing dependencies.
