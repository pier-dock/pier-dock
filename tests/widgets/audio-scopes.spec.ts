import { expect, test } from "@playwright/test";
import { openMock } from "./fixtures";

test("the panel names the system and application output scopes", async ({ page }) => {
  await page.setViewportSize({ width: 376, height: 536 });
  await openMock(page, "view=panel&kind=volume");
  await page.addStyleTag({ content: 'html[data-mock][data-view="panel"] #root { position: static; transform: none; width: 100%; height: 100%; }' });
  await expect(page.getByRole("radiogroup", { name: "Default output" })).toBeVisible();
  await expect(page.getByText("App audio", { exact: true })).toBeVisible();
  await expect(page.getByText("Choose a separate output for each app.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Spotify output", exact: true })).toBeVisible();
  const body = (await page.locator(".sound-body").boundingBox())!;
  const lastApp = (await page.locator(".sound-app").last().boundingBox())!;
  expect(lastApp.y + lastApp.height).toBeLessThanOrEqual(body.y + body.height);
});

test("tabbing away from output choices closes the list", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await page.getByRole("button", { name: "Spotify output", exact: true }).click();
  await expect(page.getByRole("listbox")).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("listbox")).toHaveCount(0);
});
