import assert from "node:assert/strict";
import { afterEach, beforeEach, describe, it, mock } from "node:test";
import { setActiveInterval, setDockActive } from "./dockActivity.ts";

describe("setActiveInterval", () => {
  let stops: Array<() => void> = [];
  const start = (fn: () => void, ms: number) => {
    const stop = setActiveInterval(fn, ms);
    stops.push(stop);
    return stop;
  };

  beforeEach(() => mock.timers.enable({ apis: ["setInterval"] }));
  afterEach(() => {
    stops.forEach((stop) => stop());
    stops = [];
    setDockActive(true);
    mock.timers.reset();
  });

  it("ticks while the dock is active", () => {
    let calls = 0;
    start(() => calls++, 1000);
    mock.timers.tick(3000);
    assert.equal(calls, 3);
  });

  it("skips ticks while hidden and catches up once when it shows", () => {
    let calls = 0;
    start(() => calls++, 1000);
    setDockActive(false);
    mock.timers.tick(5000);
    assert.equal(calls, 0);
    setDockActive(true);
    assert.equal(calls, 1);
  });

  it("does not fetch on show when no tick was missed", () => {
    let calls = 0;
    start(() => calls++, 60_000);
    setDockActive(false);
    mock.timers.tick(1000);
    setDockActive(true);
    assert.equal(calls, 0);
  });

  it("catches up only once per skipped stretch", () => {
    let calls = 0;
    start(() => calls++, 1000);
    setDockActive(false);
    mock.timers.tick(2000);
    setDockActive(true);
    setDockActive(false);
    setDockActive(true);
    assert.equal(calls, 1);
  });

  it("stops for good after cleanup", () => {
    let calls = 0;
    const stop = start(() => calls++, 1000);
    stop();
    setDockActive(false);
    mock.timers.tick(2000);
    setDockActive(true);
    mock.timers.tick(2000);
    assert.equal(calls, 0);
  });
});
