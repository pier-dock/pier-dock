import { createContext, useContext } from "react";
import { useConfig } from "../store/config";

type Update = ReturnType<typeof useConfig.getState>["update"];

const DockUpdate = createContext<Update | null>(null);

export const DockUpdateProvider = DockUpdate.Provider;

/** O `update` da config que escreve no perfil desta dock, seja ele o ativo ou não. */
export function useDockUpdate(): Update {
  const store = useConfig((s) => s.update);
  return useContext(DockUpdate) ?? store;
}
