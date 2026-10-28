# Changelog

What changed in each version of Pier. The app shows these notes once after it updates, so write them for users. Add a `## <version>` section before tagging a release, in this file and in `CHANGELOG.pt-BR.md`.

## 0.21.0

### Keyboard layout widget
- **Windows:** a new **Keyboard layout** widget shows the language code of the active layout (ENG, POR…), like the one in the Windows taskbar. Click it to list the installed layouts and switch between them. It is off by default, and it is hidden on macOS and Linux.

### Fixes
- **Windows:** if Pier is killed, for example from Task Manager, the taskbar now comes back on its own. Before, it stayed hidden until you ran `pier.exe --restore-taskbar`.
- **Dock:** when the cursor lands on an icon at once, as when the dock slides back up under it, the neighbors no longer grow past their final size and shrink back.
- **Dock:** the bar stays centered, inside the glass behind it, while magnification widens it during the reveal.

## 0.20.0

### Redesigned Appearance and feedback
- **Settings:** the **Appearance** tab has a new layout. The live preview sits on top, and the controls are in three cards of the same height: **Style** (material, opacity, reduced effects, icon size and accent color), **Dock** (position, width and panel height) and **Behavior** (auto-hide, magnification and running apps). Everything fits the default settings window without scrolling. The explanatory texts are gone, and the warnings about a system without glass and about **Never** hiding the dock stay. The preview now shows the dock position and the magnification, and the auto-hide mode **When an app is in use** is now called **Smart**.
- **Reporting a problem:** the dialog has a new look, with a notice that the report becomes a public GitHub issue and a **Cancel** button. After sending, a confirmation shows the created issue with its repository and number and a button to open it. When GitHub opens in the browser instead, the confirmation says so, and **Send another** starts a new report.

## 0.19.0

### Pokémon on your dock
- **AI agents:** the mascot that walks the dock while a session runs can now be a Pokémon instead of Clawd. In the **AI agents** widget options, pick **Mascot** and then the Pokémon. It gets a status dot that follows the lead session, it jumps when a session asks for permission, a click still opens the sessions, and the widget's dock icon follows your choice.
- **Pokémon widget:** pick up to 10 Pokémon instead of one. They all walk along the dock edge, each at its own pace, and the first one is the icon in the dock. Settings lists the ones you chose, with a button to remove each and a counter, and stops adding at 10. Your current Pokémon carries over.

## 0.18.0

### Updates that wait for you
- **Updates:** Pier no longer installs an update and restarts on its own. When a new version is out, a dot appears on the app launcher (or on the start button when the launcher is hidden), and **Update to x.y.z available** is added to the dock menus. Select either to open **Settings** > **General**, and then select **Install and restart**. The tray item still installs right away. **Check for updates automatically** now turns the background checks on or off.
- **macOS:** Pier is now signed with a fixed certificate, so macOS keeps the Accessibility, Screen Recording and system audio permissions between updates. After this update it asks for them one last time.

### First launch
- On a fresh install Pier no longer hides the Windows taskbar, the GNOME Dash or the macOS Dock right away. It opens **Settings** with a welcome card that asks first.
- While the GNOME extension or macOS Accessibility still needs setting up, an amber dot on the app launcher opens **Settings**, where the notice says what to do. Before, you could only see it by opening **Settings**.

### Fixes
- **Settings:** an unreadable config file is no longer replaced with the defaults. It is kept as `config.json.corrupt-<time>` and the previous save is restored. Saves are atomic, so an update or a crash in the middle of one can't leave a half-written file. If the settings can't load, the window shows an error and retries instead of staying blank, and a setting that fails to save goes back to its previous value.
- **Language:** errors from the system, like "no Wi-Fi interface", follow the interface language instead of always being in Portuguese. The iCal address placeholder, the clock widget description and the GNOME extension description are fixed too.
- **Windows:** an app entry with a malformed folder ID could break the app list. It is skipped now.

## 0.17.0

