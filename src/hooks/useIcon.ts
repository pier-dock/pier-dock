import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";

const cache = new Map<string, Promise<string | null>>();

function loadIcon(key: string, small: boolean): Promise<string | null> {
  const id = `${small ? "s" : "l"}:${key}`;
  let pending = cache.get(id);
  if (!pending) {
    // Falha transitória (pacote da Store ainda não registrado no boot) não pode virar null para sempre.
    pending = ipc.getIcon(key, small).catch(() => {
      cache.delete(id);
      return null;
    });
    cache.set(id, pending);
  }
  return pending;
}

/** `small` serve ícones até ~48 px na tela (listas, popovers): a versão cheia é de 256 px. */
export function useIcon(key: string | null | undefined, small = false): string | null {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    if (!key) return;
    let alive = true;
    loadIcon(key, small).then((s) => alive && setSrc(s));
    return () => {
      alive = false;
    };
  }, [key, small]);
  return src;
}
