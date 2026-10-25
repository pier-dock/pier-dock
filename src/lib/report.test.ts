import { test } from "node:test";
import assert from "node:assert/strict";
import { issueRef } from "./report.ts";

test("lê repositório e número da URL da issue", () => {
  assert.deepEqual(issueRef("https://github.com/pier-dock/pier-dock/issues/42"), { repo: "pier-dock/pier-dock", number: 42 });
  assert.deepEqual(issueRef("https://github.com/pier-dock/pier-dock/issues/7/"), { repo: "pier-dock/pier-dock", number: 7 });
  assert.deepEqual(issueRef("https://github.com/pier-dock/pier-dock/issues/7#issuecomment-1"), { repo: "pier-dock/pier-dock", number: 7 });
});

test("ignora URLs que não são de uma issue", () => {
  assert.equal(issueRef("https://github.com/pier-dock/pier-dock/issues/new?title=oi"), null);
  assert.equal(issueRef("https://example.com/pier-dock/pier-dock/issues/1"), null);
  assert.equal(issueRef(""), null);
});
