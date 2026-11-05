import { test } from "node:test";
import assert from "node:assert/strict";
import { MIN_FIT_ICON, fitDock, fitItems, type FitItem } from "./fit.ts";

const icon = (key: string, fixed = false): FitItem => ({ key, length: 48, fixed });
const wide = (key: string): FitItem => ({ key, length: 176, compact: 48 });

test("quando tudo cabe nada muda", () => {
  const fit = fitItems([icon("a"), wide("w")], 400, 4, 48);
  assert.deepEqual([...fit.compact], []);
  assert.deepEqual(fit.overflow, []);
});

test("compacta só os widgets largos necessários, do fim para o começo", () => {
  const items = [wide("w1"), icon("a"), wide("w2")];
  // Natural: 176 + 48 + 176 + 8 = 408. Compactar w2 dá 280.
  const fit = fitItems(items, 300, 4, 48);
  assert.deepEqual([...fit.compact], ["w2"]);
  assert.deepEqual(fit.overflow, []);
});

test("o que sobra depois de compactar vai para o menu, do fim da barra", () => {
  const items = [icon("start", true), icon("a"), icon("b"), icon("c")];
  // 4 itens = 204; com o botão do menu cabem 3 itens + botão = 48 * 4 + 12.
  const fit = fitItems(items, 200, 4, 48);
  assert.deepEqual(fit.overflow, ["b", "c"]);
});

test("itens fixos nunca saem da barra", () => {
  const items = [icon("start", true), icon("launcher", true), icon("a")];
  const fit = fitItems(items, 100, 4, 48);
  assert.deepEqual(fit.overflow, ["a"]);
});

test("um widget compactado que acaba no menu não conta como compactado", () => {
  const items = [icon("start", true), wide("w1"), wide("w2")];
  const fit = fitItems(items, 110, 4, 48);
  assert.deepEqual(fit.overflow, ["w1", "w2"]);
  assert.deepEqual([...fit.compact], []);
});

test("o resultado não depende do estado anterior", () => {
  const items = [wide("w1"), wide("w2")];
  assert.deepEqual(fitItems(items, 230, 4, 48), fitItems(items, 230, 4, 48));
});

const dock = { iconSize: 48, magnification: 1, rest: 200, count: 10 };

test("com espaço de sobra o ícone e a magnificação ficam como configurados", () => {
  assert.deepEqual(fitDock({ ...dock, magnification: 1.5, available: 1400 }), { icon: 48, scale: 1.5 });
});

test("encolhe o ícone só o necessário para a barra caber", () => {
  // 200 + 10 * 40 = 600.
  assert.equal(fitDock({ ...dock, available: 600 }).icon, 40);
  assert.equal(fitDock({ ...dock, available: 609 }).icon, 40);
  assert.equal(fitDock({ ...dock, available: 610 }).icon, 41);
});

test("o ícone não passa do piso, mesmo sem caber", () => {
  assert.equal(fitDock({ ...dock, available: 100 }).icon, MIN_FIT_ICON);
});

test("um ícone configurado abaixo do piso não é aumentado", () => {
  assert.equal(fitDock({ ...dock, iconSize: 20, available: 100 }).icon, 20);
});

test("barra sem ícones de app não muda", () => {
  assert.deepEqual(fitDock({ ...dock, count: 0, available: 100 }), { icon: 48, scale: 1 });
});

test("a magnificação fica limitada à folga que sobra", () => {
  const roomy = fitDock({ ...dock, magnification: 2, available: 1400 });
  const tight = fitDock({ ...dock, magnification: 2, available: 720 });
  const none = fitDock({ ...dock, magnification: 2, available: 680 });
  assert.equal(roomy.scale, 2);
  assert.equal(tight.icon, 48);
  assert.ok(tight.scale > 1 && tight.scale < 2);
  assert.equal(none.scale, 1);
});

test("o tamanho não alterna quando o resto da barra varia com o ícone", () => {
  // Com 40 o resto é 200 e sobra exatamente o suficiente para 41, mas com 41 o resto sobe e não caberia.
  const shrunk = fitDock({ ...dock, available: 609, rest: 201, previous: 41 }).icon;
  assert.equal(shrunk, 40);
  assert.equal(fitDock({ ...dock, available: 610, previous: shrunk }).icon, 40);
});

test("com folga de verdade o ícone volta ao tamanho configurado", () => {
  assert.equal(fitDock({ ...dock, available: 1400, previous: 30 }).icon, 48);
  assert.equal(fitDock({ ...dock, available: 684, previous: 47 }).icon, 48);
});
