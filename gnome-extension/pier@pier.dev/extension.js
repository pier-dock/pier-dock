// Ponte entre o GNOME Shell e o Pier.
//
// No Wayland um app comum não enxerga as janelas dos outros, não consegue focá-las e não reserva
// espaço na tela. Esta extensão roda dentro do Shell e expõe isso no session bus como
// `dev.pier.Shell`. Só o processo dono de `dev.pier.App` (o Pier) pode chamar os
// métodos; quando ele some (inclusive num crash), tudo o que foi alterado é desfeito.

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {ExtensionState} from 'resource:///org/gnome/shell/misc/extensionUtils.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const VERSION = 2;
const APP_NAME = 'dev.pier.App';
const BUS_NAME = 'dev.pier.Shell';
const OBJECT_PATH = '/dev/pier/Shell';
// Docks de outras extensões, que ficariam por cima da do Pier.
const DOCK_EXTENSIONS = [
    'dash-to-dock@micxgx.gmail.com',
    'ubuntu-dock@ubuntu.com',
    'dash2dock-lite@icedman.github.com',
    'dash2dock-animated@icedman.github.com',
];

const IFACE = `
<node>
  <interface name="dev.pier.Shell">
    <method name="ListWindows"><arg type="s" direction="out" name="json"/></method>
    <method name="Activate"><arg type="t" direction="in" name="id"/></method>
    <method name="Minimize"><arg type="t" direction="in" name="id"/></method>
    <method name="Close"><arg type="t" direction="in" name="id"/></method>
    <method name="ShowApps"/>
    <method name="OpenQuickSettings"/>
    <method name="OpenCalendar"/>
    <method name="ToggleDesktop"/>
    <method name="SetHideNative"><arg type="b" direction="in" name="hide"/></method>
    <method name="SetReservedHeight"><arg type="u" direction="in" name="height"/></method>
    <method name="SetShortcutsEnabled"><arg type="b" direction="in" name="enabled"/></method>
    <signal name="WindowsChanged"/>
    <signal name="ShortcutActivated"><arg type="u" name="index"/></signal>
    <property name="Version" type="u" access="read"/>
  </interface>
</node>`;

class PierService {
    constructor() {
        this._appOwner = null;
        this._windowSignals = new Map();
        this._displaySignals = [];
        this._otherSignals = [];
        this._changedSource = 0;
        this._reserve = null;
        this._reservedHeight = 0;
        this._hiddenNative = null;
        this._accelerators = new Map();
        this._acceleratorSignal = 0;
        this._desktopWindows = new Set();
    }

    enable() {
        this._dbus = Gio.DBusExportedObject.wrapJSObject(IFACE, this);
        this._dbus.export(Gio.DBus.session, OBJECT_PATH);
        this._ownId = Gio.bus_own_name(Gio.BusType.SESSION, BUS_NAME, Gio.BusNameOwnerFlags.NONE, null, null, null);
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            APP_NAME,
            Gio.BusNameWatcherFlags.NONE,
            (_conn, _name, owner) => {
                this._appOwner = owner;
            },
            () => {
                this._appOwner = null;
                this._reset();
            }
        );

