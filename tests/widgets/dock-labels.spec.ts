import { expect, test } from "@playwright/test";
import { openMock } from "./fixtures";

test("GitHub counters show an icon by default and network announces its function", async ({ page }) => {
  await openMock(page, "view=dock");
  const mine = page.locator('button[aria-label="Your open pull requests: 0"]');
  const review = page.locator('button[aria-label="Pending reviews: 12"]');
  await expect(mine.locator("svg")).toBeVisible();
  await expect(review.locator("svg")).toBeVisible();
  await expect(mine).not.toContainText("PRs");
  await expect(review).not.toContainText("Reviews");
  await expect(page.locator('button[aria-label="Network · Casa"]')).toBeVisible();
});

test("GitHub counters can show the count with a text label instead of the icon", async ({ page }) => {
  await openMock(page, "view=dock");
  await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    const { config } = useConfig.getState();
    useConfig.setState({
      config: {
        ...config,
        widgets: config.widgets.map((w) => (w.kind === "github-mine" || w.kind === "github-review" ? { ...w, size: "count" } : w)),
      },
    });
  });
  const mine = page.locator('button[aria-label="Your open pull requests: 0"]');
  const review = page.locator('button[aria-label="Pending reviews: 12"]');
  await expect(mine).toContainText("PRs");
  await expect(review).toContainText("Reviews");
  await expect(mine.locator("svg")).toHaveCount(0);
});