### Pokémon on your dock
A new **Pokémon** widget, in the new **Fun** group, shows the party menu icon of the Pokémon you pick, from PokeAPI. Search by name or number (#1 to #809, the ones that have a party icon). Like the AI sessions widget, it can sit in the dock as an icon, walk along the dock edge, or both. It faces the way it walks, with its feet on the edge, and it works alongside Clawd.

### Widgets
- **Settings:** the widget gallery lists the widgets in use first. Turning one on moves it to the end of the active ones, where it shows up in the dock.
- **Settings:** with **Clawd only** or **Walking only**, the widget card shows as on instead of off.

### Fixes
- **Sound:** on Linux (PulseAudio/PipeWire), outputs that aren't plugged in, like HDMI ports with nothing connected, are no longer listed, so you can't pick one and lose the sound.

## 0.16.0

### Fixes
- **Updates:** on Linux, **Install and restart** reopens Pier after installing the new version, instead of just closing it. On NixOS, an AppImage update restarts through `appimage-run`.
- **Sound:** each app's output selector opens inside the sound panel, so picking a device no longer closes the selector or the whole panel. It also works with the keyboard (arrows, Enter and Esc).
- **Dock:** clicking an app that isn't pinned no longer shuffles the open apps. They stay where they appeared, and newly opened apps go at the end.

## 0.15.0

### Notification badges on Windows
On Windows, the red badge on a dock icon now also counts the app's notifications in the Notification Center, and the badge that Store apps like WhatsApp put on the taskbar. It works for apps that don't show a count in the window title, such as Discord and Steam. When both are available, the dock shows the larger number.

### Fixes
- **Windows:** Store apps that don't set their ID on the window, like WhatsApp, show their real icon and are opened the same way as from the Start menu.

## 0.14.0

### Power modes and per-app audio
- **Battery:** the battery widget opens a panel with the active power profile and the Power save, Balanced and Performance modes your system supports. A mode that fails to apply is reported instead of showing as selected.
- **Sound:** on Linux (PulseAudio/PipeWire), each playing app shows its icon and its own output selector, so you can send one app to another device without moving the rest.

### Reports without a GitHub account
You can now send a bug report from Pier without connecting a GitHub account. It is opened as an issue for you.

### Settings in the dock
While **Settings** or **What's new** is open, Pier shows up as a running app: in the dock, and on macOS also in the system Dock and Cmd+Tab. Click it to bring the window back, even if other windows are covering it.

### Smoother dock
Hovering and magnifying icons is faster and lighter, and background updates from widgets no longer redraw the whole dock. **Settings** > **Appearance** has a new **Reduce visual effects** option that uses a solid background and drops the decorative shadows, for machines that struggle with the blur.

### More widget improvements
- **Clock:** clicking the date opens a month calendar with previous/next month and **Today**, instead of the notifications.
- **Bluetooth:** saved devices stay in the list even when Bluetooth is off, and you can turn it on, connect, and disconnect from the panel.
- **Network, Sound and Bluetooth:** the panel footers no longer repeat confusing connection details.
- **Dock icons** now match the artwork on the website, including the battery, media controls, volume, Wi-Fi, and calendar icons.

### Fixes
- **Windows:** with the dock always shown, fullscreen apps and videos hide it.
- **AI Agents:** Antigravity and other providers running inside T3 Code are detected.
- **Clawd:** the mascot stands on the dock on every screen edge, not only the bottom.
- **Dock:** the hover highlight of a magnified icon stays inside the bar.
- **System settings:** Bluetooth opens the right settings app on each Linux desktop, and a settings action that fails now shows an error you can retry.

## 0.13.0

### Unread counts on the dock
App icons now show a red badge with the unread count that apps and sites put at the start of the window title, like "(3) WhatsApp" or "[12] Inbox".
- The count adds up across the app's windows and stops at **99+**.
- The tooltip mentions the notifications, and screen readers read the count along with the app state.

### A clearer calendar
The calendar popover now has one focal point: your next event.
- The header shows the weekday, the date, and how many events are left.
- The next event is a neutral card with the event color as an accent bar, the status, title, time, and place on one line, and a progress bar while it is happening. **Join** takes the full width, and **Copy link** is an icon button.
- The rest of the day is a timeline with a colored rail, split into **Later today** and **Tomorrow** (dimmed). A free day shows a centered empty state.

### Fixes
- **AI Agents:** with opencode V2, the plugin failed to load and opencode sessions never showed up. Reinstall the plugin in the widget's options and restart opencode.
- **AI Agents:** Codex sessions started by an app server, such as T3 Code, are detected automatically from local lifecycle events, without hooks or a restart. Optional hooks still add the permission status.
- **Windows:** apps from the Microsoft Store, like WhatsApp, show their real icon instead of the generic Windows one. An icon that fails to load is now retried instead of cached forever.
- **Linux:** clicking an app in the AppImage works on NixOS again; launched apps no longer inherit the AppImage's bundled libraries.

## 0.12.0

### One widget for all your AI CLIs
The **Claude Agents** widget became **AI Agents** and now shows sessions from Claude Code, Codex, opencode, and Gemini CLI: running, asking for permission, with an error, or done.
- Each CLI has its own integration in **Settings** > **Widgets** > the widget's options. **Enable** installs the hooks (or the opencode plugin) and **Disable** removes them.
- Codex asks you to trust Pier's hooks with `/hooks`; opencode needs a restart to load the plugin.
- The list shows which CLI each session belongs to, and clicking a row still jumps to the terminal or IDE that started it. Your existing Claude Code setup keeps working, and **Reinstall** moves it to the shared hook.
- The widget only shares the provider, status, project folder, and time — never the conversation content.

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
