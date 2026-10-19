import { test } from "node:test";
import assert from "node:assert/strict";
import { PREVIEW_GAP, PREVIEW_HEIGHT, dockMetrics, previewLeft } from "./layout.ts";

test("a barra flutuante reserva a altura e as duas margens", () => {
  const metrics = dockMetrics(40, 1.6);
  assert.equal(metrics.barHeight, 64);
  assert.equal(metrics.reservedHeight, 80);
});

test("na largura total a área reservada é a própria barra", () => {
  const metrics = dockMetrics(40, 1.6, true);
  assert.equal(metrics.reservedHeight, metrics.barHeight);
  // A janela da dock ainda cobre a faixa de prévias acima da barra.
  assert.equal(metrics.windowHeight, metrics.barHeight + PREVIEW_HEIGHT + PREVIEW_GAP);
});

test("a faixa de prévias aumenta a área de hover, mas não a janela", () => {
  const base = dockMetrics(40, 1.6);
  const previewSpace = PREVIEW_HEIGHT + PREVIEW_GAP;
  const withPreview = dockMetrics(40, 1.6, false, null, previewSpace);
  assert.equal(base.hoverHeadroom, 72);
  assert.equal(withPreview.hoverHeadroom, previewSpace);
  assert.equal(withPreview.windowHeight, base.windowHeight);
  assert.equal(base.windowHeight, base.reservedHeight + previewSpace);
  // Com magnificação grande a folga dela passa a da faixa, e a janela acompanha.
  assert.ok(dockMetrics(64, 3).windowHeight > dockMetrics(64, 3).reservedHeight + previewSpace);
  // Uma folga menor que a da magnificação não pode encolher a janela.
  assert.equal(dockMetrics(40, 2.6, false, null, 0).hoverHeadroom, dockMetrics(40, 2.6, false, null, 10).hoverHeadroom);
});

test("a faixa de prévias fica centrada no ícone", () => {
  assert.equal(previewLeft(500, 1000, 200), 400);
});

test("a faixa é presa às bordas da tela", () => {
  assert.equal(previewLeft(50, 1000, 200), 8);
  assert.equal(previewLeft(990, 1000, 200), 792);
});

test("faixa mais larga que a janela encosta na esquerda", () => {
  assert.equal(previewLeft(500, 400, 500), 8);
  // Também quando sobra menos que as duas folgas, senão a faixa sairia pela direita.
  assert.equal(previewLeft(500, 400, 390), 8);
});

test("a altura automática acompanha o ícone", () => {
  assert.equal(dockMetrics(32, 1.6).barHeight, 56);
  assert.equal(dockMetrics(72, 1.6).barHeight, 96);
  assert.equal(dockMetrics(40, 1.6).itemBottom, 3);
});

test("a altura escolhida à mão muda a barra, a reserva e o pé dos ícones", () => {
  const tall = dockMetrics(40, 1.6, true, 80);
  assert.equal(tall.barHeight, 80);
  assert.equal(tall.reservedHeight, 80);
  // Metade da sobra embaixo dos ícones, que ficam presos à base da barra.
  assert.equal(tall.itemBottom, 3 + (80 - 64) / 2);
});

test("altura menor que o ícone é limitada, para os ícones não saírem da barra", () => {
  assert.equal(dockMetrics(40, 1.6, true, 10).barHeight, 56);
  assert.equal(dockMetrics(40, 1.6, true, 1000).barHeight, 240);
});
