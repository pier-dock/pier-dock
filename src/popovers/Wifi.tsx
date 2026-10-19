import { useEffect, useRef, useState } from "react";
import { Lock, MapPin, Wifi as WifiIcon, WifiHigh, WifiLow, WifiOff, WifiZero, X } from "lucide-react";
import { ipc, type WifiNetwork } from "../lib/ipc";
import { useWifi } from "../hooks/useSystem";
import { ActionFooter } from "../ui/ActionFooter";
import { Switch } from "../ui/Switch";
import { useT } from "../i18n";

const BAR_ICONS = [WifiZero, WifiLow, WifiLow, WifiHigh, WifiIcon, WifiIcon];

function Signal({ network }: { network: WifiNetwork }) {
  const Icon = BAR_ICONS[network.signalBars] ?? WifiIcon;
  return (
    <span className="wifi-signal" aria-hidden>
      <Icon size={16} strokeWidth={1.8} />
      {network.secured && <Lock size={9} strokeWidth={2.4} className="wifi-lock" />}
    </span>
  );
}

/** O painel só recebe o teclado depois do `focus_panel` (ver `panels.rs`). */
function PasswordForm({ ssid, error, busy, onSubmit, onCancel }: { ssid: string; error: string | null; busy: boolean; onSubmit: (password: string) => void; onCancel: () => void }) {
  const t = useT().wifi;
  const input = useRef<HTMLInputElement>(null);
  const [password, setPassword] = useState("");
  useEffect(() => {
    ipc
      .focusPanel()
      .catch(() => {})
      .finally(() => input.current?.focus());
  }, []);
  return (
    <form
      className="wifi-password"
      onSubmit={(e) => {
        e.preventDefault();
        if (password) onSubmit(password);
      }}
    >
      <input
        ref={input}
        className="text-input"
        type="password"
        value={password}
        placeholder={t.password}
        aria-label={t.passwordFor(ssid)}
        disabled={busy}
        onChange={(e) => setPassword(e.target.value)}
      />
      <button className="btn btn-primary" type="submit" disabled={busy || !password}>
        {busy ? t.connecting : t.connect}
      </button>
      <button className="icon-button" type="button" aria-label={t.cancel} onClick={onCancel}>
        <X size={15} />
      </button>
      {error && <span className="wifi-error">{error}</span>}
    </form>
  );
}

export function WifiPanel() {
  const { wifi, refresh, setEnabled } = useWifi();
  const t = useT().wifi;
  const [expanded, setExpanded] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<{ ssid: string; message: string } | null>(null);

  const networks = wifi?.networks ?? [];
  const current = networks.find((n) => n.connected);
  const others = networks.filter((n) => !n.connected);

  // A resposta ao pedido chega sem aviso: busca de novo algumas vezes para os nomes aparecerem logo.
  const requestPermission = () => {
    ipc.wifiRequestPermission();
    for (const delay of [2_000, 5_000]) setTimeout(() => ipc.wifiScan().catch(() => {}).finally(refresh), delay);
  };

  const connect = (network: WifiNetwork, password: string | null) => {
    if (!wifi?.canConnect) return ipc.openShell("network");
    setBusy(network.ssid);
    setError(null);
    ipc
      .wifiConnect(network.ssid, password)
      .then(() => setExpanded(null))
      .catch((e) => {
        const code = String(e);
        if (code === "unsupported") return ipc.openShell("network");
        if (code === "auth" && network.secured) {
          setExpanded(network.ssid);
          return setError({ ssid: network.ssid, message: password ? t.wrongPassword : t.needsPassword });
        }
        setError({ ssid: network.ssid, message: code === "auth" ? t.failed : code });
      })
      .finally(() => {
        setBusy(null);
        refresh();
      });
  };

  const choose = (network: WifiNetwork) => {
    if (busy) return;
    if (network.secured && !network.known) {
      setError(null);
      setExpanded(network.ssid);
    } else connect(network, null);
  };

  const disconnect = () => {
    setBusy("");
    ipc
      .wifiDisconnect()
      .catch(console.error)
      .finally(() => {
        setBusy(null);
        refresh();
      });
  };

  const empty = (Icon: typeof WifiIcon, title: string, sub: string, action?: { label: string; onClick: () => void }) => (
    <div className="popover-empty">
      <span className="empty-icon">
        <Icon size={22} strokeWidth={1.8} />
      </span>
      <span className="empty-title">{title}</span>
      <span className="empty-sub">{sub}</span>
      {action && (
        <button className="btn wifi-empty-action" onClick={action.onClick}>
          {action.label}
        </button>
      )}
    </div>
  );

  const row = (network: WifiNetwork) => (
    <li key={network.ssid} className="wifi-item">
      <button className="list-row wifi-row" disabled={network.connected} aria-expanded={expanded === network.ssid} onClick={() => choose(network)}>
        <Signal network={network} />
        <span className="list-main">
          <span className="wifi-name">{network.ssid}</span>
          {(network.connected || busy === network.ssid || (error?.ssid === network.ssid && expanded !== network.ssid)) && (
            <span className={`wifi-sub ${error?.ssid === network.ssid ? "error" : ""}`}>
              {busy === network.ssid ? t.connecting : error?.ssid === network.ssid ? error.message : t.connected}
            </span>
          )}
        </span>
        {network.known && !network.connected && <span className="wifi-saved">{t.saved}</span>}
      </button>
      {network.connected && (
        <button className="btn wifi-disconnect" disabled={busy !== null} onClick={disconnect}>
          {t.disconnect}
        </button>
      )}
      {expanded === network.ssid && (
        <PasswordForm
          ssid={network.ssid}
          busy={busy === network.ssid}
          error={error?.ssid === network.ssid ? error.message : null}
          onSubmit={(password) => connect(network, password)}
          onCancel={() => {
            setExpanded(null);
            setError(null);
          }}
        />
      )}
    </li>
  );

  let body;
  if (!wifi) body = <span className="popover-loading">…</span>;
  else if (!wifi.available) body = empty(WifiOff, t.unavailable, t.unavailableSub);
  else if (!wifi.enabled) body = empty(WifiOff, t.off, t.offSub);
  else if (wifi.needsPermission) body = empty(MapPin, t.permission, t.permissionSub, { label: t.allow, onClick: requestPermission });
  else if (!networks.length) body = empty(WifiIcon, t.searching, t.searchingSub);
  else
    body = (
      <>
        {current && (
          <>
            <span className="eyebrow wifi-eyebrow">{t.current}</span>
            <ul className="list wifi-list">{row(current)}</ul>
          </>
        )}
        {!!others.length && (
          <>
            <span className="eyebrow wifi-eyebrow">{t.available}</span>
            <ul className="list wifi-list">{others.map(row)}</ul>
          </>
        )}
      </>
    );

  const status = !wifi?.available ? "" : !wifi.enabled ? t.offShort : (current?.ssid ?? t.notConnected);
  return (
    <>
      <div className="popover-header">
        <span className="popover-heading wifi-heading">Wi‑Fi</span>
        {wifi?.available && wifi.canToggle && <Switch checked={wifi.enabled} label="Wi-Fi" onChange={setEnabled} />}
      </div>
      <div className="wifi-body">{body}</div>
      <ActionFooter context={status} actions={[{ label: t.settings, onClick: () => ipc.openShell("network") }]} />
    </>
  );
}
