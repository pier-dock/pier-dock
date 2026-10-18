<p align="center">
  <img src="app-icon.svg" width="96" height="96" alt="Pier logo">
</p>

<h1 align="center">Pier</h1>

<p align="center">
  A customizable dock for Windows, macOS, and Linux (GNOME and KDE Plasma).
</p>

<p align="center">
  <a href="https://github.com/pier-dock/pier-dock/releases/latest"><img src="https://img.shields.io/github/v/release/pier-dock/pier-dock?label=release" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/pier-dock/pier-dock" alt="License: GPL-3.0"></a>
  <img src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-informational" alt="Platforms: Windows, macOS, and Linux">
</p>

<p align="center">
  <b>English</b> · <a href="README.pt-BR.md">Português</a>
</p>

Pier replaces your system's taskbar or dock with a single bar that holds your pinned and running apps alongside live widgets. It's built with [Tauri 2](https://tauri.app) (Rust) and React, and draws native glass on Windows 11, macOS, and Linux compositors that support blur.

## Features

- **Apps:** Pinned and running apps with magnification, drag-to-reorder, window lists, and dividers.
- **Media player:** Shows what's playing in Spotify, YouTube Music, and other players, with playback controls.
- **GitHub widgets:** Pull requests waiting for your review, your open pull requests, recent commits, and your contribution graph.
- **System widgets:** Network, volume (with the output device and per-app volume), Bluetooth (with connected devices), battery, clock, system usage (CPU and memory with the last minute of history), and, on Windows, the tray arrow that opens the hidden icons.
- **Productivity widgets:** Weather, a Pomodoro timer, your next calendar event with a join button for video calls, and your Claude Code usage.
- **Keyboard shortcuts:** Open or focus the app in position 1 to 9 from anywhere.
- **Automatic updates:** Pier checks for new versions and installs them in one click.
- **English and Portuguese:** The interface follows your system language. To change it, go to **Settings** > **General** > **Language**.

> [!NOTE]
> Pier's interface is currently available in Portuguese only. This README shows each label as it appears on screen, followed by its English translation.

## Installation

Download the installer for your platform from the [latest release](https://github.com/pier-dock/pier-dock/releases/latest).

| Platform | File | Notes |
| --- | --- | --- |
| Windows 10 and 11 | `Pier_x.y.z_x64-setup.exe` | Installs to `%LOCALAPPDATA%\Pier` without admin rights. |
| macOS 12 or later | `Pier_x.y.z_universal.dmg` | Runs natively on Apple silicon and Intel Macs. |
| Linux (GNOME 45 to 49, KDE Plasma 6) | `Pier_x.y.z_amd64.deb` or `Pier_x.y.z_amd64.AppImage` | Install the `.deb` with `sudo apt install ./Pier_x.y.z_amd64.deb`, or make the AppImage executable with `chmod +x` and run it. On KDE Plasma nothing else is needed; on GNOME, install the extension from Pier's settings. |

### First launch

The installers aren't code-signed yet, so your system warns you the first time you open Pier.

- **Windows:** When SmartScreen appears, select **More info**, and then select **Run anyway**.
- **macOS:** Drag Pier to **Applications** and open it. When macOS says it can't verify the app, go to **System Settings** > **Privacy & Security** and select **Open Anyway**. Alternatively, run `xattr -dr com.apple.quarantine /Applications/Pier.app` in Terminal.
- **Linux:** On GNOME, open Pier's settings and select **Install extension**, and then log out and log back in. The extension lets Pier see and focus windows, reserve screen space, hide the Dash, and register shortcuts. On KDE Plasma there is nothing to install: in an X11 session the dock lists windows, reserves screen space and blurs with KWin.

### Updates

Pier checks for updates shortly after it starts and every 12 hours. When a new version is available, select **Update to x.y.z** in the tray menu, or select **Install and restart** in **Settings** > **General**.

Automatic updates work with the Windows installer, the macOS app, and the AppImage. If you installed the `.deb` package, download and install each new version from the releases page.

## Usage

- **Click** an app to open or focus it. Click the active app to minimize it. If the app has several windows, clicking cycles through them.
- **Shift+click** an app to open a new instance.
- **Right-click** an app to see its windows, pin or unpin it, or close it.
- **Right-click** the dock background to open settings or quit.
- **Drag** pinned apps to reorder them. To pin an app, drop it on the dock (an `.exe` or shortcut on Windows, an `.app` on macOS, or a `.desktop` file on Linux), or add it in **Settings** > **Apps**.
- **Click** a widget to open its panel. On the volume widget, scroll to adjust the volume and right-click to mute; its panel has the output device and, on Windows and Linux, each app's volume.

| Shortcut | Windows and Linux | macOS |
| --- | --- | --- |
| Open or focus the app in position N | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>1</kbd>…<kbd>9</kbd> | <kbd>Control</kbd>+<kbd>Option</kbd>+<kbd>1</kbd>…<kbd>9</kbd> |

### GitHub widgets

Pier uses your [GitHub CLI](https://cli.github.com) login (`gh auth login`) automatically. To use a different account, paste a personal access token with the `repo` and `read:user` scopes in **Settings** > **GitHub**. The token takes priority over the GitHub CLI and is stored in your system's keychain.

### Weather and calendar widgets

- **Weather:** Choose your city in **Settings** > **Widgets** > **Weather**. Forecasts come from [Open-Meteo](https://open-meteo.com) and don't need an API key.
- **Calendar:** Paste your calendar's secret iCal address in **Settings** > **Widgets** > **Calendar**. Google Calendar, Outlook, and iCloud all publish one. Pier stores the address in your system's keychain.

### Claude Code usage widget

A ring shows how much of your Claude Code limit is gone in the tighter of the two windows — the 5-hour session or the week — and the panel breaks both down with the time until each resets, plus today's token count.

Nothing is configured and nothing is sent anywhere. Pier reads two files the Claude Code CLI already keeps on disk:

- `~/.claude.json`, field `cachedUsageUtilization`, for the limit percentages. This is the same source the `/usage` command reads, so the numbers match your terminal. The CLI only refreshes it while it runs, so with Claude Code closed the number freezes — the widget dims and says how old the reading is instead of passing a stale value off as current.
- `~/.claude/projects/**/*.jsonl`, the session transcripts, for the token counts. Only the numeric `usage` fields, the model name, and the timestamp are read — never the content of your conversations.

## Platform notes

### macOS

Pier asks for two permissions:

- **Accessibility** (**System Settings** > **Privacy & Security** > **Accessibility**) to list, focus, minimize, and close windows. Without it, Pier still shows running apps and can focus, hide, and quit them. Pier's settings show a **Grant permission** button that requests it.
- **Automation**, the first time Spotify or Music plays something, so the media widget can read and control them through AppleScript.

Because the app isn't signed with an Apple Developer ID yet, macOS asks for the Accessibility permission again after each update.

macOS has some limitations compared to Windows:

- Apps can't reserve screen space, so maximized windows extend under the dock. To avoid this, set **Hide automatically** in **Settings** > **Appearance**.
- The native Dock can't be removed. Pier sets it to auto-hide with a very long reveal delay and restores your original preferences when it quits.
- The media widget supports only Spotify and Music.
- The Wi-Fi widget shows the network name only if Pier has Location permission.
- The Bluetooth widget asks for Bluetooth permission the first time, can't turn Bluetooth on or off (macOS doesn't allow it), and doesn't show device battery levels.

### Linux

Pier fully supports GNOME 45 to 49 on Wayland and X11 through its GNOME Shell extension. On KDE Plasma 6 the dock needs no extension: in an X11 session it lists windows, reserves screen space and draws glass with KWin's blur, and on Wayland it lists the windows of X11 apps (windows of native Wayland apps aren't listed yet). On other desktop environments, Pier can list windows only in X11 sessions, and it can't hide the native panel. Pinned apps and widgets work everywhere.

Running as an AppImage, Pier adds its own menu entry and icons to `~/.local/share` on the first launch, so the dock shows up in the application menu without any manual setup.

## Configuration

Pier stores its settings in `config.json`:

- **Windows:** `%APPDATA%\pier\config.json`
- **macOS:** `~/Library/Application Support/pier/config.json`
- **Linux:** `~/.config/pier/config.json`

If you used Pier when it was called WinDock, Pier imports your settings, GitHub token, and launch-at-login setting the first time it runs. On GNOME, reinstall the extension from Pier's settings.

## Troubleshooting

### The taskbar or Dock doesn't come back

Pier restores the native taskbar (or the macOS Dock) when it quits. If the process is killed, run the following command to restore it:

```sh
%LOCALAPPDATA%\Pier\pier.exe --restore-taskbar                # Windows
/Applications/Pier.app/Contents/MacOS/pier --restore-taskbar   # macOS
```

On GNOME, the Dash comes back automatically.

## Building from source

### Prerequisites

- [Node.js](https://nodejs.org) 22 or later
- [Rust](https://rustup.rs) (stable)
- Platform tools:
  - **Windows:** Visual Studio Build Tools with the C++ workload and the Windows SDK. Use the `stable-x86_64-pc-windows-msvc` Rust toolchain.
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).
  - **Linux:** `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, and `patchelf`.

### Run and build

```sh
npm install
npm run tauri dev     # run in development mode
npm run tauri build   # build the installers into src-tauri/target/release/bundle/
```

When you run `npm run tauri dev` on macOS, the system grants permissions to the terminal that ran the command, not to Pier.

### Preview in the browser

Outside Tauri, `npm run dev` answers IPC calls with sample data so you can review the UI in a browser. Open `http://localhost:1420` with one of these query strings:

- `?view=dock`
- `?view=panel&kind=media` (or `github-mine`, `github-review`, `github-contrib`, `volume`)
- `?view=settings`

### Tests

```sh
npm test                               # frontend tests
cd src-tauri
cargo test                             # Rust unit tests
cargo test -- --ignored --nocapture    # smoke tests against the real system
```

The smoke tests exercise real windows, apps, media sessions, and the GitHub API on the machine that runs them.

### Icons

All platform icons come from `app-icon.svg` and `src-tauri/icons/source/`. To regenerate them, run `./scripts/icons.sh` on macOS (it requires `iconutil`).

## Releasing

To publish a new version:

1. Run `./scripts/release.sh x.y.z`. The script updates the version in `package.json` and `Cargo.toml`, commits the change, and creates the `vx.y.z` tag.
2. Run `git push --follow-tags`.
3. Wait for the **Release** workflow to finish. It builds Pier on macOS, Windows, and Linux and creates a draft release with the installers and the updater's `latest.json`.
4. Review the draft and publish it. Installed copies of Pier see the new version only after you publish.

To test the build without creating a release, run the workflow manually from **Actions** > **Release** > **Run workflow**. The installers are attached to the workflow run as artifacts.

The workflow signs update packages with the updater key stored in the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets. The matching public key is in `src-tauri/tauri.conf.json`. Keep a backup of the private key: if it's lost, installed copies of Pier reject future updates.

To sign and notarize the macOS app later, add the `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` secrets, pass them in the `env` of the `tauri-action` step, and remove `"signingIdentity": "-"` from `src-tauri/tauri.macos.conf.json`.

## License

Copyright © 2026 Nathan Ferreira and Matheus Basilio.

Pier is free software: you can redistribute it and/or modify it under the terms of the [GNU General Public License](LICENSE) as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

Pier is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
