import { test, expect } from "@playwright/test";
import { openMock, expectCall } from "./fixtures";

// O widget só existe no Windows: o WebView do teste precisa se identificar como tal.
test.use({
  userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.0.0",
});

test("clicking the tile turns Do Not Disturb on and the tile follows", async ({ page }) => {
  await openMock(page, "view=dock&new");
  await page.locator('button[aria-label="Do Not Disturb · off"]').click();
  await expectCall(page, "set_do_not_disturb", { enabled: true });
  await expect(page.locator('button[aria-label="Do Not Disturb · on"]')).toBeVisible();
});

test("clicking again turns it back off", async ({ page }) => {
  await openMock(page, "view=dock&new");
  await page.locator('button[aria-label="Do Not Disturb · off"]').click();
  await page.locator('button[aria-label="Do Not Disturb · on"]').click();
  await expectCall(page, "set_do_not_disturb", { enabled: false });
  await expect(page.locator('button[aria-label="Do Not Disturb · off"]')).toBeVisible();
});

test("without system access the tile opens the notification settings instead", async ({ page }) => {
  await openMock(page, "view=dock&new", { do_not_disturb_status: { available: false, enabled: false } });
  await page.locator('button[aria-label="Do Not Disturb · off"]').click();
  await expectCall(page, "open_shell", { kind: "do-not-disturb" });
});
