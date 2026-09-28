import { signIn } from "./helpers";
import { test, expect } from "@playwright/test";

test("login dashboard is usable", async ({ page }) => {
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({
      json: path.endsWith("/login")
        ? { access_token: "test" }
        : path.endsWith("/users/me")
          ? {
              id: "admin",
              email: "admin@example.com",
              full_name: "Admin",
              is_superuser: true,
              is_active: true,
            }
          : [],
    });
  });
  await page.goto("/");

  await expect(page.getByText("Axum + Vue")).toBeVisible();

  await signIn(page);

  await expect(page.getByText("Good to see you")).toBeVisible();

  await page.screenshot({ path: "../artifacts/dashboard.png", fullPage: true });
});
