import { expect, test } from "@playwright/test";
import { expectCall, openMock } from "./fixtures";

test("the group card replaces its members and opens the group panel", async ({ page }) => {
  await openMock(page, "view=dock&groups=1");
  const card = page.getByRole("button", { name: "Group: Volume, Clock, Wi-Fi" });
  await expect(card).toBeVisible();
  await expect(page.locator(".widget-slot > .widget-group")).toHaveCount(1);

  const box = (await card.boundingBox())!;
  expect(box.width).toBeLessThan(70);
  expect(box.height).toBeLessThan(70);

  await card.hover();
  const tooltip = page.locator(".tooltip");
  await expect(tooltip).toContainText("Group: Volume, Clock, Wi-Fi");
  const tip = (await tooltip.boundingBox())!;
  const viewport = page.viewportSize()!;
  expect(tip.x).toBeGreaterThanOrEqual(0);
  expect(tip.x + tip.width).toBeLessThanOrEqual(viewport.width);

  await card.click();
  await expectCall(page, "open_panel", { kind: "group:demo" });
});

test("holding a widget over another creates a group", async ({ page }) => {
  await openMock(page, "view=dock");
  await page.waitForTimeout(600);
  const source = page.locator(".widget-slot").filter({ has: page.getByRole("button", { name: /^Volume/ }) });
  const target = page.locator(".widget-slot").filter({ has: page.locator(".clock-tile") });
  const from = (await source.boundingBox())!;
  const to = (await target.boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 12 });
  await expect(target).toHaveClass(/merge-ready/, { timeout: 2000 });
  await page.mouse.up();
  await expectCall(page, "save_config", {
    config: expect.objectContaining({ widgetGroups: [expect.objectContaining({ members: ["clock", "volume"] })] }),
  });
});

test("the group panel shows every widget and removes one", async ({ page }) => {  await openMock(page, "view=panel&kind=group%3Ademo&groups=1");
  await expect(page.getByRole("heading", { name: "Widget group" })).toBeVisible();
  await expect(page.getByText("3 widgets")).toBeVisible();
  await expect(page.getByRole("button", { name: "Remove Clock from group" })).toBeVisible();
  await page.getByRole("button", { name: "Remove Clock from group" }).click();
  await expectCall(page, "save_config", {
    config: expect.objectContaining({ widgetGroups: [{ id: "demo", members: ["volume", "network"] }] }),
  });
});

test("settings lists a group, dissolves it and creates another", async ({ page }) => {
  await page.addInitScript(() => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: {}, __PIER_TEST_ERRORS__: {} });
  });
  await page.goto("/?view=settings&groups=1");
  await page.getByRole("tab", { name: "Widgets" }).click();

  const groups = page.getByRole("region", { name: "Groups" });
  await expect(groups.getByText("Volume, Clock, Wi-Fi")).toBeVisible();
  await groups.getByRole("button", { name: "Create group" }).click();

  const dialog = page.getByRole("dialog", { name: "Create group" });
  await dialog.getByRole("checkbox", { name: "Bluetooth" }).check();
  await dialog.getByRole("checkbox", { name: "Battery" }).check();
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expectCall(page, "save_config");

  const rows = groups.locator(".widget-group-row");
  await expect(rows).toHaveCount(2);
  await groups.getByRole("button", { name: "Ungroup" }).first().click();
  await expect(rows).toHaveCount(1);
  await groups.getByRole("button", { name: "Ungroup" }).click();
  await expect(rows).toHaveCount(0);
  await expectCall(page, "save_config", { config: expect.objectContaining({ widgetGroups: [] }) });
});
