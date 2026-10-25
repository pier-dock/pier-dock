import { test, expect, type Page } from "@playwright/test";
import { expectCall } from "./fixtures";

async function openReport(page: Page, button: string, responses: Record<string, unknown> = {}, errors: Record<string, string> = {}) {
  await page.addInitScript(({ responses, errors }) => {
    Object.assign(window, { __PIER_TEST_CALLS__: [], __PIER_TEST_RESPONSES__: responses, __PIER_TEST_ERRORS__: errors });
  }, { responses, errors });
  await page.goto("/?view=settings");
  await page.getByRole("tab", { name: "General" }).click();
  await page.getByRole("button", { name: button }).click();
  return page.getByRole("dialog");
}

test("sends the report and shows the created issue", async ({ page }) => {
  const dialog = await openReport(page, "Report a problem", { report_issue: { kind: "created", url: "https://github.com/pier-dock/pier-dock/issues/42" } });
  await expect(dialog.getByText("This becomes a public issue on GitHub")).toBeVisible();
  const send = dialog.getByRole("button", { name: "Create issue" });
  await expect(send).toBeDisabled();
  await dialog.getByLabel("Title").fill("Dock disappears on monitor change");
  await dialog.getByLabel("Description").fill("It happens on Windows 11");
  await send.click();
  await expectCall(page, "report_issue", { kind: "bug", title: "Dock disappears on monitor change", description: "It happens on Windows 11", logs: null });
  await expect(dialog.getByRole("heading", { name: "Got it, thank you!" })).toBeVisible();
  await expect(dialog.getByText("Dock disappears on monitor change")).toBeVisible();
  await expect(dialog.getByText("pier-dock/pier-dock #42")).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Open issue" })).toBeVisible();
});

test("send another goes back to an empty form", async ({ page }) => {
  const dialog = await openReport(page, "Suggest a feature");
  await dialog.getByLabel("Title").fill("A queue widget");
  await dialog.getByRole("button", { name: /Create issue|Send report/ }).click();
  await dialog.getByRole("button", { name: "Send another" }).click();
  await expect(dialog.getByLabel("Title")).toHaveValue("");
});

test("without an API result it says GitHub opened in the browser", async ({ page }) => {
  const dialog = await openReport(page, "Report a problem", { report_issue: { kind: "fallback", url: "https://github.com/pier-dock/pier-dock/issues/new", reason: "offline" } });
  await dialog.getByLabel("Title").fill("Something broke");
  await dialog.getByRole("button", { name: /Create issue|Send report/ }).click();
  await expect(dialog.getByRole("heading", { name: "GitHub opened in your browser" })).toBeVisible();
  await expect(dialog.getByText("Couldn't send it directly (offline).")).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Open issue" })).toHaveCount(0);
});

test("keeps the form editable when sending fails", async ({ page }) => {
  const dialog = await openReport(page, "Report a problem", {}, { report_issue: "boom" });
  await dialog.getByLabel("Title").fill("Something broke");
  await dialog.getByRole("button", { name: /Create issue|Send report/ }).click();
  await expect(dialog.getByText("Couldn't send it: boom")).toBeVisible();
  await expect(dialog.getByLabel("Title")).toBeEnabled();
});
