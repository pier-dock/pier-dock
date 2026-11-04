import { expect, test, type Locator } from "@playwright/test";
import { expectCall, openMock } from "./fixtures";

const NOTE_COLORS = {
  yellow: "rgb(247, 226, 122)",
  pink: "rgb(244, 182, 200)",
  blue: "rgb(169, 205, 240)",
  green: "rgb(181, 224, 176)",
} as const;
const FILL_HOVER = "rgba(255, 255, 255, 0.07)";

const note = (id: string, color: keyof typeof NOTE_COLORS = "yellow") => ({ id, text: "Comprar pão", color, modifiedAt: 1 });
const background = (el: Locator) => el.evaluate((node) => getComputedStyle(node).backgroundColor);

for (const [color, rgb] of Object.entries(NOTE_COLORS) as [keyof typeof NOTE_COLORS, string][]) {
  test(`a ${color} reminder shows its color on a dot over the glass tile`, async ({ page }) => {
    await openMock(page, "view=dock&new", { notes_list: [note("lembrete-1", color)] });
    const tile = page.locator(".notes-tile");
    await expect(tile.locator(".note-dot")).toHaveCSS("background-color", rgb);

    await tile.hover();
    await expect.poll(() => background(tile)).toBe(FILL_HOVER);
  });
}

test("the tile shows the position only with more than one reminder", async ({ page }) => {
  await openMock(page, "view=dock&new", { notes_list: [note("a"), note("b", "pink")] });
  await expect(page.locator(".notes-position")).toHaveText("1 of 2");
});

test("a single reminder has no position", async ({ page }) => {
  await openMock(page, "view=dock&new", { notes_list: [note("a")] });
  await expect(page.locator(".notes-text")).toBeVisible();
  await expect(page.locator(".notes-position")).toHaveCount(0);
});

test("a long reminder wraps inside the tile", async ({ page }) => {
  await openMock(page, "view=dock&new", { notes_list: [{ ...note("a"), text: "Ligar para o contador antes das 17h para fechar a declaração e enviar os comprovantes" }] });
  const tile = (await page.locator(".notes-tile").boundingBox())!;
  const text = page.locator(".notes-text");
  const box = (await text.boundingBox())!;

  expect(box.height).toBeGreaterThan(20);
  expect(box.x + box.width).toBeLessThanOrEqual(tile.x + tile.width);
  expect(box.y + box.height).toBeLessThanOrEqual(tile.y + tile.height);
  expect(await text.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
});

test("the empty reminders tile uses the regular hover", async ({ page }) => {
  await openMock(page, "view=dock&new", { notes_list: [] });
  const tile = page.locator(".notes-tile.idle");
  await tile.hover();
  await expect.poll(() => background(tile)).toBe(FILL_HOVER);
});

test.describe("panel", () => {
  test.beforeEach(async ({ page }) => {
    await openMock(page, "view=panel&kind=notes", { notes_list: [note("a")] });
  });

  test("the action bar appears on hover and on focus", async ({ page }) => {
    const bar = page.locator(".note-bar");
    await expect(bar).toHaveCSS("opacity", "0");

    await page.locator(".note-card").hover();
    await expect(bar).toHaveCSS("opacity", "1");

    await page.mouse.move(0, 0);
    await expect(bar).toHaveCSS("opacity", "0");

    await page.locator(".note-input").focus();
    await expect(bar).toHaveCSS("opacity", "1");
  });

  test("picking a color saves the reminder", async ({ page }) => {
    await page.locator(".note-card").hover();
    await page.locator(".note-swatch.blue").click();
    await expectCall(page, "notes_save", { id: "a", color: "blue" });
  });

  test("the delete confirmation stays visible after the mouse leaves", async ({ page }) => {
    await page.locator(".note-card").hover();
    await page.locator(".note-delete").click();
    await page.mouse.move(0, 0);
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());

    await expect(page.locator(".note-delete.confirm")).toBeVisible();
    await expect(page.locator(".note-bar")).toHaveCSS("opacity", "1");
  });
});
