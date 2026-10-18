import { test } from "node:test";
import assert from "node:assert/strict";
import type { AiUsage, AiUsageWindow } from "./ipc.ts";
import { CRITICAL, limitLevel, tightest, WARN, windows } from "./aiUsage.ts";

function win(patch: Partial<AiUsageWindow> = {}): AiUsageWindow {
  return { id: "session", minutes: 300, percent: null, resetsIn: null, ...patch };
}

function usage(patch: Partial<AiUsage> = {}): AiUsage {
  return {
    provider: "claude",
    windows: [],
    ageMin: null,
    stale: false,
    todayTokens: 0,
    todayMessages: 0,
    blockTokens: 0,
    topModel: null,
    fetchedAt: 0,
    ...patch,
  };
}

/** Sessão de 5h e semana, como o Claude Code devolve. */
const both = (session: Partial<AiUsageWindow>, week: Partial<AiUsageWindow>) =>
  usage({ windows: [win(session), win({ id: "weekly", minutes: 10080, ...week })] });

test("faixas de cor do anel", () => {
  assert.equal(limitLevel(null), "unknown");
  assert.equal(limitLevel(0), "normal");
  assert.equal(limitLevel(WARN - 1), "normal");
  assert.equal(limitLevel(WARN), "warn");
  assert.equal(limitLevel(CRITICAL), "critical");
  assert.equal(limitLevel(100), "critical");
});

test("o tile mostra a janela mais perto do limite", () => {
  const semana = tightest(both({ percent: 12 }, { percent: 78, resetsIn: 3000 }));
  assert.equal(semana.name, "Semana");
  assert.equal(semana.percent, 78);
  assert.equal(semana.resetsIn, 3000);

  const sessao = tightest(both({ percent: 90 }, { percent: 40 }));
  assert.equal(sessao.name, "Sessão 5h");
});

test("o rótulo da janela sai da duração", () => {
  const [sessao, semana] = windows(both({ percent: 12 }, { percent: 78 }));
  assert.deepEqual([sessao.name, semana.name], ["Sessão 5h", "Semana"]);

  // Janela de outra ferramenta: o Codex só informa quantos minutos ela dura.
  const outras = windows(usage({ windows: [win({ id: "primary", minutes: 480 }), win({ id: "secondary", minutes: 20160 }), win({ id: "x", minutes: null })] }));
  assert.deepEqual(outras.map((w) => w.name), ["Janela de 8h", "Janela de 14d", "Outra janela"]);
});

test("janela desconhecida não concorre", () => {
  // A semana virou (percentual desconhecido): o tile mostra a de 5h, mesmo com percentual menor.
  const only = tightest(both({ percent: 5 }, { percent: null }));
  assert.equal(only.name, "Sessão 5h");
  assert.equal(only.percent, 5);
});

test("sem dado nenhum, o tile fica no estado desconhecido", () => {
  for (const data of [usage({ windows: [win()] }), usage(), null]) {
    const nothing = tightest(data);
    assert.equal(nothing.percent, null);
    assert.equal(limitLevel(nothing.percent), "unknown");
  }
});
