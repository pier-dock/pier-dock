import { test, expect } from "@playwright/test";
import { expectCall } from "./fixtures";

type Fixtures = { __PIER_TEST_ERRORS__: Record<string, string> };

async function openSettings(page: import("@playwright/test").Page, errors: Record<string, string>) {
  await page.addInitScript(({ errors }) => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: {}, __PIER_TEST_ERRORS__: errors });
  }, { errors });
  await page.goto("/?view=settings");
}

test("a config that fails to load shows an error and recovers on retry", async ({ page }) => {
  await openSettings(page, { get_config: "permission denied" });
  await expect(page.getByRole("alert")).toContainText("Couldn't load Pier's settings.");
  await page.evaluate(() => { (window as unknown as Fixtures).__PIER_TEST_ERRORS__ = {}; });
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(page.getByRole("tablist")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("a failed save puts the previous value back", async ({ page }) => {
  await openSettings(page, { save_config: "disk full" });
  await page.getByRole("tab", { name: "General", exact: true }).click();
  const autoUpdate = page.getByRole("switch", { name: "Check for updates automatically", exact: true });
  await expect(autoUpdate).toHaveAttribute("aria-checked", "true");
  await autoUpdate.click();
  await expectCall(page, "save_config", { config: expect.objectContaining({ autoUpdate: false }) });
  await expect(autoUpdate).toHaveAttribute("aria-checked", "true");
});

test("a tab that fails to render leaves the other tabs reachable", async ({ page }) => {
  // Sem `battery`, a prévia do widget de bateria lança ao renderizar.
  await page.addInitScript(() => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: { system_status: {} }, __PIER_TEST_ERRORS__: {} });
  });
  await page.goto("/?view=settings");
  await page.getByRole("tab", { name: "Widgets", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("This section couldn't be shown.");

  await page.getByRole("tab", { name: "Appearance", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);

  await page.getByRole("tab", { name: "Widgets", exact: true }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  await page.evaluate(() => { (window as unknown as { __PIER_TEST_RESPONSES__: object }).__PIER_TEST_RESPONSES__ = {}; });
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(page.locator(".widget-card").first()).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
});
