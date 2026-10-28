import { test, expect } from "@playwright/test";
import { openMock, expectCall } from "./fixtures";

const picker = (page: import("@playwright/test").Page, name = "Spotify") => page.getByRole("button", { name: `${name} output`, exact: true });

test("click opens choices inside the panel, survives polling, and routes only the chosen app", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await expect(page.locator(".sound-app-icon img")).toHaveCount(2);
  await expectCall(page, "get_icon", { key: "/Applications/Spotify.app" });
  await expect(page.locator(".sound-app select")).toHaveCount(0);
  await picker(page).click();
  const choices = page.getByRole("listbox", { name: "Spotify output", exact: true });
  await expect(choices).toBeVisible();
  await expect(choices.getByRole("option", { name: "MacBook Air Speakers" })).toHaveAttribute("aria-selected", "true");
  await expectCall(page, "focus_panel");
  // Wait for a fresh native audio-status response while the picker is open.
  const before = await page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: {command: string}[] }).__PIER_TEST_CALLS__.filter(c => c.command === "audio_status").length);
  await expect.poll(() => page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: {command: string}[] }).__PIER_TEST_CALLS__.filter(c => c.command === "audio_status").length)).toBeGreaterThan(before);
  await expect(choices).toBeVisible();
  await choices.getByRole("option", { name: "WH-1000XM4" }).click();
  await expectCall(page, "set_app_output", { id: "1", output: "headphones" });
  await expect(picker(page)).toContainText("WH-1000XM4");
  await expect(picker(page, "Firefox")).toContainText("MacBook Air Speakers");
  await expect(choices).toHaveCount(0);
  await expect(page.locator(".popover")).toBeVisible();
});

test("failed per-app route keeps the previous selection and allows retry", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume", {}, { set_app_output: "output disconnected" });
  await picker(page).click();
  await page.getByRole("option", { name: "WH-1000XM4" }).click();
  await expect(page.getByRole("alert")).toContainText("Could not switch");
  await expect(picker(page)).toContainText("MacBook Air Speakers");
  await picker(page).click();
  await expect(page.getByRole("listbox")).toBeVisible();
});

test("keyboard navigation selects an output without triggering panel footer shortcuts", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await picker(page).focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("option", { name: "MacBook Air Speakers" })).toBeFocused();
  await page.keyboard.press("End");
  await expect(page.getByRole("option", { name: "WH-1000XM4" })).toBeFocused();
  await page.keyboard.press("Home");
  await page.keyboard.press("ArrowUp");
  await expect(page.getByRole("option", { name: "WH-1000XM4" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expectCall(page, "set_app_output", { id: "1", output: "headphones" });
  await expect(picker(page)).toContainText("WH-1000XM4");
  const calls = await page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: {command: string}[] }).__PIER_TEST_CALLS__);
  expect(calls.some(c => c.command === "open_shell")).toBe(false);
});

test("Escape and outside clicks dismiss choices while preserving normal panel dismissal", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await picker(page).click();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("listbox")).toHaveCount(0);
  await expect(picker(page)).toBeFocused();
  const closed = () => page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: {command: string}[] }).__PIER_TEST_CALLS__.filter(c => c.command === "close_panel").length);
  expect(await closed()).toBe(0);
  await picker(page).click();
  await page.locator(".popover-heading").click();
  await expect(page.getByRole("listbox")).toHaveCount(0);
  expect(await closed()).toBe(0);
  await page.keyboard.press("Escape");
  await expectCall(page, "close_panel");
});

test("opening another app's picker dismisses the previous one", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await picker(page).click();
  await picker(page, "Firefox").click();
  await expect(page.getByRole("listbox")).toHaveCount(1);
  await expect(page.getByRole("listbox", { name: "Firefox output", exact: true })).toBeVisible();
});

test("device removal while open closes empty choices and disables routing", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await picker(page).click();
  await expect(page.getByRole("listbox")).toBeVisible();
  await page.evaluate(() => {
    const fixture = window as unknown as { __PIER_TEST_RESPONSES__: Record<string, unknown> };
    fixture.__PIER_TEST_RESPONSES__.audio_status = {
      outputs: [], canRouteApps: true, appsNeedPermission: false,
      apps: [{ id: "1", name: "Spotify", icon: null, level: .8, muted: false, outputId: null }],
    };
  });
  await expect(picker(page)).toBeDisabled();
  await expect(page.getByRole("listbox")).toHaveCount(0);
  await expect(page.locator(".popover")).toBeVisible();
});

test("output choices fit inside the native panel dimensions", async ({ page }) => {
  await page.setViewportSize({ width: 376, height: 536 });
  await openMock(page, "view=panel&kind=volume");
  // The browser canvas normally offsets the panel from the screen edge; native windows do not.
  await page.addStyleTag({ content: 'html[data-mock][data-view="panel"] #root { position: static; transform: none; width: 100%; height: 100%; }' });
  await picker(page).click();
  const options = page.getByRole("listbox", { name: "Spotify output", exact: true });
  await expect(options).toBeVisible();
  const box = (await options.boundingBox())!;
  const body = (await page.locator(".sound-body").boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(18);
  expect(box.x + box.width).toBeLessThanOrEqual(358);
  expect(box.y).toBeGreaterThanOrEqual(body.y);
  expect(box.y + box.height).toBeLessThanOrEqual(body.y + body.height);
  await page.screenshot({ path: test.info().outputPath("audio-picker.png") });
});
