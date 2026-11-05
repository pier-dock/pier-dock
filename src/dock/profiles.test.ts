import assert from "node:assert/strict";
import test from "node:test";
import type { Config, DockProfile } from "../lib/ipc";
import {
  addEmptyProfile,
  anyProfileWidget,
  duplicateProfile,
  moveProfile,
  neighborProfile,
  patchForProfile,
  profileOfGroup,
  profileView,
  removeProfile,
  renameProfile,
  scopedUpdate,
  setWindowProfile,
  switchProfile,
  windowProfileId,
} from "./profiles.ts";

const profile = (id: string, extra: Partial<DockProfile> = {}): DockProfile => ({
  id,
  name: id,
  icon: "layers",
  accent: "coral",
  pinnedApps: [],
  widgets: [],
  dockOrder: [],
  widgetGroups: [],
  showStartButton: true,
  showLauncher: false,
  ...extra,
});

const app = (id: string) => ({ id, name: id, target: id });

function config(extra: Partial<Config> = {}): Config {
  return {
    accent: "coral",
    pinnedApps: [app("a")],
    widgets: [],
    dockOrder: ["app:a"],
    widgetGroups: [],
    showStartButton: true,
    showLauncher: false,
    activeProfile: "work",
    dockProfiles: [
      profile("work", { pinnedApps: [app("stale")] }),
      profile("home", { accent: "green", pinnedApps: [app("b")], dockOrder: ["app:b"] }),
      profile("focus"),
    ],
    monitorProfiles: {},
    profilePerMonitor: false,
    dockMonitor: "primary",
    glassOpacity: 0.4,
    ...extra,
  } as Config;
}

test("a view shows the content of the profile and keeps the shared settings", () => {
  const view = profileView(config(), "home");
  assert.deepEqual(view.pinnedApps, [app("b")]);
  assert.equal(view.accent, "green");
  assert.equal(view.glassOpacity, 0.4);
});

test("the active profile is read from the top level, which is the fresh copy", () => {
  const c = config();
  assert.equal(profileView(c, "work"), c);
});

test("a patch to another profile stays in that profile, and shared settings stay global", () => {
  const patch = patchForProfile(config(), "home", { pinnedApps: [app("c")], glassOpacity: 0.9 });
  assert.equal(patch.glassOpacity, 0.9);
  assert.equal(patch.pinnedApps, undefined);
  assert.deepEqual(patch.dockProfiles!.find((p) => p.id === "home")!.pinnedApps, [app("c")]);
  assert.deepEqual(patch.dockProfiles!.find((p) => p.id === "work")!.pinnedApps, [app("stale")]);
});

test("a patch to the active profile goes to the top level", () => {
  const patch = { pinnedApps: [app("c")] };
  assert.equal(patchForProfile(config(), "work", patch), patch);
});

test("a scoped update computes the patch from the view of its profile", () => {
  let result: Partial<Config> = {};
  const update = scopedUpdate((patch: (c: Config) => Partial<Config>) => {
    result = patch(config());
  }, "home");
  update((c) => ({ pinnedApps: [...c.pinnedApps, app("c")] }));
  assert.deepEqual(result.dockProfiles!.find((p) => p.id === "home")!.pinnedApps, [app("b"), app("c")]);
});

test("switching saves the outgoing profile and loads the incoming one", () => {
  const patch = switchProfile(config(), "home");
  assert.equal(patch.activeProfile, "home");
  assert.deepEqual(patch.pinnedApps, [app("b")]);
  assert.equal(patch.accent, "green");
  assert.deepEqual(patch.dockProfiles!.find((p) => p.id === "work")!.pinnedApps, [app("a")]);
});

test("switching to the current or an unknown profile changes nothing", () => {
  assert.deepEqual(switchProfile(config(), "work"), {});
  assert.deepEqual(switchProfile(config(), "gone"), {});
});

test("neighbors wrap around in both directions", () => {
  const { dockProfiles } = config();
  assert.equal(neighborProfile(dockProfiles, "work", 1)!.id, "home");
  assert.equal(neighborProfile(dockProfiles, "focus", 1)!.id, "work");
  assert.equal(neighborProfile(dockProfiles, "work", -1)!.id, "focus");
});

test("a single profile has no neighbor", () => {
  assert.equal(neighborProfile([profile("only")], "only", 1), null);
});

test("an extra dock only has its own profile in the per-monitor mode with docks on all monitors", () => {
  const monitorProfiles = { m2: "home" };
  assert.equal(windowProfileId(config({ monitorProfiles }), "m2"), "work");
  assert.equal(windowProfileId(config({ monitorProfiles, profilePerMonitor: true }), "m2"), "work");
  const each = config({ monitorProfiles, profilePerMonitor: true, dockMonitor: "all" });
  assert.equal(windowProfileId(each, "m2"), "home");
  assert.equal(windowProfileId(each, null), "work");
  assert.equal(windowProfileId(each, "m3"), "work");
});

