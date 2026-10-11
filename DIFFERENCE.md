# How VRCCreationApp differs from the VRChat Creator Companion

VRCCreationApp is an independent, open-source manager for VRChat Unity projects and VPM packages, built on the open-source [vrc-get](https://github.com/vrc-get/vrc-get) package library. It is not made or endorsed by VRChat.

This page lists what it adds, what works differently, and what the Creator Companion (VCC) has that it does not. The VCC side comes from VCC's public documentation at [vcc.docs.vrchat.com](https://vcc.docs.vrchat.com) and from using VCC.

## Added in this app

### Projects
- Duplicate a project.
- Open a project with any installed Unity version, with default launch arguments and per-project launch arguments.
- Delete a project from disk, behind a confirmation checkbox. The app refuses to delete a folder that holds other listed projects, or the home folder, and checks the resolved path again before deleting.
- One-way import of your project list from VCC. Both apps keep their own project lists afterwards.

### Packages
- A review dialog before any package change, with selections that stay staged until you submit or cancel them. Removing a package lists the packages that depend on it and asks you to acknowledge.
- Resolve missing packages, reinstall a package, and clean up redundant entries in a project's package list.
- Presets: saved package lists for World or Avatar projects that can be chosen when creating a project.
- Hide individual packages and repositories you never use.
- Import repositories (including their saved headers) and local package folders from VCC.
- A repository preview before adding, which shows what the repository holds, and header values that look sensitive are masked.

### Unity
- A Unity page that lists installed editors and lets you add one by hand.
- Install the recommended editors (2022.3.22f1 and 2019.4.31f1) through Unity Hub with a progress bar and optional Android and iOS support, behind a confirmation dialog.
- On Windows, download and install Unity Hub itself after a confirmation. The installer is checked against Unity's published checksum and its Windows signature before it runs.
- A download speed limit for editor installs (Unlimited, 1, 2, 4, 8, 16, 24 or 32 MB/s).
- While an install runs, projects cannot be opened or changed, so a half-installed editor is never used.

### App
- Dark, OLED, light and system themes.
- A Logs page with backend activity and download progress.
- An update check that tells you when a new release is on GitHub, at most once a day, with a switch to turn it off. It only notifies; it does not download or install anything.
- Open source license notices inside the app, under Settings, About.

### Safety and privacy
- Repository addresses that point to this computer or a local network are blocked by default. A switch in Settings allows them for the current session only.
- Repository listings and package downloads have size and time limits, and credentials, query strings and fragments are removed from logged repository addresses.
- A restrictive content security policy for the app window.
- No analytics or telemetry code. The only outside requests are to the repositories and Unity or GitHub addresses described on this page.

## Works differently

- **Own project list.** The app keeps its own `projects.json` and never writes to VCC's files. Projects are copied from VCC once, when you press Import, and the two lists are not kept in sync.
- **Unity installs are on request.** VCC's documentation says it installs VRChat's supported Unity version and checks Unity and Unity Hub at launch. This app does not check at launch. The Unity page shows what is installed and installs one of its two recommended versions only when you press Install and confirm.
- **Templates.** This app bundles four: World and Avatar, each for Unity 2022.3.22f1 and 2019.4.31f1. VCC's documentation lists Avatar, World and UdonSharp templates.
- **Add to VCC links.** The `vcc://` links on websites are off until you register them in Settings. When one is opened, the app shows a notice and waits for you to press Open, and nothing is requested until you press Check Repository.
- **Platforms.** Windows 10 and 11 are supported. Linux builds (AppImage and deb) are attached to releases but are best effort; installing Unity Hub from the app is Windows only. VCC's documentation lists Windows 10 and 11 (64-bit) with a limited command line for Mac and Linux.
- **Authentication headers** for private repositories are stored in plain text in the app's data folder, the same way VCC stores them. The README says so.

## In VCC but not in this app

- The UdonSharp template.
- Custom project templates (VCC documents copying a template into its Templates folder and editing it).
- The command line interface that VCC documents. (vrc-get is a separate project that provides one.)
- The VRC Quick Launcher tool.
- A check for Unity and Unity Hub when the app starts, with installation guidance.
- Installing a new version of the app from inside the app. This app only tells you a new version exists.

## The same as VCC

- Create projects from templates, add existing projects, and open them in Unity.
- A project list with favorites, search, sorting and list or grid views.
- Project backups.
- Install, upgrade, downgrade and remove packages, with dependency resolution through the VPM library.
- Official and curated package repositories, plus community repositories, including ones that need authentication headers, which you can edit after adding.
- Migrate legacy SDK3 projects to the VPM format and upgrade Unity 2019 projects to Unity 2022.
- Learning links to VRChat's documentation.

---

This comparison may be incomplete or out of date. VCC changes over time, and some of its features may not be covered here. If you spot something missing or wrong, please open an issue on GitHub.
