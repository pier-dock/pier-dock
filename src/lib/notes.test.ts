import test from "node:test";
import assert from "node:assert/strict";
import { MAX_NOTE_CHARS, NOTE_COLORS } from "./ipc.ts";

test("notes limit and palette match the Rust side", () => {
  assert.equal(MAX_NOTE_CHARS, 280);
  assert.deepEqual([...NOTE_COLORS], ["yellow", "pink", "blue", "green"]);
});
