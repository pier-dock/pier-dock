import type { ReactNode } from "react";
import type { Config, PlatformInfo } from "../lib/ipc";
import type { Action } from "../ui/ActionFooter";

/** Contexto e ações que uma aba põe no rodapé das configurações no lugar dos padrões. */
export interface TabFooter {
  context: ReactNode;
  actions: Action[];
}

export interface TabProps {
  config: Config;
  update: (patch: Partial<Config> | ((c: Config) => Partial<Config>)) => void;
  platform: PlatformInfo | null;
  setFooter: (footer: TabFooter | null) => void;
  /** Abre a aba de apps ou de widgets editando este perfil. */
  editProfile?: (id: string, tab: "apps" | "widgets") => void;
}
