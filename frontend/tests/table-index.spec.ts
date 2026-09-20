import { test, expect } from "@playwright/test";

for (const view of ["My items", "Users"]) {
  test(`${view} sorts numeric indexes across pages`, async ({ page }) => {
    const user = { id: "admin", full_name: "Admin", email: "admin@example.com", is_superuser: true };
    await page.route("**/api/v1/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      const rows = Array.from({ length: 12 }, (_, i) => ({
        id: String(i), title: `Item ${i}`, description: "", full_name: `User ${i}`,
        email: `user${i}@example.com`, is_superuser: false,
      }));
      return route.fulfill({ json: path.endsWith("/login") ? { access_token: "test" } : path.endsWith("/users/me") ? user : rows });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "Sign in" }).click();
    await page.getByRole("navigation", { name: "Workspace" }).getByRole("button", { name: view, exact: true }).click();
    const table = page.getByRole("table");
    const indexes = table.locator("tbody tr td:first-child");
    const header = table.getByRole("columnheader", { name: "#", exact: true });
    await header.click();
    await expect(header).toHaveAttribute("aria-sort", "ascending");
    await expect(indexes).toHaveText(["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]);
    await header.click();
    await expect(header).toHaveAttribute("aria-sort", "descending");
    await expect(indexes).toHaveText(["12", "11", "10", "9", "8", "7", "6", "5", "4", "3"]);
    await page.getByRole("button", { name: "Next Page", exact: true }).click();
    await expect(indexes).toHaveText(["2", "1"]);
  });
}
