import { test } from "node:test";
import assert from "node:assert/strict";
import { addToGroup, createGroup, createGroupFromKinds, dissolveGroup, newGroupId, removeFromGroup } from "./groups.ts";

const app = (id: string) => ({ id, name: id, target: `/Applications/${id}.app` });
const widget = (kind: string, enabled = true) => ({ kind, enabled, size: "small" }) as const;

const base = {
  dockOrder: ["start", "widget:media", "widget:clock", "widget:volume", "app:finder", "app:safari"],
  widgetGroups: [] as { id: string; members: string[] }[],
  pinnedApps: [app("finder"), app("safari")],
  widgets: [widget("media"), widget("clock"), widget("volume"), widget("network"), widget("battery", false)],
  showStartButton: true,
  showLauncher: false,
};

test("createGroup tira os dois widgets da dock e põe o cartão no lugar do alvo", () => {
  const patch = createGroup(base, "widget:volume", "widget:media");
  assert.ok(patch?.widgetGroups?.length === 1);
  const group = patch!.widgetGroups![0];
  assert.deepEqual(group.members, ["media", "volume"]);
  assert.ok(patch!.dockOrder!.includes(`group:${group.id}`));
  assert.ok(!patch!.dockOrder!.includes("widget:media"));
  assert.ok(!patch!.dockOrder!.includes("widget:volume"));
});

test("addToGroup acrescenta um widget solto ao grupo", () => {
  const first = createGroup(base, "widget:volume", "widget:media")!;
  const config = { ...base, ...first };
  const groupId = first.widgetGroups![0].id;
  const patch = addToGroup(config, groupId, "clock");
  assert.deepEqual(patch?.widgetGroups![0].members, ["media", "volume", "clock"]);
  assert.ok(!patch!.dockOrder!.includes("widget:clock"));
});

test("addToGroup move o widget do grupo anterior e desfaz o grupo que ficou com um membro", () => {
  const first = createGroup(base, "widget:volume", "widget:media")!;
  const second = createGroup({ ...base, ...first }, "widget:network", "widget:clock")!;
  const config = { ...base, ...second };
  const firstId = second.widgetGroups![0].id;
  const secondId = second.widgetGroups![1].id;
  const patch = addToGroup(config, firstId, "network")!;
  assert.deepEqual(patch.widgetGroups!.find((g) => g.id === firstId)?.members, ["media", "volume", "network"]);
  assert.deepEqual(patch.widgetGroups!.find((g) => g.id === secondId), { id: secondId, members: ["clock"] });
  assert.ok(!patch.dockOrder!.includes(`group:${secondId}`));
  assert.ok(patch.dockOrder!.includes("widget:clock"));
});

test("removeFromGroup mantém o grupo enquanto sobram dois membros ligados", () => {
  const first = createGroup(base, "widget:volume", "widget:media")!;
  const config = { ...base, ...first };
  const groupId = first.widgetGroups![0].id;
  const withClock = addToGroup(config, groupId, "clock")!;
  const patch = removeFromGroup({ ...config, ...withClock }, groupId, "volume")!;
  assert.deepEqual(patch.widgetGroups![0].members, ["media", "clock"]);
  assert.ok(patch.dockOrder!.includes(`group:${groupId}`));
  assert.ok(patch.dockOrder!.includes("widget:volume"));
});

test("removeFromGroup desfaz o grupo quando sobra um membro ligado", () => {
  const first = createGroup(base, "widget:volume", "widget:media")!;
  const config = { ...base, ...first };
  const groupId = first.widgetGroups![0].id;
  const patch = removeFromGroup(config, groupId, "volume")!;
  assert.deepEqual(patch.widgetGroups![0], { id: groupId, members: ["media"] });
  assert.ok(!patch.dockOrder!.includes(`group:${groupId}`));
  assert.ok(patch.dockOrder!.includes("widget:media"));
  assert.ok(patch.dockOrder!.includes("widget:volume"));
});

test("dissolveGroup devolve os membros no lugar do cartão", () => {
  const first = createGroup(base, "widget:volume", "widget:media")!;
  const config = { ...base, ...first };
  const groupId = first.widgetGroups![0].id;
  const patch = dissolveGroup(config, groupId);
  assert.deepEqual(patch.widgetGroups, []);
  assert.ok(patch.dockOrder!.indexOf("widget:media") < patch.dockOrder!.indexOf("widget:volume"));
  assert.ok(patch.dockOrder!.indexOf("widget:volume") < patch.dockOrder!.indexOf("widget:clock"));
});

test("createGroupFromKinds cria um grupo no lugar do primeiro escolhido", () => {
  const patch = createGroupFromKinds(base, ["clock", "volume"]);
  assert.ok(patch?.widgetGroups?.length === 1);
  const group = patch!.widgetGroups![0];
  assert.deepEqual(group.members, ["clock", "volume"]);
  assert.ok(patch!.dockOrder!.includes(`group:${group.id}`));
  assert.ok(!patch!.dockOrder!.includes("widget:clock"));
  assert.ok(!patch!.dockOrder!.includes("widget:volume"));
  assert.equal(createGroupFromKinds(base, ["clock"]), null);
});

test("createGroupFromKinds ignora widgets que já estão em um grupo", () => {
  const first = createGroup(base, "widget:volume", "widget:media")!;
  const config = { ...base, ...first };
  assert.equal(createGroupFromKinds(config, ["media", "clock"]), null);
});

test("newGroupId gera ids diferentes", () => {
  assert.notEqual(newGroupId(), newGroupId());
});
