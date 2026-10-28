import { expect, test } from "@playwright/test";
import { openMock } from "./fixtures";

test("GitHub counters name their purpose and network announces its function", async ({ page }) => {
  await openMock(page, "view=dock");
  await expect(page.locator('button[aria-label="Your open pull requests: 0"]')).toContainText("PRs");
  await expect(page.locator('button[aria-label="Pending reviews: 12"]')).toContainText("Reviews");
  await expect(page.locator('button[aria-label="Network · Casa"]')).toBeVisible();
});
