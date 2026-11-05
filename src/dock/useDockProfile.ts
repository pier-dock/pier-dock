import { useEffect, useMemo, useState } from "react";
import { ipc, type Config } from "../lib/ipc";
import { currentView } from "../lib/view";
import { useConfig } from "../store/config";
import { profileView, windowProfileId } from "./profiles";

/**
 * O perfil que esta janela mostra. Só uma dock extra pode ter um perfil diferente do ativo; ela
 * espera saber o próprio monitor antes de desenhar, para não piscar o perfil errado.
 */
export function useDockProfile(config: Config | null) {
  const isDock = currentView.view === "dock";
  const [monitor, setMonitor] = useState<string | null | undefined>(isDock ? undefined : null);
  const update = useConfig((s) => s.update);

  useEffect(() => {
    if (isDock) ipc.extraDockMonitor().then(setMonitor).catch(() => setMonitor(null));
  }, [isDock]);

  // Sem isso a extra acompanharia a principal até o primeiro scroll dela.
  const perMonitor = !!config?.profilePerMonitor && config.dockMonitor === "all";
  const linked = !!monitor && !!config?.monitorProfiles[monitor];
  useEffect(() => {
    if (!monitor || !perMonitor || linked) return;
    void update((c) => ({ monitorProfiles: { ...c.monitorProfiles, [monitor]: c.activeProfile } }));
  }, [monitor, perMonitor, linked, update]);

  return useMemo(() => {
    if (!config || monitor === undefined) return null;
    const id = windowProfileId(config, monitor);
    return { id, monitor, view: profileView(config, id) };
  }, [config, monitor]);
}
