import { expect, test } from "@playwright/test";
import { openMock } from "./fixtures";

test("sound panel explains dock volume gestures", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await expect(page.getByText("On the dock: scroll to adjust, right-click to mute.")).toBeVisible();
});
