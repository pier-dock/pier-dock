import { test } from "node:test";
import assert from "node:assert/strict";
import { compareVersions, entriesBetween, parseChangelog, parseInline } from "./changelog.ts";

const MARKDOWN = `# Changelog

Intro that is not part of any version.

## Unreleased

### Not shown
- draft

## 0.8.0

### Bluetooth
Paired devices show up,
and connect with one click.
- **Paired** section
- uses \`openConnection\`

> **GNOME:** reinstall
> the extension.

## v0.7.0

### Old
Text.
`;

test("parses versions and skips non-version sections", () => {
  const entries = parseChangelog(MARKDOWN);
  assert.deepEqual(
    entries.map((e) => e.version),
    ["0.8.0", "0.7.0"],
  );
});

test("parses headings, paragraphs, lists and notes", () => {
  const [entry] = parseChangelog(MARKDOWN);
  assert.deepEqual(
    entry.blocks.map((b) => b.type),
    ["heading", "paragraph", "list", "note"],
  );
  assert.deepEqual(entry.blocks[1], { type: "paragraph", text: [{ text: "Paired devices show up, and connect with one click." }] });
  assert.deepEqual(entry.blocks[2], {
    type: "list",
    items: [[{ text: "Paired", bold: true }, { text: " section" }], [{ text: "uses " }, { text: "openConnection", code: true }]],
  });
  assert.deepEqual(entry.blocks[3], { type: "note", text: [{ text: "GNOME:", bold: true }, { text: " reinstall the extension." }] });
});

test("parses inline bold and code", () => {
  assert.deepEqual(parseInline("a **b** `c` d"), [{ text: "a " }, { text: "b", bold: true }, { text: " " }, { text: "c", code: true }, { text: " d" }]);
  assert.deepEqual(parseInline("no markup"), [{ text: "no markup" }]);
});

test("compares versions numerically", () => {
  assert.ok(compareVersions("0.10.0", "0.9.9") > 0);
  assert.ok(compareVersions("0.7.0", "0.7.1") < 0);
  assert.equal(compareVersions("1.2.3", "1.2.3"), 0);
});

test("picks the versions since the last one seen, newest first", () => {
  const entries = ["0.6.0", "0.7.0", "0.8.0", "0.9.0"].map((version) => ({ version, blocks: [] }));
  assert.deepEqual(
    entriesBetween(entries, "0.6.0", "0.8.0").map((e) => e.version),
    ["0.8.0", "0.7.0"],
  );
  assert.deepEqual(
    entriesBetween(entries, null, "0.8.0").map((e) => e.version),
    ["0.8.0"],
  );
  assert.deepEqual(entriesBetween(entries, "0.8.0", "0.8.0"), []);
});
