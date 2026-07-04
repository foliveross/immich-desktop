# Immich Desktop

A lightweight, privacy-first **Windows GUI wrapper** for the [official Immich CLI](https://github.com/immich-app/immich/pkgs/container/immich-cli). Built with [Tauri](https://tauri.app/) (Rust + React), Immich Desktop gives you a native dashboard, background sync, and system tray control — without storing any private data in this repository.

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![License](https://img.shields.io/badge/license-MIT-green)

---

## Features

- **Auto-Discovery** — mDNS/Bonjour scan plus local subnet probe to find Immich servers on your network.
- **HTTP Handshake Auth** — Verifies server connectivity (HTTP 200/204) before any CLI process is spawned.
- **Process Lifecycle Manager** — Hidden CLI subprocesses, lock file, and graceful shutdown (fixes CMD window loops).
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
2. Download `ImmichDesktop-vX.Y.Z.exe` (portable) or `ImmichDesktop-Setup-vX.Y.Z.exe` (installer).
3. Run the installer or double-click the portable executable.

#### Option B: Build from Source (Local)

```powershell
git clone https://github.com/foliveross/immich-desktop.git
cd immich-desktop
npm install

# Validate environment before building
npm run build:check

# Development
npm run tauri dev

# Production build (validates env + compiles)
npm run build:release

# Full preflight: build + verify ImmichDesktop-v{version}.exe exists
npm run build:preflight
```

Production artifacts:

```
dist/release/ImmichDesktop-v{version}.exe     # Renamed portable (preflight script)
src-tauri/target/release/ImmichDesktop.exe    # Raw Tauri output
src-tauri/target/release/bundle/nsis/         # NSIS installer
src-tauri/target/release/bundle/msi/          # MSI package
```

---

## Quick Start — Auto-Discovery

1. Launch **Immich Desktop**. The setup wizard opens on first run.
2. Click **Scan Network** — the app searches via:
   - **mDNS/Bonjour** (`_immich._tcp`, `_http._tcp`, `_https._tcp`)
   - **Subnet scan** on common Immich ports (2283, 3001, 8080, 80, 443)
3. Select your server from the **Discovered servers** dropdown.
4. Enter your **API key** (Account Settings → API Keys in Immich web UI).
5. Click **Run Handshake Test** — the app calls `/api/server/ping` and requires **HTTP 200 or 204** before proceeding.
6. Click **Open Dashboard** to finalize CLI login and enter the main interface.

> The handshake uses direct HTTP (reqwest) — **no CLI subprocess is spawned** during connection testing.

---

## Manual Setup

Use manual entry when auto-discovery fails:

| Scenario | What to do |
|---|---|
| **Remote server** (different network) | Check "Enter server URL manually" and paste your public URL |
| **Reverse proxy** | Use the external URL your proxy exposes (e.g. `https://photos.example.com`) |
| **Different subnet/VLAN** | Enter the server's IP directly: `http://192.168.1.10:2283` |
| **Docker on localhost** | Try `http://127.0.0.1:2283` or your mapped host port |

The app normalizes URLs automatically (appends `/api` if missing).

---

## First-Run Setup (Step-by-Step)

1. **Step 1 — Server**: Auto-discover or enter URL manually.
2. **Step 2 — API Key**: Paste your Immich API key.
3. **Step 3 — Handshake**: HTTP connectivity test (200 OK required).
4. **Step 4 — Done**: CLI login + open dashboard.

Credentials are stored in **Windows Credential Manager** — never in this repository.

---

## Troubleshooting

### Infinite CMD windows / zombie CLI processes

Fixed: CLI processes run with `CREATE_NO_WINDOW` (0x08000000), are tracked by a process manager, and terminated on quit via `taskkill /PID /T /F`. Network trigger checks (`netsh`, `powershell`) also use hidden subprocesses.

### App already running

If you see "Immich Desktop is already running", check the system tray or delete:

```
%APPDATA%\ImmichDesktop\immich-desktop-app.lock
```

### CLI lock file (`immich-desktop.lock`)

If the app crashes mid-upload, a lock file may block new CLI operations:

```
%APPDATA%\ImmichDesktop\immich-desktop.lock
```

**To recover:**
1. Close all Immich Desktop instances (check system tray).
2. Delete `immich-desktop.lock` from the path above.
3. Relaunch the app.

Or from PowerShell:
```powershell
Remove-Item "$env:APPDATA\ImmichDesktop\immich-desktop.lock" -ErrorAction SilentlyContinue
```

### Logs

```
%APPDATA%\ImmichDesktop\logs\
```

Open from the app sidebar (**View Logs**) or system tray.

### Handshake fails

- Verify the server URL is reachable in a browser.
- Confirm the API key has upload permissions.
- Check firewall rules for port 2283 (default Immich port).
- For HTTPS with self-signed certs, ensure the certificate is trusted by Windows.

---

## Version History

Releases are **tag-triggered** — each Git tag (`v0.1.0`, `v0.2.0`, …) produces a GitHub Release with auto-generated notes.

View all releases: [GitHub Releases](https://github.com/foliveross/immich-desktop/releases)

To create a new release:
```powershell
git tag v0.1.1
git push origin v0.1.1
```

Or trigger manually via **Actions → Release → Run workflow** with a tag name.

---

## Configuration Guide

All user configuration lives in:

```
%APPDATA%\ImmichDesktop\
├── config.json          # App settings (no API keys)
├── .immich\             # Immich CLI auth directory (IMMICH_CONFIG_DIR)
│   └── auth.yml         # CLI session (managed by immich login)
├── logs\                # Application logs
├── immich-desktop.lock      # CLI operation lock (delete if app crashes)
├── immich-desktop-app.lock  # Single-instance lock (delete if app won't start)
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

## CI/CD — Tag-Triggered Releases

Builds run **only when a version tag is pushed** (not on every commit), saving GitHub Actions minutes.

**Trigger:** Push a tag matching `v*` (e.g. `v0.1.1`) or use **workflow_dispatch** manually.

**Pipeline** (`.github/workflows/release.yml`):

1. **Validate** — `npm run build:check` verifies Node, Rust, and version alignment
2. **Cache** — `swatinem/rust-cache` + npm cache for faster rebuilds
3. **Build** — `npm run tauri build` on `windows-latest`
4. **Package** — Renames artifacts to `ImmichDesktop-v{version}.exe`, `ImmichDesktop-Setup-v{version}.exe`
5. **Release** — `softprops/action-gh-release` with auto-generated release notes

No user-specific configuration is included in build artifacts.

### npm Scripts Reference

| Script | Purpose |
|---|---|
| `npm run build:check` | Pre-flight environment validation (CI + local) |
| `npm run build:release` | Check + frontend build + Tauri build |
| `npm run build:preflight` | Full local build + verify `ImmichDesktop-v{version}.exe` |

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
| `connection.rs` | HTTP handshake (`/api/server/ping`) |
| `discovery.rs` | mDNS + subnet server discovery |
| `process_manager.rs` | CLI lock file, hidden subprocesses, shutdown |
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
