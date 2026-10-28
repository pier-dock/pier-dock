# Pier user guide

**English** · [Português](guide.pt-BR.md) · [Back to README](../README.md)

## Features

- **Apps:** Pinned and running apps with magnification, drag-to-reorder, window lists, and count badges.
- **Customizable bar:** Put the dock on any edge of the screen, fit it to the icons or stretch it along the edge, and pick the icon size, material, and accent color.
- **Widgets:** Media player, GitHub (reviews, pull requests, commits, contributions), system (network, volume, Bluetooth, battery, clock, show desktop, system usage and, on Windows, keyboard layout and the tray), weather, calendar, Pomodoro, and AI usage.
- **AI agents:** Sessions from Claude Code, Codex, opencode, and Gemini CLI at a glance, and a click jumps to the terminal or IDE that started one. Clawd or a Pokémon can walk along the dock while a session runs.
- **And more:** Keyboard shortcuts for the apps in position 1 to 9, one-click updates, and an English and Portuguese interface.

## Installation

Download the installer for your platform from the [latest release](https://github.com/pier-dock/pier-dock/releases/latest).

| Platform | File | Notes |
| --- | --- | --- |
| Windows 10 and 11 | `Pier_x.y.z_x64-setup.exe` | Installs to `%LOCALAPPDATA%\Pier` without admin rights. |
| macOS 12 or later | `Pier_x.y.z_universal.dmg` | Runs natively on Apple silicon and Intel Macs. |
| Linux (GNOME 45 to 49, KDE Plasma 6) | `Pier_x.y.z_amd64.deb` or `Pier_x.y.z_amd64.AppImage` | Install the `.deb` with `sudo apt install ./Pier_x.y.z_amd64.deb`, or make the AppImage executable with `chmod +x` and run it. On KDE Plasma nothing else is needed; on GNOME, install the extension from Pier's settings. |

### First launch

On its first launch Pier opens **Settings** and asks whether it should replace your taskbar, Dash, or Dock. The system bar stays visible until you choose. If something still needs setting up (the GNOME extension or macOS Accessibility), a notice appears in **Settings** and an amber dot appears on the app launcher.

The installers aren't code-signed yet, so your system warns you the first time you open Pier.

- **Windows:** When SmartScreen appears, select **More info**, and then select **Run anyway**.
- **macOS:** Drag Pier to **Applications** and open it. When macOS says it can't verify the app, go to **System Settings** > **Privacy & Security** and select **Open Anyway**. Alternatively, run `xattr -dr com.apple.quarantine /Applications/Pier.app` in Terminal.
- **Linux:** On GNOME, open Pier's settings and select **Install extension**, and then log out and log back in. The extension lets Pier see and focus windows, reserve screen space, hide the Dash, show the desktop, and register shortcuts. On KDE Plasma there is nothing to install: in an X11 session the dock lists windows, reserves screen space and blurs with KWin.

### Updates

Pier checks for updates shortly after it starts and every 12 hours, and never restarts on its own. When a new version is available, a dot appears on the app launcher. Select it (or **Update to x.y.z** in the tray menu), and then select **Install and restart** in **Settings** > **General**. To stop checking, turn off **Check for updates automatically** there.

In-app updates work with the Windows installer, the macOS app, and the AppImage. If you installed the `.deb` package, download and install each new version from the releases page.

## Usage

- **Click** an app to open or focus it (clicking the active app minimizes it; with several windows, clicking cycles through them). **Shift+click** opens a new instance.
- **Right-click** an app to see its windows, pin or unpin it, or close it. Right-click the dock background to open settings or quit.
- **Click** a divider to open its removal menu; it can also be reached with the keyboard.
- **Drag** pinned apps to reorder them. To pin an app, drop it on the dock or add it in **Settings** > **Apps**.
- **Click** a widget to open its panel. On the volume widget, scroll to adjust the volume and right-click to mute.
- **Press** <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>1</kbd>…<kbd>9</kbd> (<kbd>Control</kbd>+<kbd>Option</kbd>+<kbd>1</kbd>…<kbd>9</kbd> on macOS) to open or focus the app in that position.

## Platform notes

### macOS

Pier asks for these permissions:

- **Accessibility** (**System Settings** > **Privacy & Security** > **Accessibility**) to list, focus, minimize, and close windows. Without it, Pier still shows running apps and can focus, hide, and quit them. Pier's settings show a **Grant permission** button that requests it.
- **Automation**, the first time Spotify or Music plays something, so the media widget can read and control them through AppleScript.
- **Screen Recording** (macOS 14 or later), the first time the window previews open, so each card shows a thumbnail of the window. Without it, the cards show the app icon.
- **System Audio Recording** (macOS 14.2 or later), the first time you move an app's slider in the volume panel. Pier passes that app's audio through itself to change its volume. Without it, the panel lists the apps but can't change their volume, and shows a button that opens the setting.

Because the app isn't signed with an Apple Developer ID yet, macOS asks for the Accessibility, Screen Recording and System Audio Recording permissions again after each update.

macOS has some limitations compared to Windows:

- Apps can't reserve screen space, so maximized windows extend under the dock. To avoid this, set **Hide automatically** in **Settings** > **Appearance**.
- The native Dock can't be removed. Pier sets it to auto-hide with a very long reveal delay and restores your original preferences when it quits.
- The media widget supports only Spotify and Music.
- The Wi-Fi widget shows the network name and the networks nearby only if Pier has Location permission. Its panel asks for it.
- The Bluetooth widget asks for Bluetooth permission the first time, can't turn Bluetooth on or off (macOS doesn't allow it), and doesn't show device battery levels.