        const display = global.display;
        this._displaySignals = [
            display.connect('window-created', (_d, win) => {
                this._trackWindow(win);
                this._changed();
            }),
            display.connect('notify::focus-window', () => this._changed()),
            display.connect('in-fullscreen-changed', () => this._changed()),
        ];
        this._otherSignals = [
            [Main.overview, Main.overview.connect('showing', () => this._changed())],
            [Main.overview, Main.overview.connect('hidden', () => this._changed())],
            [Shell.WindowTracker.get_default(), Shell.WindowTracker.get_default().connect('tracked-windows-changed', () => this._changed())],
            [Main.layoutManager, Main.layoutManager.connect('monitors-changed', () => this._placeReserve())],
        ];
        for (const actor of global.get_window_actors())
            this._trackWindow(actor.meta_window);
    }

    disable() {
        this._reset();
        if (this._changedSource)
            GLib.source_remove(this._changedSource);
        this._changedSource = 0;
        for (const id of this._displaySignals)
            global.display.disconnect(id);
        this._displaySignals = [];
        for (const [obj, id] of this._otherSignals)
            obj.disconnect(id);
        this._otherSignals = [];
        for (const [win, ids] of this._windowSignals)
            ids.forEach(id => win.disconnect(id));
        this._windowSignals.clear();
        Gio.bus_unwatch_name(this._watchId);
        Gio.bus_unown_name(this._ownId);
        this._dbus.unexport();
        this._dbus = null;
    }

    // ---- D-Bus: só o Pier pode chamar ----

    _guard(invocation) {
        if (this._appOwner && invocation.get_sender() === this._appOwner)
            return true;
        invocation.return_dbus_error('dev.pier.Shell.Error.AccessDenied', 'só o Pier pode usar esta interface');
        return false;
    }

    _handle(invocation, fn) {
        if (!this._guard(invocation))
            return;
        try {
            const result = fn();
            invocation.return_value(result ?? null);
        } catch (e) {
            invocation.return_dbus_error('dev.pier.Shell.Error.Failed', String(e));
        }
    }

    get Version() {
        return VERSION;
    }

    ListWindowsAsync(_params, invocation) {
        this._handle(invocation, () => new GLib.Variant('(s)', [JSON.stringify(this._snapshot())]));
    }

    ActivateAsync([id], invocation) {
        this._handle(invocation, () => {
            const win = this._find(id);
            if (win)
                Main.activateWindow(win, global.get_current_time());
        });
    }

    MinimizeAsync([id], invocation) {
        this._handle(invocation, () => this._find(id)?.minimize());
    }

    CloseAsync([id], invocation) {
        this._handle(invocation, () => this._find(id)?.delete(global.get_current_time()));
    }

    ShowAppsAsync(_params, invocation) {
        this._handle(invocation, () => {
            if (Main.overview.visible)
                Main.overview.hide();
            else
                Main.overview.showApps();
        });
    }

    OpenQuickSettingsAsync(_params, invocation) {
        this._handle(invocation, () => Main.panel.statusArea.quickSettings?.menu.toggle());
    }

    OpenCalendarAsync(_params, invocation) {
        this._handle(invocation, () => Main.panel.statusArea.dateMenu?.menu.toggle());
    }

    ToggleDesktopAsync(_params, invocation) {
        this._handle(invocation, () => this._toggleDesktop());
    }

    SetHideNativeAsync([hide], invocation) {
        this._handle(invocation, () => this._setHideNative(hide));
    }

    SetReservedHeightAsync([height], invocation) {
        this._handle(invocation, () => this._setReservedHeight(height));
    }

    SetShortcutsEnabledAsync([enabled], invocation) {
        this._handle(invocation, () => this._setShortcuts(enabled));
    }

    // ---- Janelas ----

    _trackWindow(win) {
        if (!win || this._windowSignals.has(win))
            return;
        const ids = [
            win.connect('notify::title', () => this._changed()),
            win.connect('notify::minimized', () => this._changed()),
            win.connect('notify::skip-taskbar', () => this._changed()),
            win.connect('unmanaged', () => {
                ids.forEach(id => win.disconnect(id));
                this._windowSignals.delete(win);
                this._desktopWindows.delete(win);
                this._changed();
            }),
        ];
        this._windowSignals.set(win, ids);
    }

    // Agrupa rajadas de eventos num único sinal.
    _changed() {
        if (this._changedSource)
            return;
        this._changedSource = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 50, () => {
            this._changedSource = 0;
            this._dbus?.emit_signal('WindowsChanged', null);
            return GLib.SOURCE_REMOVE;
        });
    }

    _find(id) {
        return global.display
            .get_tab_list(Meta.TabList.NORMAL_ALL, null)
            .find(w => String(w.get_id()) === String(id)) ?? null;
    }

    _snapshot() {
        const tracker = Shell.WindowTracker.get_default();
        // Ordem de uso mais recente primeiro, em todas as áreas de trabalho.
        const windows = global.display
            .get_tab_list(Meta.TabList.NORMAL_ALL, null)
            .filter(w => !w.is_skip_taskbar())
            .map(w => {
                const appId = tracker.get_window_app(w)?.get_id() ?? null;
                return {
                    id: w.get_id(),
                    title: w.get_title() ?? '',
                    appId: appId && !appId.startsWith('window:') ? appId : null,
                    wmClass: w.get_wm_class() ?? null,
                    pid: w.get_pid(),
                    minimized: w.minimized,
                };
            });
        const focus = global.display.focus_window;
        return {
            windows,
            focus: focus ? {id: focus.get_id(), pid: focus.get_pid(), fullscreen: focus.is_fullscreen()} : null,
            overview: Main.overview.visible,
        };
    }

    // ---- Mostrar a área de trabalho ----

    // O Mutter tem esse modo internamente (`showing_desktop`), mas não o expõe para extensões:
    // as janelas minimizadas daqui ficam lembradas para o próximo toque devolvê-las.
    _toggleDesktop() {
        if (this._desktopWindows.size > 0) {
            this._restoreDesktop();
            return;
        }
        const workspace = global.workspace_manager.get_active_workspace();
        for (const actor of global.get_window_actors()) {
            const win = actor.meta_window;
            if (win.is_skip_taskbar() || win.minimized)
                continue;
            if (win.get_workspace() !== workspace && !win.is_on_all_workspaces())
                continue;
            this._desktopWindows.add(win);
            win.minimize();
        }
    }

    _restoreDesktop() {
        const windows = [...this._desktopWindows];
        this._desktopWindows.clear();
        for (const win of windows)
            win.unminimize();
    }

    // ---- Esconder o Dash e docks de outras extensões ----

    // As docks de outras extensões são suspensas chamando o disable()/enable() delas direto, sem
    // passar pelo gerenciador de extensões: nada vai para o gsettings, então um logout ou um
    // reinício do Shell no meio do caminho não as deixa desligadas.
    _setHideNative(hide) {
        if (hide && !this._hiddenNative) {
            const suspended = [];
            for (const uuid of DOCK_EXTENSIONS) {
                const ext = Main.extensionManager.lookup(uuid);
                if (ext?.state !== ExtensionState.ACTIVE || !ext.stateObj)
                    continue;
                try {
                    ext.stateObj.disable();
                    suspended.push(ext);
                } catch (e) {
                    console.error(`Pier: falha ao suspender ${uuid}: ${e}`);
                }
            }
            // Depois das extensões: o disable() delas costuma reexibir o Dash.
            const dash = Main.overview.dash;
            dash.hide();
            dash.set_height(0);
            this._hiddenNative = {dash, suspended};
        } else if (!hide && this._hiddenNative) {
            const {dash, suspended} = this._hiddenNative;
            this._hiddenNative = null;
            dash.set_height(-1);
            dash.show();
            for (const ext of suspended) {
                // O usuário pode ter desligado a extensão enquanto ela estava suspensa.
                if (ext.state !== ExtensionState.ACTIVE)
                    continue;
                try {
                    ext.stateObj.enable();
                } catch (e) {
                    console.error(`Pier: falha ao reativar ${ext.uuid}: ${e}`);
                }
            }
        }
    }

    // ---- Área reservada na base da tela ----

    _setReservedHeight(height) {
        this._reservedHeight = height;
        if (height > 0 && !this._reserve) {
            // Ator invisível que só existe para o Mutter tirar esta faixa da área de trabalho.
            this._reserve = new St.Widget({name: 'pierReserve', reactive: false});
            Main.layoutManager.addChrome(this._reserve, {affectsStruts: true, affectsInputRegion: false, trackFullscreen: true});
        } else if (height === 0 && this._reserve) {
            Main.layoutManager.removeChrome(this._reserve);
            this._reserve.destroy();
            this._reserve = null;
        }
        this._placeReserve();
    }

    _placeReserve() {
        const monitor = Main.layoutManager.primaryMonitor;
        if (!this._reserve || !monitor)
            return;
        const scale = St.ThemeContext.get_for_stage(global.stage).scale_factor;
        const height = Math.round(this._reservedHeight * scale);
        this._reserve.set_position(monitor.x, monitor.y + monitor.height - height);
        this._reserve.set_size(monitor.width, height);
    }

    // ---- Atalhos Ctrl+Alt+1..9 ----

    _setShortcuts(enabled) {
        if (enabled && this._accelerators.size === 0) {
            for (let i = 0; i < 9; i++) {
                const action = global.display.grab_accelerator(`<Control><Alt>${i + 1}`, Meta.KeyBindingFlags.NONE);
                if (action === Meta.KeyBindingAction.NONE)
                    continue;
                Main.wm.allowKeybinding(Meta.external_binding_name_for_action(action), Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW);
                this._accelerators.set(action, i);
            }
            this._acceleratorSignal = global.display.connect('accelerator-activated', (_d, action) => {
                const index = this._accelerators.get(action);
                if (index !== undefined)
                    this._dbus?.emit_signal('ShortcutActivated', new GLib.Variant('(u)', [index]));
            });
        } else if (!enabled && this._accelerators.size > 0) {
            for (const action of this._accelerators.keys()) {
                global.display.ungrab_accelerator(action);
                Main.wm.allowKeybinding(Meta.external_binding_name_for_action(action), Shell.ActionMode.NONE);
            }
            this._accelerators.clear();
            global.display.disconnect(this._acceleratorSignal);
            this._acceleratorSignal = 0;
        }
    }

    _reset() {
        // Sem o Pier por perto ninguém chamaria o toque de volta: as janelas voltam agora.
        this._restoreDesktop();
        this._setHideNative(false);
        this._setReservedHeight(0);
        this._setShortcuts(false);
    }
}

export default class PierExtension extends Extension {
    enable() {
        this._service = new PierService();
        this._service.enable();
    }

    disable() {
        this._service?.disable();
        this._service = null;
    }
}
