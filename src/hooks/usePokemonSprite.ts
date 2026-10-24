import { useEffect, useState } from "react";
import { ipc } from "../lib/ipc";

const sprites = new Map<number, string>();

export function usePokemonSprite(id: number | null): string | null {
  const [src, setSrc] = useState<string | null>(id === null ? null : (sprites.get(id) ?? null));
  useEffect(() => {
    if (id === null) {
      setSrc(null);
      return;
    }
    const cached = sprites.get(id);
    setSrc(cached ?? null);
    if (cached) return;
    let alive = true;
    ipc
      .pokemonSprite(id)
      .then((url) => {
        sprites.set(id, url);
        if (alive) setSrc(url);
      })
      .catch(console.error);
    return () => {
      alive = false;
    };
  }, [id]);
  return src;
}
