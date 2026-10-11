# Changelog

## 1.0.0

First release. Windows 10 and 11 only.

### Projects
- Project list with search, sorting, list and grid views, favorites and a green Migrate button for legacy projects.
- Create projects from bundled World and Avatar templates for Unity 2022.3.22f1 and 2019.4.31f1, with an optional preset of packages.
- Add existing projects by folder picker or by dragging folders onto the window, import the project list from the VRChat Creator Companion, duplicate projects, and open them with any installed Unity version and custom launch arguments.
- Delete a project from disk, behind a confirmation checkbox, from the Dangerous Actions menu. Add Unity editors that were not found automatically.
- Back up projects to zip files, browse the app's own backups, and restore or delete them.
- Migrate legacy SDK3 projects to the VPM package system and Unity 2019 projects to Unity 2022, with an optional backup first.

### Packages
- See installed and available packages per project, with update tags, search, repository filters and an option to show only packages that support the project type.
- Install, upgrade, downgrade and remove packages through a review dialog. Removing a package also lists the packages that depend on it and asks for acknowledgement.
- Resolve missing packages, reinstall a package, and clean up redundant entries in a project's package list.
- Add community repositories (including ones that need authentication headers), show or hide repositories, hide individual packages, and import repositories and package folders from the Creator Companion.
- Presets: saved package lists for World or Avatar projects that can be picked when creating a project.
- Edit a community repository's URL and authentication headers.
- Optional support for the Add to VCC buttons on websites: Settings has a Register Deep Link button that makes the app handle vcc:// links by opening the Add Repository window with the link prefilled. It is off until you register it.

### Unity
- A Unity page lists installed editors, lets you add one by hand, and installs the recommended VRChat editors (2022.3.22f1 and 2019.4.31f1) through Unity Hub, with optional Android and iOS support and a progress bar.
- On Windows it can also download and install Unity Hub after a confirmation. The installer is checked against Unity's published checksum and signature before it runs, and Windows asks for permission first.
- An editor download can be limited to a chosen speed (Unlimited, 1, 2, 4, 8, 16, 24 or 32 MB/s) from the install dialog or Settings.
- While an install runs, projects cannot be opened or changed.

### App
- Dark, OLED, light and system themes.
- Logs page with backend activity, download progress for package installs, keyboard and screen reader support for dialogs, menus and option groups.
- Open source license notices are included in the app under Settings, About.
- The app checks GitHub for a newer release when it starts (at most once a day) and from Settings, About, with a switch to turn the automatic check off. It only tells you and links the release page.

### Notes
- The installers download the Microsoft WebView2 Runtime if it is missing. The portable executable shows a message with a download link instead.
- This app is not made or endorsed by VRChat.
