import assert from "node:assert/strict";
import test from "node:test";
import { createProfileGesture, QUIET_MS, type GestureStep } from "./profileGesture.ts";

const sample = (deltaY: number, time: number, extra = {}) => ({ deltaX: 0, deltaY, deltaMode: 0, time, ...extra });
const commits = (steps: GestureStep[]) => steps.filter((s) => s.kind === "commit");

/** Um movimento de trackpad: sobe até o pico e decai por inércia, com eventos a cada 16 ms. */
function flick(peak: number, start = 0) {
  const deltas = [peak * 0.2, peak * 0.5, peak];
  for (let d = peak; d > 1; d *= 0.92) deltas.push(d);
  return deltas.map((d, i) => sample(d, start + i * 16));
}

test("a small scroll tracks without switching", () => {
  const gesture = createProfileGesture();
  const step = gesture.wheel(sample(11, 0));
  assert.equal(step.kind, "track");
  assert.equal((step as { progress: number }).progress, 0.1);
});

test("scrolling past the threshold switches in the scroll direction", () => {
  assert.deepEqual(createProfileGesture().wheel(sample(60, 0)), { kind: "commit", direction: 1 });
  assert.deepEqual(createProfileGesture().wheel(sample(-60, 0)), { kind: "commit", direction: -1 });
});

test("a whole trackpad flick with inertia switches only once", () => {
  const gesture = createProfileGesture();
  const steps = flick(70).map((s) => gesture.wheel(s));
  assert.equal(commits(steps).length, 1);
});

test("a new gesture after the inertia switches again", () => {
  const gesture = createProfileGesture();
  const first = flick(40);
  first.forEach((s) => gesture.wheel(s));
  const start = first[first.length - 1].time + QUIET_MS + 1;
  const steps = flick(40, start).map((s) => gesture.wheel(s));
  assert.equal(commits(steps).length, 1);
});

test("a new flick during the inertia tail is not swallowed", () => {
  const gesture = createProfileGesture();
  [sample(60, 0), sample(30, 100), sample(20, 190), sample(4, 206)].forEach((s) => gesture.wheel(s));
  assert.equal(gesture.wheel(sample(60, 222)).kind, "commit");
});

test("a wheel step in lines switches at once", () => {
  assert.equal(createProfileGesture().wheel(sample(2, 0, { deltaMode: 1 })).kind, "track");
  assert.equal(createProfileGesture().wheel(sample(3, 0, { deltaMode: 1 })).kind, "commit");
});

test("fast wheel clicks switch one at a time, spaced by the transition", () => {
  const gesture = createProfileGesture();
  const steps = [0, 50, 100, 150, 200, 250, 300, 350].map((t) => gesture.wheel(sample(100, t)));
  assert.equal(commits(steps).length, 2);
  assert.equal(steps[0].kind, "commit");
  assert.equal(steps[7].kind, "commit");
});

test("mostly horizontal scroll is ignored", () => {
  assert.equal(createProfileGesture().wheel(sample(10, 0, { deltaX: 40 })).kind, "ignore");
  assert.equal(createProfileGesture().wheel(sample(0, 0)).kind, "ignore");
});

test("a pause restarts the accumulated scroll", () => {
  const gesture = createProfileGesture();
  gesture.wheel(sample(40, 0));
  assert.equal(gesture.wheel(sample(40, QUIET_MS + 1)).kind, "track");
});

test("release restarts the accumulated scroll right away", () => {
  const gesture = createProfileGesture();
  gesture.wheel(sample(40, 0));
  gesture.release();
  assert.equal(gesture.wheel(sample(40, 16)).kind, "track");
});
