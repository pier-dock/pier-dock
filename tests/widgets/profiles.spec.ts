import { expect, test, type Page } from "@playwright/test";
import { expectCall, openMock } from "./fixtures";

async function wheelOver(page: Page, deltaY: number, target = page.locator(".launcher-button").first()) {
  await target.hover();
  await page.mouse.wheel(0, deltaY);
}

const widgetCount = (page: Page) => page.locator(".dock-bar > .widget-slot").count();

test("a wheel click switches to the next profile and saves it", async ({ page }) => {
  await openMock(page, "view=dock&profiles=1");
  await page.waitForTimeout(600);
  const before = await widgetCount(page);
  await wheelOver(page, 120);
  await expectCall(page, "save_config", { config: expect.objectContaining({ activeProfile: "home" }) });
  await expect(page.locator(".dock-profile-hud.visible")).toContainText("Home");
  await expect.poll(() => widgetCount(page)).toBeLessThan(before);
});

test("scrolling past the last profile goes back to the first", async ({ page }) => {
  await openMock(page, "view=dock&profiles=1");
  await page.waitForTimeout(600);
  await wheelOver(page, -120);
  await expect(page.locator(".dock-profile-hud")).toContainText("Focus");
  await page.waitForTimeout(400);
  await wheelOver(page, 120);
  await expect(page.locator(".dock-profile-hud")).toContainText("Main");
});

test("a small scroll shows the next profile and then goes back", async ({ page }) => {
  await openMock(page, "view=dock&profiles=1");
  await page.waitForTimeout(600);
  await wheelOver(page, 20);
  await expect(page.locator(".dock-profile-peek")).toBeVisible();
  await expect(page.locator(".dock-profile-peek")).toHaveCount(0, { timeout: 2000 });
  await expect(page.locator(".dock-profile-hud")).toContainText("Main");
});

test("scrolling over the volume widget does not switch profiles", async ({ page }) => {
  await openMock(page, "view=dock&profiles=1");
  await page.waitForTimeout(600);
  await wheelOver(page, 120, page.getByRole("button", { name: /^Volume/ }));
  await expect(page.locator(".dock-profile-hud")).toContainText("Main");
  await expect(page.locator(".dock-profile-peek")).toHaveCount(0);
});

test("with one profile the scroll does nothing and there is no indicator", async ({ page }) => {
  await openMock(page, "view=dock");
  await page.waitForTimeout(600);
  await wheelOver(page, 120);
  await expect(page.locator(".dock-profile-hud")).toHaveCount(0);
  await expect(page.locator(".dock-profile-peek")).toHaveCount(0);
});

test("an extra dock in the per-monitor mode shows its own profile and switches alone", async ({ page }) => {
  await openMock(page, "view=dock&profiles=1&monitors=1&per-monitor=1&monitor=DELL");
  await page.waitForTimeout(600);
  await wheelOver(page, 120);
  await expectCall(page, "save_config", { config: expect.objectContaining({ monitorProfiles: { DELL: "home" }, activeProfile: "main" }) });
});

test("the settings tab creates a profile and scopes the apps tab", async ({ page }) => {
  await page.addInitScript(() => Object.assign(window, { __PIER_TEST_CALLS__: [] }));
  await page.goto("/?view=settings&profiles=1");
  await page.getByRole("tab", { name: "Profiles" }).click();
  await page.getByRole("button", { name: "New profile" }).click();
  await page.getByPlaceholder("Name this profile").fill("Studies");
  await page.getByRole("button", { name: "Create" }).click();
  await expectCall(page, "save_config", {
    config: expect.objectContaining({ dockProfiles: expect.arrayContaining([expect.objectContaining({ name: "Studies" })]) }),
  });
  await page.getByRole("button", { name: "Edit apps" }).click();
  await expect(page.getByRole("radio", { name: "Studies" })).toHaveAttribute("aria-checked", "true");
  await expect(page.locator(".profile-scope-hint")).toBeVisible();
});

async function barWidthsDuringSwitch(page: Page, deltaY: number) {
  await page.locator(".launcher-button").first().hover();
  await page.waitForTimeout(400);
  return page.evaluate(async (delta) => {
    const bar = document.querySelector<HTMLElement>(".dock-bar")!;
    const widths = [bar.offsetWidth];
    const end = performance.now() + 1100;
    bar.querySelector(".launcher-button")!.dispatchEvent(new WheelEvent("wheel", { deltaY: delta, bubbles: true, cancelable: true }));
    while (performance.now() < end) {
      await new Promise(requestAnimationFrame);
      widths.push(bar.offsetWidth);
    }
    return widths;
  }, deltaY);
}

for (const query of ["view=dock&profiles=1", "view=dock&profiles=1&magnify=1"]) {
  test(`switching to a smaller profile never shrinks past its final size (${query})`, async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 300 });
    await openMock(page, query);
    await page.waitForTimeout(600);
    const widths = await barWidthsDuringSwitch(page, 120);
    const final = widths[widths.length - 1];
    expect(final).toBeLessThan(widths[0]);
    expect(Math.min(...widths)).toBeGreaterThanOrEqual(final - 1);
  });

  test(`switching to a bigger profile never grows past its final size (${query})`, async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 300 });
    await openMock(page, query);
    await page.waitForTimeout(600);
    await barWidthsDuringSwitch(page, 120);
    await page.waitForTimeout(400);
    const widths = await barWidthsDuringSwitch(page, -120);
    const final = widths[widths.length - 1];
    expect(final).toBeGreaterThan(widths[0]);
    expect(Math.max(...widths)).toBeLessThanOrEqual(final + 1);
  });
}
