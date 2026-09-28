import type { Page } from "@playwright/test";

export async function signIn(page: Page) {
  await page.getByRole("textbox", { name: "Email" }).fill("admin@example.com");
  await page
    .locator('input[type="password"]')
    .fill("development-admin-password");
  await page.getByRole("button", { name: "Sign in" }).click();
}