### Linux

Pier fully supports GNOME 45 to 49 on Wayland and X11 through its GNOME Shell extension. On KDE Plasma 6 it needs no extension: in an X11 session it lists windows, reserves screen space and blurs with KWin, and on Wayland it lists only the windows of X11 apps. On other desktop environments, Pier can list windows only in X11 sessions, and it can't hide the native panel. Pinned apps and widgets work everywhere.

To keep the dock on another monitor, or have it follow the cursor, on GNOME you need version 4 of the extension. If **Settings** > **General** > **Show the dock on** offers to update it, select **Update the extension**, and then log out and log back in.

Running as an AppImage, Pier adds its own menu entry and icons to `~/.local/share` on the first launch, so the dock shows up in the application menu without any manual setup.

## Configuration

Pier stores its settings in `config.json`:

- **Windows:** `%APPDATA%\pier\config.json`
- **macOS:** `~/Library/Application Support/pier/config.json`
- **Linux:** `~/.config/pier/config.json`

If you used Pier when it was called WinDock, Pier imports your settings, GitHub token, and launch-at-login setting the first time it runs. On GNOME, reinstall the extension from Pier's settings.

## Troubleshooting

### Hover feels slow

If hover animation feels sluggish, try **Settings > Appearance > Reduce visual effects**. It disables blur and decorative dock/icon shadows while keeping your widgets and animations. Turning it off restores your chosen material.

### The taskbar or Dock doesn't come back

Pier restores the native taskbar (or the macOS Dock) when it quits. On Windows a small helper process also restores the taskbar when Pier is killed, for example from Task Manager. If the taskbar is still missing (or on macOS), run the following command to restore it:

```sh
%LOCALAPPDATA%\Pier\pier.exe --restore-taskbar                # Windows
/Applications/Pier.app/Contents/MacOS/pier --restore-taskbar   # macOS
```

On GNOME, the Dash comes back automatically.

## Contributing

Found a bug or have an idea? [Open an issue](https://github.com/pier-dock/pier-dock/issues/new/choose). To build Pier from source, run the tests, or send a pull request, see [CONTRIBUTING.md](../CONTRIBUTING.md).

## License

Copyright © 2026 Nathan Ferreira and Matheus Basilio.

Pier is free software: you can redistribute it and/or modify it under the terms of the [GNU General Public License](../LICENSE) as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

Pier is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
