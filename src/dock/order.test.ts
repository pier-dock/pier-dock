import { test } from "node:test";
import assert from "node:assert/strict";
import { addDivider, applyOrder, moveAmong, removeItem, resolveOrder } from "./order.ts";

const app = (id: string) => ({ id, name: id, target: `/Applications/${id}.app` });
const widget = (kind: string, enabled = true) => ({ kind, enabled, size: "small" }) as const;

const base = {
  dockOrder: [] as string[],
  pinnedApps: [app("finder"), app("safari")],
  widgets: [widget("media"), widget("clock"), widget("battery", false)],
  showStartButton: true,
  showLauncher: false,
};

test("config antiga vira os grupos do botão de apps, apps, mídia e sistema, com separadores", () => {
  assert.deepEqual(resolveOrder(base), [
    "start",
    "divider:apps",
    "app:finder",
    "app:safari",
    "divider:media",
    "widget:media",
    "divider:system",
    "widget:clock",
  ]);
});

test("o lançador de apps entra logo depois do botão de apps", () => {
  assert.deepEqual(resolveOrder({ ...base, showLauncher: true }), [
    "start",
    "launcher",
    "divider:apps",
    "app:finder",
    "app:safari",
    "divider:media",
    "widget:media",
    "divider:system",
    "widget:clock",
  ]);
});

test("config antiga com widgets do GitHub ganha um grupo para eles", () => {
  const widgets = [widget("media"), widget("github-mine"), widget("github-review"), widget("clock")];
  assert.deepEqual(resolveOrder({ ...base, showStartButton: false, pinnedApps: [], widgets }), [
    "widget:media",
    "divider:github",
    "widget:github-mine",
    "widget:github-review",
    "divider:system",
    "widget:clock",
  ]);
});

test("ordem salva mistura apps e widgets e é respeitada", () => {
  const dockOrder = ["widget:clock", "app:safari", "start", "divider:x", "app:finder", "widget:media"];
  assert.deepEqual(resolveOrder({ ...base, dockOrder }), dockOrder);
});

test("itens que sumiram saem e os novos entram no fim", () => {
  const config = {
    ...base,
    pinnedApps: [app("safari"), app("music")],
    dockOrder: ["widget:clock", "app:finder", "app:safari", "app:safari", "start", "widget:media"],
  };
  assert.deepEqual(resolveOrder(config), ["widget:clock", "app:safari", "start", "widget:media", "app:music"]);
});

test("botão de apps escondido não aparece", () => {
  const config = { ...base, showStartButton: false, dockOrder: ["start", "app:finder", "app:safari"] };
  assert.deepEqual(resolveOrder(config), ["app:finder", "app:safari", "widget:media", "widget:clock"]);
});

test("applyOrder reordena apps e widgets e deixa os desligados no fim", () => {
  const keys = ["widget:clock", "app:safari", "app:finder", "widget:media"];
  const result = applyOrder(base, keys);
  assert.deepEqual(result.dockOrder, keys);
  assert.deepEqual(result.pinnedApps.map((p) => p.id), ["safari", "finder"]);
  assert.deepEqual(result.widgets.map((w) => w.kind), ["clock", "media", "battery"]);
});

test("removeItem desafixa, desliga, esconde ou apaga conforme o item", () => {
  const dockOrder = ["start", "app:finder", "divider:a", "widget:media", "app:safari", "widget:clock"];
  const config = { ...base, dockOrder };
  assert.deepEqual(removeItem(config, "app:finder").pinnedApps?.map((p) => p.id), ["safari"]);
  assert.equal(removeItem(config, "widget:media").widgets?.find((w) => w.kind === "media")?.enabled, false);
  assert.equal(removeItem(config, "start").showStartButton, false);
  const withoutLauncher = removeItem({ ...config, showLauncher: true }, "launcher");
  assert.equal(withoutLauncher.showLauncher, false);
  assert.ok(!withoutLauncher.dockOrder!.includes("launcher"));
  assert.deepEqual(removeItem(config, "divider:a").dockOrder, ["start", "app:finder", "widget:media", "app:safari", "widget:clock"]);
});

test("addDivider põe um separador novo no fim", () => {
  const order = addDivider(base).dockOrder!;
  assert.deepEqual(order.slice(0, -1), resolveOrder(base));
  assert.match(order[order.length - 1], /^divider:/);
});

test("moveAmong troca só com o vizinho do mesmo tipo", () => {
  const dockOrder = ["app:finder", "widget:media", "app:safari", "widget:clock"];
  const isApp = (k: string) => k.startsWith("app:");
  const result = moveAmong({ ...base, showStartButton: false, dockOrder }, "app:safari", -1, isApp)!;
  assert.deepEqual(result.dockOrder, ["app:safari", "widget:media", "app:finder", "widget:clock"]);
  assert.deepEqual(result.pinnedApps.map((p) => p.id), ["safari", "finder"]);
});

test("moveAmong não passa das pontas", () => {
  const dockOrder = ["app:finder", "app:safari", "widget:media", "widget:clock"];
  const isWidget = (k: string) => k.startsWith("widget:");
  assert.equal(moveAmong({ ...base, dockOrder }, "widget:clock", 1, isWidget), null);
  assert.equal(moveAmong({ ...base, dockOrder }, "widget:media", -1, isWidget), null);
});
