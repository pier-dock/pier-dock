import { test, expect } from "@playwright/test";
import { openMock, expectCall } from "./fixtures";

for (const [panel, kind] of [["bluetooth", "bluetooth"], ["volume", "sound"], ["network", "network"]]) {
  test(`${panel} settings button sends correct native destination`, async ({ page }) => {
    await openMock(page, `view=panel&kind=${panel}`);
    await page.locator(".action-footer button").click();
    await expectCall(page, "open_shell", { kind });
  });
}

test("settings launch failure is visible and retry works", async ({ page }) => {
  await openMock(page, "view=panel&kind=bluetooth", {}, { open_shell: "No settings application installed" });
  await page.locator(".action-footer button").click();
  await expect(page.getByRole("alert")).toContainText("Try again");
  await page.evaluate(() => { (window as unknown as { __PIER_TEST_ERRORS__: object }).__PIER_TEST_ERRORS__ = {}; });
  await page.locator(".action-footer button").click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expectCall(page, "open_shell", { kind: "bluetooth" });
});

test("Enter opens Bluetooth settings without stealing focused button activation", async ({ page }) => {
  await openMock(page, "view=panel&kind=bluetooth");
  await page.locator("body").click({ position: { x: 2, y: 2 } });
  await page.keyboard.press("Enter");
  await expectCall(page, "open_shell", { kind: "bluetooth" });
  await page.evaluate(() => { (window as unknown as { __PIER_TEST_CALLS__: unknown[] }).__PIER_TEST_CALLS__ = []; });
  await page.getByRole("switch", { name: "Bluetooth" }).focus();
  await page.keyboard.press("Enter");
  await expectCall(page, "set_bluetooth", { powered: false });
  await expect.poll(() => page.evaluate(() => (window as unknown as { __PIER_TEST_CALLS__: {command: string}[] }).__PIER_TEST_CALLS__.filter(c => c.command === "open_shell").length)).toBe(0);
});

test("sound output, master mute, app mute and sliders send the selected control", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume");
  await page.getByRole("radio", { name: "WH-1000XM4" }).click();
  await expectCall(page, "set_output_device", { id: "headphones" });
  await page.locator(".sound-master button").click();
  await expectCall(page, "set_volume", { muted: true });
  await page.locator(".sound-app").filter({ hasText: "Spotify" }).locator(".sound-mute").click();
  await expectCall(page, "set_app_volume", { id: "1", muted: true });
  await page.getByRole("slider").first().focus();
  await page.keyboard.press("ArrowRight");
  await expectCall(page, "set_volume", { muted: false });
});

test("Wi-Fi connect, disconnect, toggle, password submit and cancel", async ({ page }) => {
  await openMock(page, "view=panel&kind=network");
  await page.locator(".wifi-disconnect").click();
  await expectCall(page, "wifi_disconnect");
  await expect(page.locator(".wifi-disconnect")).toBeEnabled();
  await page.getByRole("button", { name: /Escritório/ }).click();
  await expectCall(page, "wifi_connect", { ssid: "Escritório", password: null });
  await expect(page.getByRole("button", { name: /Escritório/ })).toBeEnabled();
  await page.getByRole("button", { name: /VIVO-8F21/ }).click();
  await page.locator("input[type=password]").fill("test-password");
  await page.locator(".wifi-password button[type=submit]").click();
  await expectCall(page, "wifi_connect", { ssid: "VIVO-8F21", password: "test-password" });
  await expect(page.locator(".wifi-password")).toHaveCount(0);
  await page.getByRole("button", { name: /VIVO-8F21/ }).click();
  await page.locator(".wifi-password button[type=button]").click();
  await expect(page.locator(".wifi-password")).toHaveCount(0);
  await page.getByRole("switch", { name: "Wi-Fi" }).click();
  await expectCall(page, "wifi_set_enabled", { enabled: false });
});

test("sound permission button opens the native audio-capture privacy page", async ({ page }) => {
  await openMock(page, "view=panel&kind=volume&audio-permission");
  await page.locator(".sound-permission button").click();
  await expectCall(page, "open_shell", { kind: "audio-capture" });
});

test("Wi-Fi permission button requests permission", async ({ page }) => {
  await openMock(page, "view=panel&kind=network", { wifi_status: { available: true, enabled: true, canToggle: false, canConnect: false, needsPermission: true, networks: [] } });
  await page.locator(".wifi-empty-action").click();
  await expectCall(page, "wifi_request_permission");
});

for (const kind of ["github-mine", "github-commits", "github-contrib", "market"]) {
  test(`${kind} refresh shows it is loading even with an instant response`, async ({ page }) => {
    await openMock(page, `view=panel&kind=${kind}`);
    await page.locator(".action", { hasText: "Refresh" }).click();
    const loading = page.locator(".action.loading");
    await expect(loading).toContainText("Refreshing…");
    await expect(loading).toHaveAttribute("aria-busy", "true");
    await expect(loading.locator(".action-spinner")).toBeVisible();
    await expect(loading).toHaveCount(0);
    await expect(page.locator(".action", { hasText: "Refresh" })).toBeEnabled();
  });
}

test("a failed refresh stops loading and shows the error", async ({ page }) => {
  await openMock(page, "view=panel&kind=github-commits", {}, { github_fetch: "offline" });
  await page.locator(".action", { hasText: "Refresh" }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page.locator(".action.loading")).toHaveCount(0);
});

test("actions without a loading label show no spinner", async ({ page }) => {
  await openMock(page, "view=panel&kind=github-commits");
  await page.evaluate(() => {
    document.addEventListener("click", () => {
      (window as unknown as { __SPINNERS__: number }).__SPINNERS__ = document.querySelectorAll(".action-spinner").length;
      requestAnimationFrame(() => {
        (window as unknown as { __SPINNERS__: number }).__SPINNERS__ += document.querySelectorAll(".action-spinner").length;
      });
    });
  });
  await page.locator(".action", { hasText: "View profile" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { __SPINNERS__?: number }).__SPINNERS__)).toBe(0);
  await expect(page.locator(".action.loading")).toHaveCount(0);
});
