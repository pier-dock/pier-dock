import { test } from "node:test";
import assert from "node:assert/strict";
import { setLang } from "../i18n/index.ts";
import type { PomodoroState } from "./ipc.ts";
import { caption, phaseName, toggleLabel, tone } from "./pomodoro.ts";

const state = (over: Partial<PomodoroState> = {}): PomodoroState => ({
  phase: "focus",
  running: true,
  started: true,
  remainingMs: 600_000,
  totalMs: 1_500_000,
  session: 2,
  sessions: 4,
  focusTodayMs: 0,
  ...over,
});

test("cor do anel", () => {
  assert.equal(tone(null), "paused");
  assert.equal(tone(state({ running: false })), "paused");
  assert.equal(tone(state()), "focus");
  assert.equal(tone(state({ phase: "short-break" })), "break");
  assert.equal(tone(state({ phase: "long-break" })), "break");
});

test("nome da fase", () => {
  setLang("en");
  assert.equal(phaseName(state()), "Focus");
  assert.equal(phaseName(state({ phase: "short-break" })), "Short break");
  assert.equal(phaseName(state({ phase: "long-break" })), "Long break");
});

test("legenda do widget largo", () => {
  setLang("en");
  assert.equal(caption(state()), "Focus · 2 of 4");
  assert.equal(caption(state({ phase: "short-break" })), "Short break");
  // Pausado no meio de uma fase mostra "Paused"; um timer que ainda não começou, não.
  assert.equal(caption(state({ running: false })), "Paused");
  assert.equal(caption(state({ running: false, started: false })), "Focus · 2 of 4");
});

test("rótulo do botão principal", () => {
  setLang("en");
  assert.equal(toggleLabel(null), "Start");
  assert.equal(toggleLabel(state({ started: false, running: false })), "Start");
  assert.equal(toggleLabel(state()), "Pause");
  assert.equal(toggleLabel(state({ running: false })), "Resume");
});

test("a legenda segue o idioma", () => {
  setLang("pt");
  assert.equal(caption(state()), "Foco · 2 de 4");
  assert.equal(toggleLabel(null), "Iniciar");
  setLang("en");
});
