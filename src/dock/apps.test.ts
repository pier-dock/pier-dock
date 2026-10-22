import { test } from "node:test";
import assert from "node:assert/strict";
import type { AppEntry, WindowInfo } from "../lib/ipc.ts";
import { buildDockApps, toPinned } from "./apps.ts";
import { appKey } from "./order.ts";

const win = (over: Partial<WindowInfo> = {}): WindowInfo => ({
  id: 1,
  title: "Janela",
  exe: "C:\\Apps\\notepad.exe",
  minimized: false,
  pid: 100,
  ...over,
});

const chrome: AppEntry = { id: "chrome", name: "Chrome", target: "C:\\Chrome\\chrome.exe", exe: "C:\\Chrome\\chrome.exe" };

test("a janela vai para o app fixado com o mesmo exe, sem diferenciar maiúsculas", () => {
  const apps = buildDockApps([chrome], [win({ exe: "c:\\chrome\\CHROME.EXE" })], true);
  assert.equal(apps.length, 1);
  assert.equal(apps[0]!.key, appKey("chrome"));
  assert.equal(apps[0]!.windows.length, 1);
});

test("o AUMID tem prioridade sobre o exe", () => {
  const edge: AppEntry = { id: "edge", name: "Edge", target: "edge", exe: "C:\\e\\msedge.exe", aumid: "MSEdge" };
  const other: AppEntry = { id: "other", name: "Outro", target: "o", exe: "C:\\e\\msedge.exe", aumid: "Outro" };
  const apps = buildDockApps([other, edge], [win({ exe: "C:\\e\\msedge.exe", aumid: "msedge" })], true);
  assert.equal(apps[0]!.windows.length, 0);
  assert.equal(apps[1]!.windows.length, 1);
});

test("um PWA que roda no chrome.exe não cai no Chrome fixado", () => {
  const pwa = win({ exe: "C:\\Chrome\\chrome.exe", aumid: "Chrome._crx_abcdef", title: "YouTube Music" });
  const apps = buildDockApps([chrome], [pwa], true);
  assert.equal(apps.length, 2);
  assert.equal(apps[0]!.windows.length, 0);
  assert.equal(apps[1]!.key, "run:chrome._crx_abcdef");
});

test("janelas de um app não fixado viram um grupo só, com o nome do exe", () => {
  const apps = buildDockApps([], [win({ id: 1, exe: "C:\\Apps\\my-editor.exe" }), win({ id: 2, exe: "C:\\Apps\\my-editor.exe" })], true);
  assert.equal(apps.length, 1);
  assert.equal(apps[0]!.name, "My Editor");
  assert.equal(apps[0]!.windows.length, 2);
  assert.equal(apps[0]!.pinned, undefined);
});

test("sem mostrar os não fixados, só os fixados aparecem", () => {
  const apps = buildDockApps([chrome], [win({ exe: "C:\\Apps\\outro.exe" })], false);
  assert.deepEqual(apps.map((a) => a.key), [appKey("chrome")]);
});

test("janela sem exe nem AUMID é ignorada", () => {
  assert.equal(buildDockApps([], [win({ exe: "", aumid: null })], true).length, 0);
});

test("o app instalado dá o nome e o destino de abertura do grupo", () => {
  const installed: AppEntry = { id: "code", name: "Visual Studio Code", target: "C:\\vs\\Code.exe", exe: "C:\\vs\\Code.exe" };
  const [app] = buildDockApps([], [win({ exe: "C:\\vs\\code.exe" })], true, [installed]);
  assert.equal(app!.name, "Visual Studio Code");
  assert.equal(app!.launchTarget, "C:\\vs\\Code.exe");
  assert.equal(app!.installed, installed);
});

test("fixar um app aberto usa o instalado, ou monta a entrada pela primeira janela", () => {
  const installed: AppEntry = { id: "code", name: "VS Code", target: "t", exe: "e" };
  const [withInstalled] = buildDockApps([], [win({ exe: "E" })], true, [installed]);
  assert.equal(toPinned(withInstalled!), installed);

  const [plain] = buildDockApps([], [win({ exe: "C:\\Apps\\notepad.exe" })], true);
  const entry = toPinned(plain!);
  assert.equal(entry.id, plain!.key);
  assert.equal(entry.name, "Notepad");
  assert.equal(entry.exe, "C:\\Apps\\notepad.exe");
  assert.equal(entry.aumid, null);
});

test("o badge usa o maior entre a soma dos títulos e o badge do sistema", () => {
  const apps = buildDockApps([chrome], [win({ exe: chrome.exe!, title: "(2) Chrome", badge: 5 }), win({ id: 2, exe: chrome.exe!, title: "(1) Chrome" })], true);
  assert.equal(apps[0]!.badge, 5);
  const byTitle = buildDockApps([chrome], [win({ exe: chrome.exe!, title: "(4) Chrome", badge: 1 })], true);
  assert.equal(byTitle[0]!.badge, 4);
});
