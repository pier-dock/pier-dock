import { test } from "@playwright/test";
import { openMock } from "./fixtures";

test("preview group list", async ({ page }) => {
  await openMock(page, "view=panel&kind=group%3Ademo&groups=1");
  await page.waitForTimeout(800);
  await page.locator(".group-panel-preview").first().hover();
  await page.screenshot({ path: "group-list-preview.png" });
});
