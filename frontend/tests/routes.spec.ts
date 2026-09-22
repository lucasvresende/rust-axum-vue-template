import { test, expect } from "@playwright/test";

for (const admin of [true, false]) {
  test(`page URLs survive login, history, and refresh (${admin ? "admin" : "member"})`, async ({
    page,
  }) => {
    await page.route("**/api/v1/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      return route.fulfill({
        json: path.endsWith("/login")
          ? { access_token: "test" }
          : path.endsWith("/users/me")
            ? {
                full_name: "Tester",
                email: "tester@example.com",
                is_superuser: admin,
              }
            : [],
      });
    });
    await page.goto("/items");
    await expect(page).toHaveURL(
      (url) =>
        url.pathname === "/login" &&
        url.searchParams.get("redirect") === "/items",
    );
    await page.reload();
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(
      page.getByRole("heading", { name: "My items", exact: true }),
    ).toBeVisible();
    await expect(page).toHaveURL(/\/items$/);
    const nav = page.getByRole("navigation", { name: "Workspace" });
    await nav.getByRole("link", { name: "Overview" }).click();
    await expect(page).toHaveURL(/\/overview$/);
    await page.goBack();
    await expect(page).toHaveURL(/\/items$/);
    await expect(nav.getByRole("link", { name: "My items" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await page.goForward();
    await expect(page).toHaveURL(/\/overview$/);
    await page.goto("/users");
    await expect(page).toHaveURL(admin ? /\/users$/ : /\/overview$/);
    await page.reload();
    await expect(
      page.getByRole("heading", {
        name: admin ? "Users" : "Good to see you, Tester.",
        exact: true,
      }),
    ).toBeVisible();
    await expect(page).toHaveURL(admin ? /\/users$/ : /\/overview$/);
    await page.getByRole("button", { name: "Sign out", exact: true }).click();
    await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
    await expect(page).toHaveURL(/\/login$/);
    await page.goto("/missing-page");
    await expect(page).toHaveURL(
      (url) =>
        url.pathname === "/login" &&
        url.searchParams.get("redirect") === "/overview",
    );
  });
}

for (const redirect of [
  "/items",
  "https://example.com",
  "//example.com",
  "/login",
  "/missing",
]) {
  test(`login validates redirect ${redirect}`, async ({ page }) => {
    await page.route("**/api/v1/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      return route.fulfill({
        json: path.endsWith("/login")
          ? { access_token: "test" }
          : path.endsWith("/users/me")
            ? { full_name: "Tester", is_superuser: false }
            : [],
      });
    });
    await page.goto(`/login?redirect=${encodeURIComponent(redirect)}`);
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page).toHaveURL(
      redirect === "/items" ? /\/items$/ : /\/overview$/,
    );
    await page.goto("/login");
    await expect(page).toHaveURL(/\/overview$/);
    await expect(
      page.getByRole("heading", { name: "Good to see you, Tester." }),
    ).toBeVisible();
  });
}

test("expired sessions return to login and retain destination", async ({
  page,
}) => {
  await page.addInitScript(() => localStorage.setItem("token", "expired"));
  await page.route("**/api/v1/**", (route) =>
    route.fulfill({ status: 401, json: { detail: "unauthorized" } }),
  );
  await page.goto("/items");
  await expect(page).toHaveURL(
    (url) =>
      url.pathname === "/login" &&
      url.searchParams.get("redirect") === "/items",
  );
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("button", { name: "Sign in" })).toBeEnabled();
  await expect(page).toHaveURL(
    (url) =>
      url.pathname === "/login" &&
      url.searchParams.get("redirect") === "/items",
  );
  expect(await page.evaluate(() => localStorage.getItem("token"))).toBeNull();
});
