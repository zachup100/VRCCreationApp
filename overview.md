# VRCreationApp Overview

## Status

Pre-code. Only planning documents exist. Nothing has been built, run, or verified yet. Items marked "to verify" are assumptions taken from the project brief and must be checked against upstream source before being relied on.

## What this project is

An open-source, cross-platform desktop GUI that mirrors the layout and flows of the VRChat Creator Companion (VCC). It covers:

- Project list (add, open, remove Unity projects)
- Package management (install, upgrade, remove VPM packages, manage repositories)
- Unity version switching and migration
- Settings

Targets: Windows, macOS, Linux.

## Approach

- Backend: Rust with Tauri, following the approach of ALCOM (formerly vrc-get-gui). The existing `vrc-get-vpm` crate from the vrc-get repository is added as a dependency and called directly. The backend is not reimplemented.
- Frontend: any web stack hosted by Tauri. ALCOM uses Next.js, but only the backend is being reused, so the frontend choice is open. The choice, once made, is recorded in `structure.md`.
- UX reference: the ALCOM frontend (`vrc-get/vrc-get-gui`) is the primary reference for screens and flows, since it is already close to a 1:1 VCC clone.
- Spec reference: the official VPM docs at vcc.docs.vrchat.com (repo listing format, package.json extensions, CLI behavior) for anything `vrc-get-vpm` does not abstract.

## Branding and trademark

Match VCC's screen structure and flows, not its art. Do not reuse VRChat logos or trademarked assets. Create original icons and branding.

## Unity detection (to verify)

Detection should rely on Unity Hub's install registry first, then default install roots. Check `vrc-get-vpm`'s existing Unity detection module before writing any new logic. Candidate locations per OS, to be validated against the crate source because Unity Hub's format has changed before:

| OS | editors.json | Default install root |
| --- | --- | --- |
| Windows | `%APPDATA%\UnityHub\editors.json` | `C:\Program Files\Unity\Hub\Editor\` |
| macOS | `~/Library/Application Support/UnityHub/editors.json` | `/Applications/Unity/Hub/Editor/` |
| Linux | `~/.config/UnityHub/editors.json` | `~/Unity/Hub/Editor/` |

## Packaging targets

- Windows: Tauri bundler, MSI and NSIS.
- macOS: DMG. Code signing and notarization must be budgeted for, otherwise Gatekeeper warns on unsigned builds.
- Linux: AppImage and/or .deb. Check distro-specific webview dependency quirks under Tauri.
- Credential storage (Keychain, Credential Manager, libsecret) only matters if the app stores tokens. Check whether `vrc-get-vpm` already abstracts this.

## Planned order of work

1. Study `vrc-get-vpm` public API, ALCOM frontend, and the VPM spec.
2. Wire `vrc-get-vpm` into a bare Tauri shell. Prove Unity detection and package listing on the primary dev OS (Linux, Omarchy).
3. Build the VCC-matching UI.
4. Test and fix packaging on the other two OSes.

## Project rules

See `rules.md`. Highlights: no em-dashes, no code comments, commit after each task, ask before building or testing, keep this file and `structure.md` current.
