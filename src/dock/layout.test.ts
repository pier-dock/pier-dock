import { test } from "node:test";
import assert from "node:assert/strict";
import { PREVIEW_GAP, PREVIEW_HEIGHT, PREVIEW_WIDTH, barRect, dockMetrics, previewAlong, previewSpace } from "./layout.ts";

test("a barra flutuante reserva a altura e as duas margens", () => {
  const metrics = dockMetrics(40, 1.6);
  assert.equal(metrics.barThickness, 64);
  assert.equal(metrics.reserved, 80);
});

test("nas bordas laterais a espessura é a largura da barra", () => {
  const left = dockMetrics(40, 1.6, false, null, 0, "left");
  const right = dockMetrics(40, 1.6, false, null, 0, "right");
  assert.equal(left.barThickness, 64);
  assert.equal(right.reserved, 80);
  // A dica ao lado da barra pede mais espaço que a dica acima dela.
  assert.ok(left.hoverHeadroom > dockMetrics(40, 1.6).hoverHeadroom);
});

test("de ponta a ponta a área reservada é a própria barra", () => {
  const metrics = dockMetrics(40, 1.6, true);
  assert.equal(metrics.reserved, metrics.barThickness);
  // A janela da dock ainda cobre a folga da magnificação e das dicas ao lado da barra.
  assert.equal(metrics.windowSize, metrics.barThickness + metrics.hoverHeadroom);
  // Na vertical, de ponta a ponta, a reserva é a largura da barra.
  assert.equal(dockMetrics(40, 1.6, true, null, 0, "right").reserved, 64);
});

test("a faixa de prévias aumenta a folga e o tamanho da janela", () => {
  const base = dockMetrics(40, 1.6);
  const space = previewSpace("bottom", true);
  assert.equal(space, PREVIEW_HEIGHT + PREVIEW_GAP);
  const withPreview = dockMetrics(40, 1.6, false, null, space);
  assert.equal(base.hoverHeadroom, 72);
  assert.equal(withPreview.hoverHeadroom, space);
  assert.equal(withPreview.windowSize, withPreview.reserved + space);
  // Uma folga menor que a da magnificação não pode encolher a janela.
  assert.equal(dockMetrics(40, 2.6, false, null, 0).hoverHeadroom, dockMetrics(40, 2.6, false, null, 10).hoverHeadroom);
});

test("nas laterais a faixa de prévias cabe na janela desde o começo", () => {
  const closed = dockMetrics(40, 1.6, false, null, previewSpace("left", false), "left");
  const open = dockMetrics(40, 1.6, false, null, previewSpace("left", true), "left");
  // A folga com a faixa fechada já é a que a faixa aberta precisa: abrir não redimensiona a janela.
  assert.equal(open.windowSize, closed.windowSize);
  assert.ok(closed.hoverHeadroom >= previewSpace("left", true));
  assert.ok(closed.hoverHeadroom > dockMetrics(40, 1.6).hoverHeadroom);
});

test("a faixa de prévias fica centrada no ícone", () => {
  assert.equal(previewAlong(500, 1000, 200), 400);
});

test("a faixa é presa às bordas da tela", () => {
  assert.equal(previewAlong(50, 1000, 200), 8);
  assert.equal(previewAlong(990, 1000, 200), 792);
});

test("faixa mais larga que a janela encosta no começo", () => {
  assert.equal(previewAlong(500, 400, 500), 8);
  // Também quando sobra menos que as duas folgas, senão a faixa sairia pela direita.
  assert.equal(previewAlong(500, 400, 390), 8);
});

test("a espessura automática acompanha o ícone", () => {
  assert.equal(dockMetrics(32, 1.6).barThickness, 56);
  assert.equal(dockMetrics(72, 1.6).barThickness, 96);
  assert.equal(dockMetrics(40, 1.6).itemOffset, 3);
});

test("a espessura escolhida à mão muda a barra, a reserva e o pé dos ícones", () => {
  const tall = dockMetrics(40, 1.6, true, 80);
  assert.equal(tall.barThickness, 80);
  assert.equal(tall.reserved, 80);
  // Metade da sobra do lado de dentro da barra, onde ficam os ícones presos à borda da tela.
  assert.equal(tall.itemOffset, 3 + (80 - 64) / 2);
});

test("espessura menor que o ícone é limitada, para os ícones não saírem da barra", () => {
  assert.equal(dockMetrics(40, 1.6, true, 10).barThickness, 56);
  assert.equal(dockMetrics(40, 1.6, true, 1000).barThickness, 240);
});

test("o retângulo da barra acompanha a borda escolhida", () => {
  const window = { width: 1920, height: 1080 };
  const bar = { width: 600, height: 64 };
  assert.deepEqual(barRect("bottom", false, 8, bar, window), { x: 660, y: 1008, width: 600, height: 64 });
  assert.deepEqual(barRect("top", false, 8, bar, window), { x: 660, y: 8, width: 600, height: 64 });
  // De ponta a ponta a barra é medida encostada na borda da tela.
  assert.deepEqual(barRect("bottom", true, 8, { width: 1920, height: 64 }, window), { x: 0, y: 1016, width: 1920, height: 64 });
  const vertical = { width: 64, height: 600 };
  assert.deepEqual(barRect("left", false, 8, vertical, window), { x: 8, y: 240, width: 64, height: 600 });
  assert.deepEqual(barRect("right", false, 8, vertical, window), { x: 1848, y: 240, width: 64, height: 600 });
  assert.deepEqual(barRect("right", true, 8, { width: 64, height: 1080 }, window), { x: 1856, y: 0, width: 64, height: 1080 });
});
