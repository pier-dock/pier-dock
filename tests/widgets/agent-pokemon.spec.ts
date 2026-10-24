import { test, expect, type Page } from "@playwright/test";
import { openMock, expectCall } from "./fixtures";

const running = [{ provider: "claude", sessionId: "walk", status: "running", windowId: 1, project: "test", cwd: "/test", updatedAt: Date.now() }];

async function setMascot(page: Page, id = 6) {
  await page.evaluate(async ({ id }) => {
    const { useConfig } = await import("/src/store/config.ts");
    useConfig.setState({ config: { ...useConfig.getState().config, agentPetMascot: "pokemon", agentPokemonId: id, widgets: [] } });
  }, { id });
}

test("agent mascot shows the chosen pokemon and walks the dock", async ({ page }) => {
  await openMock(page, "view=dock", { get_agent_sessions: running });
  await setMascot(page);
  await expectCall(page, "pokemon_sprite", { id: 6 });
  await expect(page.locator(".agent-pet-pokemon .pokemon-pet-sprite")).toBeVisible();
  await expect(page.locator(".agent-pet-pokemon .claude-pet-facing")).toHaveClass(/facing-left/);
  const first = (await page.locator(".agent-pet-pokemon").boundingBox())!.x;
  await expect.poll(async () => (await page.locator(".agent-pet-pokemon").boundingBox())!.x).not.toBe(first);
});

test("dock tile icon follows the chosen mascot", async ({ page }) => {
  await openMock(page, "view=dock", { get_agent_sessions: running });
  await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    useConfig.setState({ config: { ...useConfig.getState().config, agentPetMascot: "pokemon", agentPokemonId: 6, widgets: [{ kind: "ai-agents", enabled: true, size: "small" }] } });
  });
  await expectCall(page, "pokemon_sprite", { id: 6 });
  await expect(page.locator(".claude-tile .agent-tile-sprite")).toBeVisible();
  await expect(page.locator(".claude-tile svg")).toHaveCount(0);

  await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    useConfig.setState({ config: { ...useConfig.getState().config, agentPetMascot: "clawd" } });
  });
  await expect(page.locator(".claude-tile .agent-tile-sprite")).toHaveCount(0);
  await expect(page.locator(".claude-tile svg")).toBeVisible();
});

test("pokemon hit rect covers the overflowing sprite", async ({ page }) => {
  await openMock(page, "view=dock", { get_agent_sessions: running });
  await setMascot(page);
  await expect(page.locator(".agent-pet-pokemon .pokemon-pet-sprite")).toBeVisible();
  const pet = (await page.locator(".agent-pet-pokemon").boundingBox())!;
  await page.mouse.move(pet.x + pet.width / 2, pet.y + pet.height / 2);
  await expect(page.locator(".agent-pet-pokemon .pokemon-pet-sprite")).not.toHaveClass(/walking/);
  await expect.poll(() => page.evaluate(() => {
    const sprite = document.querySelector(".agent-pet-pokemon .pokemon-pet-sprite")!.getBoundingClientRect();
    const calls = (window as unknown as { __PIER_TEST_CALLS__: { command: string; args: { rect: { x: number; y: number; width: number; height: number } | null } }[] }).__PIER_TEST_CALLS__;
    const hit = calls.filter((c) => c.command === "set_pet_rect" && c.args.rect).at(-1)?.args.rect;
    return !!hit && hit.width === 48 && hit.height === 42 && sprite.left >= hit.x - 1 && sprite.top >= hit.y - 1 && sprite.right <= hit.x + hit.width + 1 && sprite.bottom <= hit.y + hit.height + 1;
  })).toBe(true);
});

test("permission shows the status dot and clicking the sprite overflow opens the sessions", async ({ page }) => {
  await openMock(page, "view=dock", { get_agent_sessions: [{ provider: "claude", sessionId: "perm", status: "permission", windowId: null, project: "test", cwd: "/test", updatedAt: Date.now() }] });
  await setMascot(page);
  await expect(page.locator(".agent-pet-status.claude-dot-permission")).toBeVisible();
  await expect(page.locator(".agent-pet-pokemon .claude-pet-sprite")).toHaveClass(/claude-pet-waiting/);
  await expect(page.locator(".agent-pet-pokemon .claude-pet-sprite")).toHaveCSS("animation-name", "claude-pet-jump-y");
  await page.locator(".agent-pet-pokemon .pokemon-pet-sprite").click({ position: { x: 2, y: 2 } });
  await expectCall(page, "open_panel", { kind: "ai-agents" });
});

test("failed sprite falls back to Clawd and its hit rect", async ({ page }) => {
  await openMock(page, "view=dock", { get_agent_sessions: [{ provider: "claude", sessionId: "done", status: "done", windowId: 1, project: "test", cwd: "/test", updatedAt: Date.now() }] }, { pokemon_sprite: "offline" });
  await setMascot(page);
  await expect(page.locator(".agent-pet-pokemon")).toHaveCount(0);
  await expect(page.locator(".agent-pet-sprite")).toHaveCount(0);
  await expect(page.locator(".claude-pet .claude-sprite svg")).toBeVisible();
  await expect.poll(() => page.evaluate(() => {
    const calls = (window as unknown as { __PIER_TEST_CALLS__: { command: string; args: { rect: { width: number } | null } }[] }).__PIER_TEST_CALLS__;
    return calls.filter((c) => c.command === "set_pet_rect" && c.args.rect).at(-1)?.args.rect?.width;
  })).toBe(34);
});

test("agent pokemon and the decorative pokemon pet coexist", async ({ page }) => {
  await openMock(page, "view=dock", { get_agent_sessions: [{ provider: "claude", sessionId: "idle", status: "done", windowId: 1, project: "test", cwd: "/test", updatedAt: Date.now() }] });
  await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    useConfig.setState({ config: { ...useConfig.getState().config, agentPetMascot: "pokemon", agentPokemonId: 25, pokemonPetEnabled: true, pokemonPetIds: [6], widgets: [] } });
  });
  await expectCall(page, "pokemon_sprite", { id: 25 });
  await expectCall(page, "pokemon_sprite", { id: 6 });
  await expect(page.locator(".agent-pet-pokemon .pokemon-pet-sprite")).toBeVisible();
  await expect(page.locator(".pokemon-pet .pokemon-pet-sprite")).toBeVisible();
});

test("agent mascot is Clawd by default and does not request a sprite", async ({ page }) => {
  await openMock(page, "view=dock");
  await expect(page.locator(".agent-pet-pokemon")).toHaveCount(0);
  await expect(page.locator(".claude-pet")).toHaveCount(1);
  const calls = await page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: { command: string }[] }).__PIER_TEST_CALLS__.map((c) => c.command));
  expect(calls).not.toContain("pokemon_sprite");
});
