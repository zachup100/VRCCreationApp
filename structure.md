# VRCreationApp Structure

Internal reference for agents working on this codebase. Keep it accurate. Update it whenever files, modules, or dependencies change.

## Current state

No source code exists yet. The repository contains only:

- `rules.md`: mandatory working rules
- `overview.md`: project description and goals
- `structure.md`: this file

## Planned architecture

```
Frontend (web stack, TBD)
        |  Tauri invoke / events
Tauri commands (Rust, src-tauri)
        |  direct crate calls
vrc-get-vpm (external crate)
        |
Filesystem: VPM manifests, Unity projects, Unity Hub registry, repo caches
```

Rules of the layering:

- The frontend never touches the filesystem or network for VPM work. It calls Tauri commands only.
- Tauri commands are thin wrappers. Logic that belongs in `vrc-get-vpm` stays there.
- Only write new backend logic where `vrc-get-vpm` has no equivalent.

## Planned layout

Not created yet. Proposed, to be adjusted once the stack is chosen:

```
VRCreationApp/
  rules.md
  overview.md
  structure.md
  src-tauri/
    Cargo.toml          vrc-get-vpm dependency declared here
    tauri.conf.json     bundle targets, window config
    src/
      main.rs           Tauri entry point and command registration
      commands/         one module per feature area (projects, packages, repos, unity, settings)
  src/                  frontend source (framework TBD)
```

## Dependencies (planned)

| Dependency | Source | Purpose |
| --- | --- | --- |
| Tauri | crates.io | desktop shell and IPC |
| vrc-get-vpm | vrc-get repo, git dependency | VPM resolution, manifest IO, project scanning, install/upgrade/remove |
| Frontend framework | TBD | UI |

The exact vrc-get-vpm revision or tag to pin is undecided. Pin it explicitly when added.

## Open items to verify before coding

- Public API surface of `vrc-get-vpm` and which features need wrapping.
- Whether `vrc-get-vpm` already contains Unity detection and which paths and file formats it reads. Compare with the candidate paths in `overview.md`.
- Whether `vrc-get-vpm` abstracts credential storage.
- How ALCOM structures its Tauri commands and frontend screens, as a UX and IPC reference.
- Tauri system dependencies on Linux (webkit2gtk and related) for the dev machine.

## Conventions

- No em-dashes in any file.
- No comments in code files.
- Commit after each completed task.
- Do not build or test without asking the user. Syntax checking is allowed.
- Original icons and branding only. No VRChat trademarked assets.
