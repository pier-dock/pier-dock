import { expect, test } from "@playwright/test";
import { openMock, expectCall } from "./fixtures";

test("per-app volume can be adjusted from the larger slider target", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  const slider = page.getByRole("slider", { name: "Spotify volume" });
  await slider.scrollIntoViewIfNeeded();
  const box = (await slider.boundingBox())!;
  expect(box.height).toBeGreaterThanOrEqual(24);
  await slider.click({ position: { x: box.width / 2, y: 2 } });
  await expectCall(page, "set_app_volume", { id: "1", level: expect.any(Number) });
});

test("switching app pickers stays reliable and tabbing away dismisses choices", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  const spotify = page.getByRole("button", { name: "Spotify output", exact: true });
  const firefox = page.getByRole("button", { name: "Firefox output", exact: true });
  for (let i = 0; i < 3; i++) {
    await spotify.click();
    await expect(page.getByRole("listbox", { name: "Spotify output" })).toBeVisible();
    await firefox.click();
    await expect(page.getByRole("listbox", { name: "Firefox output" })).toBeVisible();
    await page.keyboard.press("Escape");
  }
  await spotify.click();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("listbox")).toHaveCount(0);
});
