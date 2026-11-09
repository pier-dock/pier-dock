import { expect, type Page } from "@playwright/test";

export async function openMock(page: Page, query: string, responses: Record<string, unknown> = {}, errors: Record<string, string> = {}) {
  await page.addInitScript(({ responses, errors }) => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: responses, __PIER_TEST_ERRORS__: errors });
  }, { responses, errors });
  await page.goto(`/?${query}`);
  // O conteúdo do painel é carregado sob demanda: a moldura (`.popover`) aparece antes dele.
  await page.locator(query.includes("view=panel") ? ".popover > *" : ".dock-bar").first().waitFor();
}

export async function expectCall(page: Page, command: string, args: Record<string, unknown> = {}) {
  await expect.poll(() => page.evaluate(() => (window as unknown as {
    __PIER_TEST_CALLS__: { command: string; args: unknown }[];
  }).__PIER_TEST_CALLS__)).toContainEqual({ command, args: expect.objectContaining(args) });
}
