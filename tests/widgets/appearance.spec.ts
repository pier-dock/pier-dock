import { test, expect, type Page } from "@playwright/test";
import { expectCall } from "./fixtures";

async function openAppearance(page: Page, responses: Record<string, unknown> = {}) {
  await page.addInitScript(({ responses }) => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: responses, __PIER_TEST_ERRORS__: {} });
  }, { responses });
  await page.goto("/?view=settings");
  await page.getByRole("tab", { name: "Appearance" }).click();
  await page.getByRole("region", { name: "Dock" }).waitFor();
}

test("groups the controls in style, dock and behavior cards", async ({ page }) => {
  await openAppearance(page);
  await expect(page.getByRole("region", { name: "Style" }).getByRole("switch", { name: "Reduce visual effects" })).toBeVisible();
  const dock = page.getByRole("region", { name: "Dock" });
  await expect(dock.getByRole("radiogroup", { name: "Position" })).toBeVisible();
  await expect(dock.getByRole("radiogroup", { name: "Dock width" })).toBeVisible();
  await expect(dock.getByRole("radiogroup", { name: "Panel height" })).toBeVisible();
  const behavior = page.getByRole("region", { name: "Behavior" });
  await expect(behavior.getByRole("radiogroup", { name: "Hide automatically" })).toBeVisible();
  await expect(behavior.getByRole("switch", { name: "Show open apps that aren't pinned" })).toBeVisible();
});

test("the live preview follows the dock position", async ({ page }) => {
  await openAppearance(page);
  const stage = page.locator(".preview-stage");
  await expect(stage).toHaveAttribute("data-pos", "bottom");
  await page.getByRole("region", { name: "Dock" }).getByRole("radio", { name: "Left" }).click();
  await expectCall(page, "save_config", { config: expect.objectContaining({ dockPosition: "left" }) });
  await expect(stage).toHaveAttribute("data-pos", "left");
  await expect(stage).toHaveClass(/vertical/);
});

test("magnification slider only shows while magnify is on", async ({ page }) => {
  await openAppearance(page);
  const behavior = page.getByRole("region", { name: "Behavior" });
  const magnify = behavior.getByRole("switch", { name: "Magnify icons on hover" });
  const slider = behavior.getByRole("slider", { name: "Magnification" });
  const on = (await magnify.getAttribute("aria-checked")) === "true";
  if (on) await magnify.click();
  await expect(slider).toHaveCount(0);
  await magnify.click();
  await expect(slider).toBeVisible();
  await expectCall(page, "save_config", { config: expect.objectContaining({ magnification: 1.6 }) });
});

test("only the solid material is offered when the system has no glass", async ({ page }) => {
  await openAppearance(page, { platform_info: { os: "windows", windowList: "ok", hideNative: true, shortcuts: true, glass: false, followCursor: true, reserveAnyMonitor: true } });
  const material = page.getByRole("region", { name: "Style" });
  const options = material.getByRole("radiogroup", { name: "Material" }).getByRole("radio");
  await expect(options).toHaveCount(1);
  await expect(options.first()).toHaveText("Solid");
  await expect(material.getByRole("slider", { name: "Glass opacity" })).toHaveCount(0);
});
