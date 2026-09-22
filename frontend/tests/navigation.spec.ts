import { test, expect } from "@playwright/test";

test("theme covers dialogs and navigation can collapse", async ({ page }) => {
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({
      json: path.endsWith("/login")
        ? { access_token: "test" }
        : path.endsWith("/users/me")
          ? {
              full_name: "Member",
              email: "member@example.com",
              is_superuser: false,
            }
          : [],
    });
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(
    page.getByRole("link", { name: "Users", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Toggle dark mode" }).click();
  await expect(page.locator("html")).toHaveClass("app-dark");
  await page.getByRole("button", { name: "New item" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  expect(await dialog.evaluate((el) => el.closest(".app-dark") !== null)).toBe(
    true,
  );
  const bounds = await dialog.boundingBox();
  expect(bounds!.x).toBeGreaterThanOrEqual(0);
  expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(390);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Toggle navigation" }).click();
  await expect(page.getByRole("navigation", { name: "Workspace" })).toHaveCount(
    0,
  );
  await page.getByRole("button", { name: "Toggle navigation" }).click();
  await expect(
    page.getByRole("navigation", { name: "Workspace" }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("link", { name: "My items", exact: true }).click();
  await page.getByRole("button", { name: "Go to overview" }).click();
  await expect(
    page.getByRole("heading", { name: "Good to see you, Member." }),
  ).toBeVisible();
  const account = page.getByRole("button", { name: "Account", exact: true });
  await account.focus();
  await page.keyboard.press("Enter");
  const accountDetails = page.getByRole("dialog", { name: "Account details" });
  await expect(accountDetails).toBeVisible();
  await expect(accountDetails.getByText("member@example.com")).toBeVisible();
  await expect(account).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(accountDetails).not.toBeVisible();
  await account.click();
  await accountDetails.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
});
