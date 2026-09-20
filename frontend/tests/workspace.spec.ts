import { test, expect } from "@playwright/test";

for (const mobile of [false, true]) {
  test(`workspace navigation works on ${mobile ? "mobile" : "desktop"}`, async ({
    page,
  }) => {
    if (mobile) await page.setViewportSize({ width: 390, height: 844 });

    await page.route("**/api/v1/**", (route) => {
      const path = new URL(route.request().url()).pathname;

      const user = {
        full_name: "Admin",
        email: "admin@example.com",
        is_superuser: true,
      };

      return route.fulfill({
        json: path.endsWith("/login")
          ? { access_token: "test" }
          : path.endsWith("/users/me")
            ? user
            : path.endsWith("/users")
              ? [user]
              : [],
      });
    });

    await page.goto("/");

    await page.getByRole("button", { name: "Sign in" }).click();

    const nav = page.getByRole("navigation", { name: "Workspace" });

    await nav.getByRole("link", { name: "My items" }).click();

    await expect(
      page.getByRole("heading", { name: "My items", exact: true }),
    ).toBeVisible();

    await expect(nav.getByRole("link", { name: "My items" })).toHaveAttribute(
      "aria-current",
      "page",
    );

    await page.getByRole("button", { name: "New item" }).click();

    await expect(page.getByRole("dialog")).toBeVisible();

    await page.keyboard.press("Escape");

    await nav.getByRole("link", { name: "Users" }).focus();

    await page.keyboard.press("Enter");

    await expect(
      page.getByRole("heading", { name: "Users", exact: true }),
    ).toBeVisible();

    await expect(page.getByRole("columnheader", { name: "Email" })).toBeVisible();

    await expect(page.getByRole("button", { name: "New item" })).toHaveCount(0);

    await nav.getByRole("link", { name: "Overview" }).click();

    await expect(
      page.getByRole("heading", { name: "Good to see you, Admin." }),
    ).toBeVisible();

    await expect(nav.getByRole("link", { name: "Overview" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });
}
