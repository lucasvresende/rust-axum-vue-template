import { test, expect } from "@playwright/test";

test("item creation sends quantity and displays server creation date", async ({ page }) => {
  const user = { id: "member", full_name: "Member", email: "member@example.com", is_superuser: false };
  const items: Record<string, unknown>[] = [];
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/login")) return route.fulfill({ json: { access_token: "test" } });
    if (path.endsWith("/users/me")) return route.fulfill({ json: user });
    if (route.request().method() === "POST") {
      const body = route.request().postDataJSON();
      expect(body).toEqual({ title: "Bolts", description: "", quantity: 12 });
      const item = { ...body, id: "item", owner_id: user.id, created_at: "2026-09-20T12:00:00Z" };
      items.push(item);
      return route.fulfill({ status: 201, json: item });
    }
    return route.fulfill({ json: items });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Sign in" }).click();
  await page.getByRole("button", { name: "New item" }).click();
  await expect(page.getByRole("spinbutton", { name: "Quantity" })).toHaveValue("1");
  await page.getByLabel("Title", { exact: true }).fill("Bolts");
  await page.getByRole("spinbutton", { name: "Quantity" }).fill("12");
  await page.getByRole("button", { name: "Create item", exact: true }).click();
  const row = page.getByRole("row").filter({ hasText: "Bolts" });
  await expect(row.getByRole("cell", { name: "12", exact: true })).toBeVisible();
  await expect(row.locator("time")).toHaveAttribute("datetime", "2026-09-20T12:00:00Z");
  await expect(row.locator("time")).not.toHaveText("—");
  for (const name of ["Quantity", "Creation date"]) {
    const header = page.getByRole("columnheader", { name, exact: true });
    await header.click();
    await expect(header).toHaveAttribute("aria-sort", "ascending");
  }
  await page.getByRole("button", { name: "New item" }).click();
  await expect(page.getByRole("spinbutton", { name: "Quantity" })).toHaveValue("1");
});
