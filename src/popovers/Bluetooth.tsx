import { useState } from "react";
import { BatteryFull, BatteryLow, BatteryMedium, Bluetooth as BluetoothIcon, BluetoothOff, Gamepad2, Headphones, Keyboard, Mouse, Smartphone } from "lucide-react";
import { ipc, type BluetoothDevice } from "../lib/ipc";
import { useBluetooth } from "../hooks/useSystem";
import { ActionFooter } from "../ui/ActionFooter";
import { Switch } from "../ui/Switch";
import { useT } from "../i18n";

const KIND_ICONS = { audio: Headphones, mouse: Mouse, keyboard: Keyboard, gamepad: Gamepad2, phone: Smartphone, other: BluetoothIcon };

export function BluetoothPanel() {
  const [bt, setPowered, refresh] = useBluetooth(true, 5_000);
  const t = useT().bluetooth;
  const [busy, setBusy] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const devices = bt?.devices ?? [];
  const connected = devices.filter((d) => d.connected);
  const paired = devices.filter((d) => !d.connected);

  const run = (device: BluetoothDevice, action: (id: string) => Promise<void>) => {
    if (busy) return;
    setBusy(device.id);
    setFailed(null);
    action(device.id)
      .catch((e) => {
        console.error(e);
        setFailed(device.id);
      })
      .finally(() => {
        setBusy(null);
        refresh();
      });
  };

  const choose = (device: BluetoothDevice) => (bt?.canConnect ? run(device, ipc.bluetoothConnect) : ipc.openShell("bluetooth"));

  const empty = (Icon: typeof BluetoothIcon, title: string, sub: string) => (
    <div className="popover-empty">
      <span className="empty-icon">
        <Icon size={22} strokeWidth={1.8} />
      </span>
      <span className="empty-title">{title}</span>
      <span className="empty-sub">{sub}</span>
    </div>
  );

  const row = (device: BluetoothDevice) => {
    const Icon = KIND_ICONS[device.kind];
    const level = device.battery;
    const Battery = level === null ? null : level > 70 ? BatteryFull : level > 30 ? BatteryMedium : BatteryLow;
    const sub = busy === device.id && !device.connected ? t.connecting : failed === device.id ? t.failed : t.kinds[device.kind];
    return (
      <li key={device.id} className="bt-item">
        <button className="list-row bt-device" disabled={bt?.canConnect && device.connected} onClick={() => choose(device)}>
          <span className="bt-device-icon" aria-hidden>
            <Icon size={16} strokeWidth={1.8} />
          </span>
          <span className="list-main">
            <span className="bt-device-name">{device.name}</span>
            <span className={`bt-device-kind ${failed === device.id ? "error" : ""}`}>{sub}</span>
          </span>
          {Battery && (
            <span className="bt-battery" aria-label={t.battery(level!)}>
              <Battery size={16} strokeWidth={1.8} aria-hidden />
              {level}%
            </span>
          )}
        </button>
        {bt?.canConnect && device.connected && (
          <button className="btn bt-disconnect" disabled={busy !== null} onClick={() => run(device, ipc.bluetoothDisconnect)}>
            {t.disconnect}
          </button>
        )}
      </li>
    );
  };

  let body;
  if (!bt) body = <span className="popover-loading">…</span>;
  else if (!bt.available) body = empty(BluetoothOff, t.unavailable, t.unavailableSub);
  else if (!bt.powered) body = empty(BluetoothOff, t.off, t.offSub);
  else if (!devices.length) body = empty(BluetoothIcon, t.none, t.noneSub);
  else
    body = (
      <>
        {connected.length > 0 && (
          <>
            <span className="eyebrow bt-eyebrow">{t.connected}</span>
            <ul className="list bt-list">{connected.map(row)}</ul>
          </>
        )}
        {paired.length > 0 && (
          <>
            <span className="eyebrow bt-eyebrow">{t.paired}</span>
            <ul className="list bt-list">{paired.map(row)}</ul>
          </>
        )}
      </>
    );

  const status = !bt?.available ? "" : !bt.powered ? t.offShort : connected.length ? t.connectedCount(connected.length) : t.on;
  return (
    <>
      <div className="popover-header">
        <span className="popover-heading bt-heading">Bluetooth</span>
        {bt?.available && bt.canToggle && <Switch checked={bt.powered} label="Bluetooth" onChange={setPowered} />}
      </div>
      <div className="bt-body">{body}</div>
      <ActionFooter context={status} actions={[{ label: t.settings, keys: "↵", onClick: () => ipc.openShell("bluetooth") }]} />
    </>
  );
}
