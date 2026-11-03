import { expect, test, type Locator } from "@playwright/test";
import { openMock } from "./fixtures";

const NOTE_COLORS = ["yellow", "pink", "blue", "green"] as const;
const FILL_HOVER = "rgba(255, 255, 255, 0.07)";

const background = (tile: Locator) => tile.evaluate((el) => getComputedStyle(el).backgroundColor);

for (const color of NOTE_COLORS) {
  test(`a ${color} reminder keeps its color on hover`, async ({ page }) => {
    await openMock(page, "view=dock&new", { notes_list: [{ id: "lembrete-1", text: "Comprar pão", color, modifiedAt: 1 }] });
    const tile = page.locator(`.notes-tile.${color}`);
    const rest = await background(tile);

    await tile.hover();
    // O hover só clareia o post-it: o fundo continua opaco e diferente do repouso.
    await expect.poll(() => background(tile)).not.toBe(rest);
    const hover = await background(tile);
    expect(hover).not.toBe(FILL_HOVER);
    expect(hover).not.toMatch(/\/ 0\.|rgba/);
  });
}

test("the empty reminders tile uses the regular hover", async ({ page }) => {
  await openMock(page, "view=dock&new", { notes_list: [] });
  const tile = page.locator(".notes-tile.idle");
  await tile.hover();
  await expect.poll(() => background(tile)).toBe(FILL_HOVER);
});
