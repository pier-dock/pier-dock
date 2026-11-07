import { useCallback, useEffect, useState } from "react";
import { ipc, type AppStream, type AudioState, type BluetoothState, type KeyboardLayouts, type SystemStatus, type Volume, type WifiState } from "../lib/ipc";
import { setActiveInterval } from "../lib/dockActivity";

export function useSystemStatus(enabled = true): SystemStatus | null {
  const [status, setStatus] = useState<SystemStatus | null>(null);
  useEffect(() => {
    if (!enabled) return;
    const refresh = () => ipc.systemStatus().then(setStatus).catch(() => {});
    refresh();
    return setActiveInterval(refresh, 10_000);
  }, [enabled]);
  return status;
}

export function useVolume(): [Volume | null, (patch: Partial<Volume>) => void] {
  const [volume, setVolume] = useState<Volume | null>(null);
  useEffect(() => {
    const refresh = () => ipc.getVolume().then(setVolume).catch(() => {});
    refresh();
    return setActiveInterval(refresh, 3_000);
  }, []);
  const update = useCallback((patch: Partial<Volume>) => {
    setVolume((v) => (v ? { ...v, ...patch } : v));
    ipc.setVolume(patch).catch(() => {});
  }, []);
  return [volume, update];
}

/** Saídas e apps do popover de volume, com atualização otimista. */
export function useAudio(intervalMs = 1_500) {
  const [state, setState] = useState<AudioState | null>(null);
  const refresh = useCallback(() => ipc.audioStatus().then(setState).catch(() => {}), []);
  useEffect(() => {
    refresh();
    const id = setInterval(refresh, intervalMs);
    return () => clearInterval(id);
  }, [intervalMs, refresh]);
  const setOutput = useCallback(
    (id: string) => {
      setState((s) => s && { ...s, outputs: s.outputs.map((o) => ({ ...o, default: o.id === id })) });
      ipc.setOutputDevice(id).catch(console.error).finally(refresh);
    },
    [refresh],
  );
  const setApp = useCallback((id: string, patch: Partial<Pick<AppStream, "level" | "muted">>) => {
    setState((s) => (s?.apps ? { ...s, apps: s.apps.map((a) => (a.id === id ? { ...a, ...patch } : a)) } : s));
    ipc.setAppVolume(id, patch).catch(console.error);
  }, []);
  const setAppOutput = useCallback(async (id: string, output: string) => {
    await ipc.setAppOutput(id, output);
    await refresh();
  }, [refresh]);
  return { audio: state, setOutput, setApp, setAppOutput };
}

/** Redes do popover de Wi-Fi: estado a cada 3 s e uma busca nova a cada 10 s. */
export function useWifi() {
  const [state, setState] = useState<WifiState | null>(null);
  const refresh = useCallback(() => ipc.wifiStatus().then(setState).catch(() => {}), []);
  useEffect(() => {
    const scan = () => ipc.wifiScan().catch(() => {}).finally(refresh);
    refresh();
    scan();
    const poll = setInterval(refresh, 3_000);
    const rescan = setInterval(scan, 10_000);
    return () => {
      clearInterval(poll);
      clearInterval(rescan);
    };
  }, [refresh]);
  const setEnabled = useCallback(
    (enabled: boolean) => {
      setState((s) => s && { ...s, enabled, networks: enabled ? s.networks : [] });
      ipc
        .wifiSetEnabled(enabled)
        .catch(console.error)
        .finally(() => (enabled ? ipc.wifiScan().catch(() => {}).finally(refresh) : refresh()));
    },
    [refresh],
  );
  return { wifi: state, refresh, setEnabled };
}

/** Só consulta com `enabled`: no macOS a primeira leitura pede a permissão de Bluetooth. */
export function useBluetooth(enabled = true, intervalMs = 10_000): [BluetoothState | null, (powered: boolean) => Promise<void>, () => void] {
  const [state, setState] = useState<BluetoothState | null>(null);
  const refresh = useCallback(() => ipc.bluetoothStatus().then(setState).catch(() => {}), []);
  useEffect(() => {
    if (!enabled) return;
    refresh();
    return setActiveInterval(refresh, intervalMs);
  }, [enabled, intervalMs, refresh]);
  const setPowered = useCallback(async (powered: boolean) => {
    try { await ipc.setBluetooth(powered); }
    finally { await refresh(); }
  }, [refresh]);
  return [state, setPowered, refresh];
}

/** Não há evento para a troca de layout, então o ativo é consultado a cada `intervalMs`. */
export function useKeyboardLayouts(enabled = true, intervalMs = 1_000) {
  const [keyboard, setKeyboard] = useState<KeyboardLayouts | null>(null);
  const refresh = useCallback(() => ipc.keyboardLayouts().then(setKeyboard).catch(() => {}), []);
  useEffect(() => {
    if (!enabled) return;
    refresh();
    return setActiveInterval(refresh, intervalMs);
  }, [enabled, intervalMs, refresh]);
  const setActive = useCallback((id: string) => setKeyboard((k) => k && { ...k, active: id }), []);
  return { keyboard, refresh, setActive };
}

export function useNow(intervalMs = 1000): Date {
  const [now, setNow] = useState(new Date());
  useEffect(() => {
    return setActiveInterval(() => setNow(new Date()), intervalMs);
  }, [intervalMs]);
  return now;
}
