# Changelog

What changed in each version of Pier. The app shows these notes once after it updates, so write them for users. Add a `## <version>` section before tagging a release, in this file and in `CHANGELOG.pt-BR.md`.

## 0.7.0

### KDE Plasma support
Pier now runs on KDE Plasma without any manual setup.
- **Plasma on X11:** the dock lists, focuses, minimizes and closes windows, reserves screen space, and the glass material blurs with KWin. No extension to install.
- **Plasma on Wayland:** the dock lists the windows of X11 apps. Settings explains that native Wayland windows aren't listed yet.
- **AppImage:** on the first launch, Pier adds itself to the app menu with its icon, so you no longer have to write a menu entry by hand. An existing entry is never touched.

### Window previews on hover
Hover an app with more than one window to see one card per window above the dock, with the window title and a close button. Click a card to focus that window.
- On Linux with X11, each card shows a thumbnail of the window. On Windows, macOS and GNOME, the card shows the app icon for now.
- Apps with a single window keep the click to show or hide it.

### Volume popover
Clicking the volume widget opens a popover with:
- the master volume and a mute button;
- the output devices, to switch to another one with one click;
- the volume of each app that is playing audio (Windows and Linux).

Scrolling and right-clicking the widget still adjust and mute the volume.

### Wi-Fi popover
Clicking the network widget opens a popover with the current network and the networks nearby.
- Connect to a saved or open network with one click, or type the password of a new one right in its row.
- Disconnect, and turn Wi-Fi on or off when the system allows it.
- On macOS 14+ and Windows 11 24H2, the system only reveals network names to apps with Location access. The popover asks for it when needed.

### App launcher
A new **App launcher** item in the dock opens a panel with every installed app.
- Type to search (case and accents are ignored), use the arrow keys to move and **Enter** to open.
- Right-click an app to pin it to the dock.
- It's off by default. Turn it on in **Settings** > **Apps** > **Show the app launcher**.

### AI usage widget with Codex CLI
The Claude Code usage widget is now **AI usage** and also reads **Codex CLI**. Choose the AI in **Settings** > **Widgets** > **AI usage**, or leave it on **Automatic**, which shows the AI you used most recently.
- The panel shows one bar per limit window the AI reports, and the tooltip names the AI the numbers come from.
- Everything is read from local files. Nothing goes over the network, and only the usage numbers are read, never your conversations.
- Your existing widget and its settings carry over automatically.

### Show desktop widget
A new **Desktop** widget minimizes every window with one click and brings them back on the next one, like the corner button of the Windows taskbar. It works on Windows, macOS, Linux with X11, GNOME and KDE Plasma. Turn it on in **Settings** > **Widgets**.

> **GNOME:** this widget needs the new version of the Pier extension. Open **Settings**, reinstall the extension, and then log out and log back in.

### Panel height
**Settings** > **Appearance** > **Panel height** sets the height of the bar: **Automatic** (as before) or a custom height in pixels. The screen space the dock reserves follows the new height, and the app icons stay centered in the bar.

### Number format for the contributions widget
The GitHub contributions widget has a new **Number** format that shows only this year's total, without the graph.

### Report a problem from Settings
**Settings** > **General** now has **Report a problem** and **Suggest a feature**.
- Bug reports include your system details and, if you choose, Pier's recent logs, which you can review and edit before sending.
- With a GitHub account connected to Pier, the issue is created directly. Otherwise, the form opens in your browser already filled in.
- Pier now keeps a log file, so problems are easier to track down.
