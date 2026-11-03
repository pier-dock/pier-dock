import test from "node:test";
import assert from "node:assert/strict";
import { shallow } from "zustand/shallow";
import { EMPTY_WIDGET_DATA, selectWidgetData } from "./widgetData.ts";

test("media updates leave other widget selections shallowly equal", () => {
  const before = EMPTY_WIDGET_DATA;
  const after = { ...before, media: { sessions: [], current: "new" } };
  assert.equal(shallow(selectWidgetData("media", before), selectWidgetData("media", after)), false);
  for (const kind of ["github-mine", "network", "calendar", "weather", "clock"] as const) {
    assert.ok(shallow(selectWidgetData(kind, before), selectWidgetData(kind, after)));
  }
});

test("keyboard layout changes update only the keyboard layout widget", () => {
  const after = { ...EMPTY_WIDGET_DATA, keyboard: { layouts: [], active: "04090409" } };
  assert.equal(shallow(selectWidgetData("keyboard-layout", EMPTY_WIDGET_DATA), selectWidgetData("keyboard-layout", after)), false);
  for (const kind of ["bluetooth", "battery", "clock"] as const) {
    assert.ok(shallow(selectWidgetData(kind, EMPTY_WIDGET_DATA), selectWidgetData(kind, after)));
  }
});

test("weather configuration changes update weather but not other widgets", () => {
  const after = { ...EMPTY_WIDGET_DATA, weatherConfigured: true };
  assert.equal(selectWidgetData("weather", after).weatherConfigured, true);
  assert.ok(shallow(selectWidgetData("battery", EMPTY_WIDGET_DATA), selectWidgetData("battery", after)));
});

test("market quotes update only the market widget", () => {
  const after = { ...EMPTY_WIDGET_DATA, market: { assets: [], unavailable: [], missing: [], fetchedAt: 1 } };
  assert.equal(selectWidgetData("market", after).market, after.market);
  for (const kind of ["weather", "calendar", "system-usage"] as const) {
    assert.ok(shallow(selectWidgetData(kind, EMPTY_WIDGET_DATA), selectWidgetData(kind, after)));
  }
});

test("notes update only the notes widget", () => {
  const after = { ...EMPTY_WIDGET_DATA, notes: [{ id: "a", text: "x", color: "yellow" as const, modifiedAt: 1 }] };
  assert.equal(selectWidgetData("notes", after).notes, after.notes);
  assert.equal(selectWidgetData("clock", after).notes, EMPTY_WIDGET_DATA.notes);
});
