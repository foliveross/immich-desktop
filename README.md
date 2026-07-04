# Immich Desktop

A lightweight, privacy-first **Windows GUI wrapper** for the [official Immich CLI](https://github.com/immich-app/immich/pkgs/container/immich-cli). Built with [Tauri](https://tauri.app/) (Rust + React), Immich Desktop gives you a native dashboard, background sync, and system tray control — without storing any private data in this repository.

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![License](https://img.shields.io/badge/license-MIT-green)

---

## Features

- **Real-time Dashboard** — Global progress bars, live upload speed (MB/s), ETA, and a detailed file-activity list (queued, uploading, skipped, failed).
- **Smart Sync Triggers** — Sync only on specific Wi-Fi networks, when plugged in, or during configured time windows.
- **Watch Mode** — Background folder monitoring with debounced auto-upload.
- **Retry Queue** — Automatically track failed uploads and retry them in one click.
- **Conflict Resolution** — Side-by-side UI for resolving upload conflicts.
- **System Tray** — Run in the background with Pause/Resume, Open Web UI, and View Logs from the tray menu.
- **Privacy First** — API keys stored in Windows Credential Manager; configuration in `%APPDATA%\ImmichDesktop\`.

---

## Getting Started

### Requirements

| Requirement | Notes |
|---|---|
| **Windows 10/11** | 64-bit |
| **Immich CLI** | Install via `npm install -g @immich/cli` or use the [Docker CLI image](https://github.com/immich-app/immich/pkgs/container/immich-cli) with Node.js on PATH |
| **Node.js 20+** | Required by `@immich/cli` |
| **Immich Server** | Self-hosted instance with API access |

### Installation

#### Option A: Portable `.exe` (Recommended)

1. Go to [Releases](https://github.com/YOUR_ORG/immich-desktop/releases).
2. Download `Immich Desktop_x.x.x_x64-setup.exe` (installer) or the portable `.exe`.
3. Run the installer or double-click the portable executable.

#### Option B: Build from Source

```powershell
# Prerequisites: Rust, Node.js 20+, npm
git clone https://github.com/YOUR_ORG/immich-desktop.git
cd immich-desktop
npm install
npm run tauri dev      # Development
npm run tauri build    # Production build
```

Production artifacts are written to:

```
src-tauri/target/release/bundle/nsis/   # Installer
src-tauri/target/release/bundle/msi/    # MSI package
src-tauri/target/release/               # Portable exe
```

---

## First-Run Setup

1. **Launch Immich Desktop** — The setup wizard opens automatically on first run.
2. **Enter your server URL** — e.g. `https://photos.example.com` (the app appends `/api`).
3. **Obtain an API key** from your Immich server:
   - Open your Immich web UI → **Account Settings** → **API Keys**
   - Click **New API Key**, set a name and permissions (upload access required)
   - Copy the generated key
4. **Paste the API key** in the wizard and click **Connect & Test**.
5. Your credentials are saved to **Windows Credential Manager** — never to disk in plain text and never to this repository.

---

## Configuration Guide

All user configuration lives in:

```
%APPDATA%\ImmichDesktop\
├── config.json          # App settings (no API keys)
├── .immich\             # Immich CLI auth directory (IMMICH_CONFIG_DIR)
│   └── auth.yml         # CLI session (managed by immich login)
├── logs\                # Application logs
├── retry_queue.json     # Failed upload retry queue
└── conflicts.json       # Unresolved conflict records
```

> **Security:** API keys are stored exclusively in **Windows Credential Manager** under the service name `immich-desktop`. The `config.json` file contains only non-sensitive settings.

### Settings Reference (`config.json`)

| Field | Description |
|---|---|
| `server_url` | Immich API endpoint (e.g. `https://photos.example.com/api`) |
| `watch_folders` | List of local folders monitored in Watch Mode |
| `watch_mode.enabled` | Enable/disable background folder monitoring |
| `watch_mode.debounce_ms` | Milliseconds to wait after a file change before uploading |
| `sync_triggers.wifi_only` | Only sync when connected via Wi-Fi |
| `sync_triggers.allowed_networks` | Whitelist of Wi-Fi SSIDs (empty = all networks) |
| `sync_triggers.require_plugged_in` | Only sync when laptop is plugged in |
| `sync_triggers.schedule` | Time window restriction (24h format) |
| `upload_options.recursive` | Include subfolders in uploads |
| `upload_options.concurrency` | Parallel upload threads (default: 4) |
| `upload_options.ignore_patterns` | Glob patterns to skip (e.g. `**/Raw/**`) |
| `cli_path` | Optional explicit path to `immich` binary |

### Watch Mode

1. Open **Settings → Watch Mode**.
2. Enable **background folder monitoring**.
3. Click **Add Watch Folder** and select directories to monitor.
4. New files are debounced and uploaded automatically when sync triggers allow.

The Immich CLI also supports its own `--watch` flag; Immich Desktop uses the native filesystem watcher for finer control over triggers and debouncing.

### Sync Schedules & Triggers

Configure contextual sync in **Settings → Sync Triggers**:

- **Wi-Fi only** — Prevents uploads over metered/mobile connections.
- **Allowed networks** — Restrict sync to home/office SSIDs.
- **Require plugged in** — Avoid battery drain on laptops.
- **Time window** — e.g. upload only between 22:00–06:00.

---

## Technical Reference

### How the App Interfaces with Immich CLI

Immich Desktop spawns the official `@immich/cli` binary as a child process:

```
immich upload --recursive --json-output --concurrency 4 <paths>
```

Environment variables passed to every CLI invocation:

| Variable | Source |
|---|---|
| `IMMICH_INSTANCE_URL` | `config.json → server_url` |
| `IMMICH_API_KEY` | Windows Credential Manager |
| `IMMICH_CONFIG_DIR` | `%APPDATA%\ImmichDesktop\.immich\` |

CLI detection order:

1. `cli_path` from settings (if set and file exists)
2. `immich` on system PATH
3. `npx --yes @immich/cli` as fallback

Authentication uses `immich login <url> <key>`, storing session data in the configured `IMMICH_CONFIG_DIR`.

Progress is parsed from CLI stdout/stderr in real time and emitted to the React dashboard via Tauri events.

### Log Location

```
%APPDATA%\ImmichDesktop\logs\
```

Open from the app: **Sidebar → View Logs** or **System Tray → View Logs**.

### Security Notes

| Data | Storage Location |
|---|---|
| API Key | Windows Credential Manager (`immich-desktop` service) |
| Server URL | `%APPDATA%\ImmichDesktop\config.json` |
| CLI auth session | `%APPDATA%\ImmichDesktop\.immich\auth.yml` |
| Upload logs | `%APPDATA%\ImmichDesktop\logs\` |

**This repository contains zero user credentials.** Never commit `config.json`, `.env`, or `auth.yml` files. The `.gitignore` excludes common secret patterns.

To revoke access: delete the API key in Immich web UI and run **Settings → Clear Credentials** (or use Windows Credential Manager directly).

---

## System Tray

When you close the window, Immich Desktop minimizes to the system tray and continues running (Watch Mode stays active).

| Menu Item | Action |
|---|---|
| Pause Sync | Pauses the current upload |
| Resume Sync | Resumes a paused upload |
| Open Web UI | Opens your Immich server in the browser |
| View Logs | Opens the logs folder in Explorer |
| Show Window | Restores the main window |
| Quit | Exits the application |

---

## CI/CD — Automated Releases

Every push to `main` triggers the release pipeline (`.github/workflows/release.yml`):

1. **Version bump** — `mathieudutour/github-tag-action` creates a semantic version tag (`v0.1.1`, `v0.2.0`, etc.).
2. **Build** — `npm run tauri build` on `windows-latest`.
3. **Release** — `softprops/action-gh-release` attaches the NSIS installer, MSI, and portable `.exe` to the GitHub Release.

No user-specific configuration is included in build artifacts. Optional code-signing secrets can be added via GitHub Secrets without affecting the open-source repo.

### Commit Message Version Bumps

The tag action respects conventional commit prefixes:

| Commit prefix | Version bump |
|---|---|
| `#major` or `BREAKING CHANGE` | Major |
| `#minor` | Minor |
| *(default)* | Patch |

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository and create a feature branch.
2. Follow existing code style (Rust + TypeScript/React).
3. **Never commit secrets** — use `%APPDATA%` paths and Credential Manager for sensitive data.
4. Test locally with `npm run tauri dev`.
5. Open a pull request with a clear description of changes.

### Development Setup

```powershell
cd immich-desktop
npm install
npm run tauri dev
```

Rust backend lives in `src-tauri/src/`. Frontend lives in `src/`.

Key modules:

| Module | Purpose |
|---|---|
| `config.rs` | `%APPDATA%` config load/save |
| `credentials.rs` | Windows Credential Manager |
| `cli.rs` | Immich CLI process spawning & progress parsing |
| `watch.rs` | Filesystem watcher for Watch Mode |
| `sync_triggers.rs` | Network/power/schedule gating |
| `retry_queue.rs` | Failed upload persistence |
| `commands.rs` | Tauri IPC command handlers |

---

## License

MIT — see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Immich](https://immich.app/) — Self-hosted photo and video backup
- [Immich CLI](https://github.com/immich-app/immich/pkgs/container/immich-cli) — Official command-line uploader
- [Tauri](https://tauri.app/) — Lightweight desktop app framework
