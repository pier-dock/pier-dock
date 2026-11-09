import { lazy, type ComponentType, type LazyExoticComponent } from "react";

type PropsOf<C> = C extends ComponentType<infer P> ? P : never;

/** `lazy` para um módulo que exporta o componente pelo nome, e não como `default`. */
export function lazyNamed<M, K extends keyof M>(
  load: () => Promise<M>,
  name: K,
): LazyExoticComponent<ComponentType<PropsOf<M[K]>>> {
  return lazy(() => load().then((module) => ({ default: module[name] as unknown as ComponentType<PropsOf<M[K]>> })));
}
