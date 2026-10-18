import { test } from "node:test";
import assert from "node:assert/strict";
import { filterApps } from "./launcherSearch.ts";

const app = (name: string) => ({ id: name.toLowerCase(), name, target: `/usr/share/applications/${name}.desktop` });

test("ordena por nome, sem mexer na lista original", () => {
  const apps = [app("Zen"), app("dolphin"), app("Arc")];
  assert.deepEqual(
    filterApps(apps, "").map((a) => a.name),
    ["Arc", "dolphin", "Zen"],
  );
  assert.deepEqual(
    apps.map((a) => a.name),
    ["Zen", "dolphin", "Arc"],
  );
});

test("filtra por nome sem depender de maiúsculas nem de acentos", () => {
  const apps = [app("Configurações"), app("Navegador"), app("Calculadora")];
  assert.deepEqual(
    filterApps(apps, "config").map((a) => a.name),
    ["Configurações"],
  );
  assert.deepEqual(
    filterApps(apps, "CALCULADORA").map((a) => a.name),
    ["Calculadora"],
  );
  assert.deepEqual(filterApps(apps, "  ").length, 3);
  assert.deepEqual(filterApps(apps, "zzz"), []);
});
