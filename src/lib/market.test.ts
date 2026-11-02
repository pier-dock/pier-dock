import { test } from "node:test";
import assert from "node:assert/strict";
import { formatChange, formatCompact, formatPrice, formatQuote, isFreeTicker, parseSymbols } from "./market.ts";

const noNbsp = (value: string) => value.replace(/\s/g, " ");

test("separa símbolos digitados e remove repetidos", () => {
  assert.deepEqual(parseSymbols("petr4, VALE3  MXRF11\npetr4;"), ["PETR4", "VALE3", "MXRF11"]);
  assert.deepEqual(parseSymbols("   "), []);
});

test("reconhece os ativos que a brapi responde sem chave", () => {
  assert.equal(isFreeTicker("petr4"), true);
  assert.equal(isFreeTicker("MXRF11"), false);
});

test("preço com a moeda e casas suficientes para cripto barata", () => {
  assert.equal(noNbsp(formatPrice(52.71)), "R$ 52,71");
  assert.equal(noNbsp(formatPrice(0.000123)), "R$ 0,000123");
  assert.equal(noNbsp(formatPrice(1.5, "USD")), "US$ 1,50");
});

test("cotação curta para o tile", () => {
  assert.equal(noNbsp(formatQuote(52.71)), "52,71");
  assert.equal(noNbsp(formatQuote(1.5)), "1,50");
  assert.equal(noNbsp(formatQuote(0.000123)), "0,0001");
  assert.equal(noNbsp(formatQuote(380_000)), "380 mil");
});

test("variação com sinal", () => {
  assert.equal(formatChange(0.98), "+0,98%");
  assert.equal(formatChange(-0.2), "-0,20%");
});

test("valores compactos para volume e valor de mercado", () => {
  assert.equal(noNbsp(formatCompact(726_564_863_632)), "726,6 bi");
  assert.equal(noNbsp(formatCompact(61_743_200)), "61,7 mi");
});
