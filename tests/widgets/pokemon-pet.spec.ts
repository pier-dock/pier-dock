import { test, expect } from "@playwright/test";
import { openMock, expectCall } from "./fixtures";

test("pokemon pet shows the chosen party sprite and walks the dock", async ({ page }) => {
  await openMock(page, "view=dock");
  await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    useConfig.setState({ config: { ...useConfig.getState().config, pokemonPetEnabled: true, pokemonPetIds: [6], widgets: [] } });
  });
  await expectCall(page, "pokemon_sprite", { id: 6 });
  await expect(page.locator(".pokemon-pet-sprite")).toBeVisible();
  const first = (await page.locator(".pokemon-pet").boundingBox())!.x;
  await expect.poll(async () => (await page.locator(".pokemon-pet").boundingBox())!.x).not.toBe(first);
});

test("pokemon pet is off by default", async ({ page }) => {
  await openMock(page, "view=dock");
  await expect(page.locator(".pokemon-pet")).toHaveCount(0);
});

test("every chosen pokemon walks the dock from its own spot", async ({ page }) => {
  await openMock(page, "view=dock");
  await page.evaluate(async () => {
    const { useConfig } = await import("/src/store/config.ts");
    useConfig.setState({ config: { ...useConfig.getState().config, pokemonPetEnabled: true, pokemonPetIds: [6, 25, 150], widgets: [] } });
  });
  for (const id of [6, 25, 150]) await expectCall(page, "pokemon_sprite", { id });
  await expect(page.locator(".pokemon-pet")).toHaveCount(3);
  const xs = await page.locator(".pokemon-pet").evaluateAll((els) => els.map((el) => Math.round(el.getBoundingClientRect().x)));
  expect(new Set(xs).size).toBe(3);
});

test("the widget picks up to ten pokemon and keeps at least one", async ({ page }) => {
  await page.addInitScript(() => Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: {}, __PIER_TEST_ERRORS__: {} }));
  await page.goto("/?view=settings");
  await page.getByRole("tab", { name: "Widgets", exact: true }).click();
  await page.locator('[data-kind="pokemon"] .widget-card-configure').click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText("1 of 10 selected")).toBeVisible();
  await expect(dialog.getByRole("button", { name: /^Remove / })).toBeDisabled();

  const search = dialog.getByPlaceholder("Search by name or number");
  for (let id = 1; id <= 9; id++) {
    await search.fill(`#${id}`);
    await dialog.locator(".choice-list .choice").first().click();
  }
  await expect(dialog.getByText("10 of 10 selected")).toBeVisible();
  await expect(dialog.getByText(/Limit of 10 reached/)).toBeVisible();
  await search.fill("#100");
  await expect(dialog.locator(".choice-list .choice").first()).toBeDisabled();

  await dialog.getByRole("button", { name: /^Remove / }).first().click();
  await expect(dialog.getByText("9 of 10 selected")).toBeVisible();
  await expectCall(page, "save_config", { config: expect.objectContaining({ pokemonPetIds: [1, 2, 3, 4, 5, 6, 7, 8, 9] }) });
});
