import { test, expect } from "@playwright/test";

test("edit item and require confirmation before deleting", async ({ page }) => {
  const user = {
    id: "member",
    full_name: "Member",
    email: "member@example.com",
    is_superuser: false,
  };

  let items = [
    {
      id: "item",
      owner_id: "member",
      title: "Bolts",
      description: "Steel",
      quantity: 12,
      created_at: "2026-09-20T12:00:00Z",
    },
  ];

  let deletes = 0;

  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname;

    if (path.endsWith("/login"))
      return route.fulfill({ json: { access_token: "test" } });

    if (path.endsWith("/users/me")) return route.fulfill({ json: user });

    if (route.request().method() === "PUT") {
      expect(path).toBe("/api/v1/items/item");
      expect(route.request().postDataJSON()).toEqual({
        title: "Nuts",
        description: "Brass",
        quantity: 5,
      });
      items = [{ ...items[0], ...route.request().postDataJSON() }];
      return route.fulfill({ json: items[0] });
    }

    if (route.request().method() === "DELETE") {
      deletes++;
      if (deletes === 1)
        return route.fulfill({ status: 500, json: { detail: "Try again" } });
      items = [];
      return route.fulfill({ status: 204 });
    }
    return route.fulfill({ json: items });
  });

  await page.goto("/");

  await page.getByRole("button", { name: "Sign in" }).click();
  await page.getByRole("button", { name: "Actions for Bolts" }).click();
  await page.getByRole("menuitem", { name: "Edit", exact: true }).click();
  const edit = page.getByRole("dialog", { name: "Edit item" });

  await expect(edit.getByLabel("Title", { exact: true })).toHaveValue("Bolts");
  await edit.getByLabel("Title", { exact: true }).fill("Nuts");
  await edit.getByLabel("Description", { exact: true }).fill("Brass");
  await edit.getByRole("spinbutton", { name: "Quantity" }).fill("5");
  await edit.getByRole("button", { name: "Save changes" }).click();

  await expect(edit).not.toBeVisible();
  await expect(
    page.getByRole("row").filter({ hasText: "Nuts" }).locator("time"),
  ).toHaveAttribute("datetime", "2026-09-20T12:00:00Z");
  const openDelete = async () => {
    await page.getByRole("button", { name: "Actions for Nuts" }).click();
    await page.getByRole("menuitem", { name: "Delete", exact: true }).click();
  };
  await openDelete();
  const confirmation = page.getByRole("dialog", {
    name: "Delete item",
    exact: true,
  });
  await expect(confirmation).toContainText("Nuts");
  expect(deletes).toBe(0);
  await confirmation.getByRole("button", { name: "Cancel" }).click();
  await expect(confirmation).not.toBeVisible();
  expect(deletes).toBe(0);
  await openDelete();
  await confirmation
    .getByRole("button", { name: "Delete item", exact: true })
    .click();
  await expect(page.getByText("Try again", { exact: true })).toBeVisible();
  await expect(confirmation).toBeVisible();
  await confirmation
    .getByRole("button", { name: "Delete item", exact: true })
    .click();
  await expect(confirmation).not.toBeVisible();
  await expect(
    page.getByText("No items yet. Create your first one."),
  ).toBeVisible();
  expect(deletes).toBe(2);
});
