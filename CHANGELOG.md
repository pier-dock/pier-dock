# Changelog

What changed in each version of Pier. The app shows these notes once after it updates, so write them for users. Add a `## <version>` section before tagging a release, in this file and in `CHANGELOG.pt-BR.md`.

## 0.11.0

### Claude Code on the dock
The new **Claude Agents** widget shows your Claude Code sessions: running, asking for permission, with an error, or done.
- Click it to jump to the terminal or IDE window that started the session. With more than one session, it opens a list to choose from.
- It needs the Claude Code hooks: in **Settings** > **Widgets**, open the widget's options and select **Enable** next to **Claude Code hooks**. Pier adds them to `~/.claude/settings.json`, and **Disable** removes them.

### Meet Clawd
Clawd, Claude's mascot, walks along the dock while a session runs. He jumps when a session needs permission and rests when everything is idle.
- **Show in dock**, in the widget's options, picks **Icon**, **Icon and Clawd**, or **Clawd only**.
- Click Clawd to open your sessions, as with the widget. He stops walking while the cursor is over him or the sessions list is open.
- He hides along with the dock when **Hide automatically** is on.

### Fixes
- After the dock hid and came back, the tooltip of the last app or widget you used could stay on screen, sometimes next to another one.
- **Windows:** when the system can't apply the Mica material, Settings and the What's new window now use the solid material instead of showing up see-through.

## 0.10.0

### The dock on another monitor
With more than one monitor, **Settings** > **General** > **Show the dock on** picks where the dock lives:
- **Main display**, as before.
- A specific monitor, by name. If it's disconnected, the dock moves to the main display and comes back when it's plugged in again.
- **Follow the cursor**: the dock moves to the monitor the pointer is on.

Popovers open on the dock's monitor, and on Windows maximized windows on that monitor make room for it. On macOS, with monitors of different scales (a Retina display and an external monitor), the dock sits in the right place and follows the pointer on either one.

> **GNOME:** to reserve space off the main display, and to follow the cursor on Wayland, the extension needs an update. In **Show the dock on**, select **Update the extension**, and then log out and log back in.

### The dock makes an entrance
When Pier starts and when the dock changes monitor, it slides onto the screen instead of popping in. When it changes monitor, it slides off the old screen first, and stays visible for a moment even with **Hide automatically** on. With **Reduce motion** on in the system, it just fades.

### Automatic updates
When there's a new version, Pier updates itself on launch, restarts, and shows what changed. To go back to installing updates by hand, turn off **Settings** > **General** > **Update automatically**. On Linux this applies to the AppImage; with the `.deb`, updates still come from the tray menu.

### Each app's volume on macOS
The sound popover now lists the apps playing audio on macOS 14.2 or later, each with its own volume slider and mute button, as on Windows and Linux.
- The first time you move a slider, macOS asks for the **System Audio Recording** permission: Pier routes that app's audio through itself to change its volume. Without the permission, the popover says so and shows a button that opens System Settings.
- Pier remembers each app's volume and applies it again when the app plays, even after Pier restarts.

### Settings
- **Import** and **Export**, in **General**, move your pinned apps, widgets and appearance to another computer in one file. The GitHub account and the calendar aren't included.
- **Reset everything** brings every setting back to the default, after asking. The GitHub account and the calendar stay connected.
- **Permissions**, under **System**, sums up what Pier has from the system. **Review** lists each permission, what it's for, and a button to grant it or open System Settings. On macOS: Accessibility, Screen Recording, Automation, Location and Bluetooth. On Windows: Location.

### Fixes
- The What's new window, which opens after an update, now shows up in front of other windows instead of behind them.

## 0.9.0

### A new app launcher
The launcher has a bigger search field and now splits your apps in two.
- **Pinned** shows the apps you pin to the launcher as a gallery, in the order you pinned them. These pins are separate from the dock's.
- **Installed apps** lists everything else by name. The search filters both.
- **Ctrl+P** (**⌘P** on macOS) pins or unpins the selected app. **Ctrl+K** (**⌘K**) or a right click shows more actions, like **Pin to dock**.
- The arrow keys move through the gallery and the list, and **Enter** opens the selected app.

### Fixes
- **Windows 11:** with the liquid glass material, the dock no longer turns white when you move the pointer over it, and its rounded corners no longer show a square edge. The same goes for the popovers and the window previews.

## 0.8.1

### Fixes
- **Windows:** after moving the pointer over the dock icons for a while, the dock could stop reacting and let the mouse reach the window behind it. Pier now notices when this happens and makes the dock respond again right away.

## 0.8.0

### Choose where the dock sits
**Settings** > **Appearance** > **Position** puts the dock on the **Bottom** (as before), **Top**, **Left** or **Right** edge of the screen.
- On the sides the bar stands as a column. Wide widgets switch to their compact size, and the tooltips and window previews open to the side.
- On Windows, on Linux with X11 and on GNOME, maximized windows make room for the dock on any edge. On macOS the dock moves, but the system still doesn't let apps reserve screen space.

> **GNOME:** the top and side edges need the new version of the Pier extension. Open **Settings**, reinstall the extension, and then log out and log back in. With the old one, only the bottom edge reserves space.

### Better window previews
Hover an app with more than one window and the previews strip now matches the rest of the dock.
- It has the same glass as the popovers.
- On macOS the cards highlight on hover and show the close button.
- Each card shows a picture of the window on macOS (14 or later) and on Windows, not just the app icon. On macOS this needs the **Screen Recording** permission, which Pier asks for the first time the previews open. Minimized windows still show the app icon.
- The strip closes as soon as the pointer leaves it and the app's icon, for example when you move to another icon.
- The dock no longer flickers when the strip opens or closes.

### Paired Bluetooth devices
The Bluetooth popover now lists every paired device, not just the connected ones, under **Connected** and **Paired**.
- Click a paired device to connect it, and use **Disconnect** on a connected one.
- On Windows, clicking a device opens the Bluetooth settings, because Windows doesn't let apps connect to headphones and keyboards.

### What's new after updating
After Pier updates, a window shows what changed in the new version, once. Open it again any time in **Settings** > **General** > **What's new**.

### Fixes
- **macOS:** the **Allow** button in the Wi-Fi popover now shows the Location prompt, so the networks nearby show up. Before, clicking it did nothing.
- With **Hide automatically** on, the dock hides as soon as the pointer moves past the icons, instead of a long way above them, and waits a little less before hiding.

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
