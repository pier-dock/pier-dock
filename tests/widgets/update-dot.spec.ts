import { test, expect } from "@playwright/test";
import { expectCall, openMock } from "./fixtures";

const platformOk = { os: "macos", windowList: "ok", hideNative: true, shortcuts: true, glass: true, followCursor: true, reserveAnyMonitor: true };
const pending = { pending_update: { version: "99.0.0", notes: null }, platform_info: platformOk };

test("a found update shows a dot that opens the General settings without installing", async ({ page }) => {
  await openMock(page, "view=dock", pending);
  const dot = page.getByRole("button", { name: "Update to 99.0.0 available", exact: true });
  await expect(dot).toBeVisible();
  await dot.click();
  await expectCall(page, "open_settings", { tab: "general" });
  const commands = await page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: { command: string }[] }).__PIER_TEST_CALLS__.map((c) => c.command));
  expect(commands).not.toContain("install_update");
});

test("no dot without a pending update", async ({ page }) => {
  await openMock(page, "view=dock", { platform_info: platformOk });
  await expect(page.locator(".update-dot")).toHaveCount(0);
});

test("settings opened for an update start on the General tab", async ({ page }) => {
  await page.goto("/?view=settings&kind=general");
  await expect(page.getByRole("tab", { name: "General", exact: true })).toHaveAttribute("aria-selected", "true");
});

test("pending setup takes the dot and opens settings; the update stays in the menu", async ({ page }) => {
  await openMock(page, "view=dock", { pending_update: { version: "99.0.0", notes: null } });
  const dot = page.locator(".attention-dot");
  await expect(dot).toHaveAttribute("data-tone", "setup");
  await expect(dot).toHaveAccessibleName("Finish setting up Pier");
  await dot.focus();
  await expect(dot.locator(".tooltip")).toBeVisible();
  await dot.click();
  await expectCall(page, "open_settings");
});
