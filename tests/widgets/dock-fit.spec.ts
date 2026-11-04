import { test, expect, type Page } from "@playwright/test";
import { openMock } from "./fixtures";

async function configure(page: Page, edge: string, width: string, apps = 16) {
  await page.evaluate(async ({ edge, width, apps }) => {
    const { useConfig } = await import("/src/store/config.ts");
    const config = useConfig.getState().config;
    const pinnedApps = Array.from({ length: apps }, (_, i) => ({
      id: `fit-${i}`,
      name: `App ${i}`,
      target: `C:\\fit\\app-${i}.exe`,
      exe: `C:\\fit\\app-${i}.exe`,
      aumid: null,
    }));
    useConfig.setState({ config: { ...config, dockPosition: edge, dockWidth: width, pinnedApps, magnification: 1.5, widgets: [], showRunningUnpinned: false } });
  }, { edge, width, apps });
}

const iconSize = (page: Page) => page.locator(".dock-root").evaluate((el) => parseFloat(getComputedStyle(el).getPropertyValue("--icon")));

for (const edge of ["left", "right", "bottom"]) {
  for (const width of ["fit", "full"]) {
    test(`the ${edge} ${width} dock shrinks its icons to fit the screen`, async ({ page }) => {
      await page.setViewportSize({ width: 620, height: 760 });
      await openMock(page, "view=dock");
      await configure(page, edge, width);
      await expect.poll(() => iconSize(page)).toBeLessThan(40);
      await expect.poll(() => page.evaluate(() => {
        const buttons = [...document.querySelectorAll(".dock-bar .app-button")].map((el) => el.getBoundingClientRect());
        return buttons.every((r) => r.top >= 0 && r.left >= 0 && r.bottom <= innerHeight && r.right <= innerWidth);
      })).toBe(true);
    });
  }
}

test("a dock that already fits keeps the configured icon size", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await openMock(page, "view=dock");
  await configure(page, "right", "fit", 3);
  const configured = await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    return useConfig.getState().config.iconSize;
  });
  await expect.poll(() => iconSize(page)).toBe(configured);
});

test("the icons grow back when the window has room again", async ({ page }) => {
  await page.setViewportSize({ width: 620, height: 760 });
  await openMock(page, "view=dock");
  await configure(page, "right", "fit");
  await expect.poll(() => iconSize(page)).toBeLessThan(40);
  await page.setViewportSize({ width: 900, height: 2400 });
  await expect.poll(() => iconSize(page)).toBe(40);
});

// O ícone encolhido na lateral precisa crescer ao passar para a horizontal, e o botão muda de largura
// antes do slot: a medida não pode reagir a isso, ou a dock entra em loop e o React a desmonta.
for (const width of [1440, 1728, 1920]) {
  for (const apps of [8, 16]) {
    test(`switching edges at ${width}px with ${apps} apps keeps the dock alive and stable`, async ({ page }) => {
      const errors: string[] = [];
      page.on("pageerror", (error) => errors.push(error.message));
      await page.setViewportSize({ width, height: 900 });
      await openMock(page, "view=dock");
      const pinnedApps = Array.from({ length: apps }, (_, i) => ({ id: `edge-${i}`, name: `App ${i}`, target: `C:\\edge\\${i}.exe`, exe: `C:\\edge\\${i}.exe`, aumid: null }));
      const set = (patch: Record<string, unknown>) => page.evaluate(async (patch) => {
        const { useConfig } = await import("/src/store/config.ts");
        useConfig.setState({ config: { ...useConfig.getState().config, ...patch } });
      }, patch);
      await set({ pinnedApps, magnification: 1.5 });
      for (const dockPosition of ["right", "top", "left", "bottom", "top"]) {
        await set({ dockPosition });
        await page.waitForTimeout(600);
        await expect(page.locator(".dock-bar")).toBeVisible();
        const samples = new Set<number>();
        for (let i = 0; i < 6; i++) {
          samples.add(await iconSize(page));
          await page.waitForTimeout(50);
        }
        expect(samples.size, `icon size on ${dockPosition}: ${[...samples].join(", ")}`).toBe(1);
      }
      expect(errors).toEqual([]);
    });
  }
}
