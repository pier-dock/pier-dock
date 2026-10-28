import { expect, test } from "@playwright/test";
import { expectCall, openMock } from "./fixtures";

test("sortable widgets expose one control and dividers have a name", async ({ page }) => {
  await openMock(page, "view=dock");
  await expect(page.locator('.widget-slot[role="button"]')).toHaveCount(0);
  await expect(page.locator('.divider-item[role="button"]')).toHaveCount(0);
  const dividers = page.getByRole("button", { name: "Divider", exact: true });
  await expect(dividers).not.toHaveCount(0);
  await dividers.first().focus();
  await expect(dividers.first()).toBeFocused();
});

test("a named divider remains draggable with the pointer", async ({ page }) => {
  await openMock(page, "view=dock");
  const divider = page.getByRole("button", { name: "Divider", exact: true }).first();
  const target = page.locator(".app-button").nth(1);
  const from = (await divider.boundingBox())!;
  const to = (await target.boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 12 });
  await page.mouse.up();
  await expectCall(page, "save_config");
});

test("a divider opens its menu from the keyboard", async ({ page }) => {
  await openMock(page, "view=dock", { "plugin:menu|new": [1, "divider-menu"] });
  await page.getByRole("button", { name: "Divider", exact: true }).first().focus();
  await page.keyboard.press("Enter");
  await expectCall(page, "plugin:menu|popup");
});
