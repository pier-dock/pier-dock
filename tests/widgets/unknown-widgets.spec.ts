import { expect, test, type Page } from "@playwright/test";
import type { Config } from "../../src/lib/ipc";

type TauriWindow = { __TAURI_INTERNALS__: { invoke: (command: string) => Promise<Config> } };

/** Abre `view` com a config do mock alterada por `change`; os erros de renderização ficam em `errors`. */
async function openWith(page: Page, view: string, change: (config: Config) => unknown) {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(() => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: {}, __PIER_TEST_ERRORS__: {} });
  });
  await page.goto("/?view=settings");
  await page.getByRole("tablist").waitFor();
  const config = await page.evaluate(() => (window as unknown as TauriWindow).__TAURI_INTERNALS__.invoke("get_config"));
  await page.addInitScript((changed) => {
    Object.assign(window, { __PIER_TEST_RESPONSES__: { get_config: changed } });
  }, change(config) as object);
  await page.goto(`/?view=${view}`);
  return errors;
}

/** Config salva por uma versão mais nova: um widget que este front não conhece, ligado e em um grupo. */
const withFutureWidget = (config: Config) => ({
  ...config,
  widgets: [
    ...config.widgets.map((w) => (w.kind === "clock" || w.kind === "volume" ? { ...w, enabled: true } : w)),
    { kind: "future-widget", enabled: true, size: "small" },
  ],
  widgetGroups: [{ id: "g", members: ["future-widget", "clock", "volume"] }],
});

test("the widgets tab skips a widget from a newer version", async ({ page }) => {
  const errors = await openWith(page, "settings", withFutureWidget);
  await page.getByRole("tab", { name: "Widgets", exact: true }).click();
  await expect(page.locator('.widget-card[data-kind="clock"]')).toBeVisible();
  await expect(page.locator('.widget-card[data-kind="future-widget"]')).toHaveCount(0);
  await expect(page.locator(".widget-group-row-name")).toHaveText(/^(Clock, Volume|Volume, Clock)$/);
  expect(errors).toEqual([]);
});

test("the dock skips a widget from a newer version", async ({ page }) => {
  const errors = await openWith(page, "dock", withFutureWidget);
  await expect(page.locator(".dock-bar")).toBeVisible();
  await expect(page.locator(".widget-slot > .widget-group")).toHaveCount(1);
  expect(errors).toEqual([]);
});