test("a monitor linked to a removed profile uses the active one", () => {
  const c = config({ monitorProfiles: { m2: "gone" }, profilePerMonitor: true, dockMonitor: "all" });
  assert.equal(windowProfileId(c, "m2"), "work");
});

test("in the per-monitor mode an extra dock switches alone", () => {
  const each = config({ profilePerMonitor: true, dockMonitor: "all" });
  assert.deepEqual(setWindowProfile(each, "m2", "home"), { monitorProfiles: { m2: "home" } });
  assert.equal(setWindowProfile(each, null, "home").activeProfile, "home");
  assert.equal(setWindowProfile(config(), "m2", "home").activeProfile, "home");
});

test("an empty profile has no apps or widgets but keeps a button in the bar", () => {
  const c = config({ widgets: [{ kind: "clock", enabled: true, size: "small" }] });
  const { id, patch } = addEmptyProfile(c, "New");
  const created = patch.dockProfiles!.find((p) => p.id === id)!;
  assert.equal(created.name, "New");
  assert.deepEqual(created.pinnedApps, []);
  assert.deepEqual(created.widgets, [{ kind: "clock", enabled: false, size: "small" }]);
  assert.equal(created.showStartButton, true);
});

test("a duplicate copies the fresh content and gets its own group ids", () => {
  const c = config({
    widgetGroups: [{ id: "g", members: ["clock", "volume"] }],
    dockOrder: ["group:g", "app:a"],
  });
  const { id, patch } = duplicateProfile(c, "work", "Copy")!;
  const ids = patch.dockProfiles!.map((p) => p.id);
  assert.equal(ids[1], id);
  const copy = patch.dockProfiles![1];
  assert.deepEqual(copy.pinnedApps, [app("a")]);
  assert.notEqual(copy.widgetGroups[0].id, "g");
  assert.deepEqual(copy.widgetGroups[0].members, ["clock", "volume"]);
  assert.deepEqual(copy.dockOrder, [`group:${copy.widgetGroups[0].id}`, "app:a"]);
});

test("removing the active profile loads its neighbor and drops monitor links", () => {
  const patch = removeProfile(config({ monitorProfiles: { m1: "work", m2: "home" } }), "work")!;
  assert.deepEqual(patch.dockProfiles!.map((p) => p.id), ["home", "focus"]);
  assert.equal(patch.activeProfile, "home");
  assert.deepEqual(patch.pinnedApps, [app("b")]);
  assert.deepEqual(patch.monitorProfiles, { m2: "home" });
});

test("removing another profile keeps the active one", () => {
  const patch = removeProfile(config(), "focus")!;
  assert.deepEqual(patch.dockProfiles!.map((p) => p.id), ["work", "home"]);
  assert.equal(patch.activeProfile, undefined);
  assert.equal(patch.pinnedApps, undefined);
});

test("the last profile cannot be removed", () => {
  assert.equal(removeProfile(config({ dockProfiles: [profile("work")] }), "work"), null);
});

test("profiles move inside the list bounds", () => {
  assert.deepEqual(moveProfile(config(), "home", -1)!.dockProfiles!.map((p) => p.id), ["home", "work", "focus"]);
  assert.equal(moveProfile(config(), "work", -1), null);
  assert.equal(moveProfile(config(), "focus", 1), null);
});

test("renaming does not touch the content", () => {
  const renamed = renameProfile(config(), "home", { name: "Casa" }).dockProfiles!.find((p) => p.id === "home")!;
  assert.equal(renamed.name, "Casa");
  assert.deepEqual(renamed.pinnedApps, [app("b")]);
});

test("a widget enabled in any profile counts for polling", () => {
  const c = config({
    dockProfiles: [profile("work"), profile("home", { widgets: [{ kind: "weather", enabled: true, size: "wide" }] })],
  });
  assert.equal(anyProfileWidget(c, (kind) => kind === "weather"), true);
  assert.equal(anyProfileWidget(c, (kind) => kind === "clock"), false);
});

test("a group panel finds the profile that owns the group", () => {
  const c = config({
    widgetGroups: [{ id: "active", members: [] }],
    dockProfiles: [profile("work"), profile("home", { widgetGroups: [{ id: "other", members: [] }] })],
  });
  assert.equal(profileOfGroup(c, "active"), "work");
  assert.equal(profileOfGroup(c, "other"), "home");
  assert.equal(profileOfGroup(c, "gone"), "work");
});
