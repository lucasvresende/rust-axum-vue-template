import { test, expect } from "@playwright/test";
import { signIn } from "./helpers";

test("admin invites and deactivates a user", async ({ page }) => {
  const admin = {
    id: "admin",
    email: "admin@example.com",
    full_name: "Admin",
    is_active: true,
    is_superuser: true,
  };
  const users = [admin];
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/login"))
      return route.fulfill({ json: { access_token: "test" } });
    if (path.endsWith("/users/me")) return route.fulfill({ json: admin });
    if (path.endsWith("/items")) return route.fulfill({ json: [] });
    if (path.endsWith("/users") && route.request().method() === "GET")
      return route.fulfill({ json: users });
    if (path.endsWith("/users") && route.request().method() === "POST") {
      const body = route.request().postDataJSON();
      expect(body).toEqual({
        email: "new@example.com",
        full_name: "New Member",
      });
      users.push({
        id: "new",
        email: body.email,
        full_name: body.full_name,
        is_active: true,
        is_superuser: false,
        needs_setup: true,
      } as typeof admin);
      return route.fulfill({ status: 201, json: users[1] });
    }
    if (path.endsWith("/status")) {
      expect(route.request().postDataJSON()).toEqual({ is_active: false });
      users[1].is_active = false;
      return route.fulfill({ json: users[1] });
    }
    return route.fulfill({ status: 404, json: { detail: "not found" } });
  });
  await page.goto("/users");
  await signIn(page);
  await page.getByRole("button", { name: "New user" }).click();
  const dialog = page.getByRole("dialog", { name: "Create user" });
  await dialog.getByRole("textbox", { name: "Full name" }).fill("New Member");
  await dialog.getByRole("textbox", { name: "Email" }).fill("new@example.com");
  await expect(dialog.locator('input[type="password"]')).toHaveCount(0);
  await dialog.getByRole("button", { name: "Send invitation" }).click();
  await expect(page.getByText("new@example.com")).toBeVisible();
  await page.getByRole("button", { name: "Deactivate" }).last().click();
  await expect(page.getByText("Inactive")).toBeVisible();
});

test("recovery, setup error, and email confirmation pages", async ({
  page,
}) => {
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/recovery-requests"))
      return route.fulfill({
        json: {
          message: "If that account exists, a recovery email will be sent.",
        },
      });
    if (path.endsWith("/setup"))
      return route.fulfill({
        status: 400,
        json: { detail: "invalid or expired link" },
      });
    return route.fulfill({ status: 204, body: "" });
  });
  await page.goto("/forgot-password");
  await page.getByRole("textbox", { name: "Email" }).fill("user@example.com");
  await page.getByRole("button", { name: "Send recovery link" }).click();
  await expect(page.getByRole("status")).toContainText(
    "If that account exists",
  );
  await page.goto("/set-password?token=expired");
  await page
    .locator('input[type="password"]')
    .first()
    .fill("long-enough-password");
  await page
    .locator('input[type="password"]')
    .last()
    .fill("long-enough-password");
  await page.getByRole("button", { name: "Save password" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "invalid or expired link",
  );
  await page.goto("/confirm-email?token=valid");
  await page.getByRole("button", { name: "Confirm email" }).click();
  await expect(page.getByRole("status")).toContainText("Email confirmed");
});

test("profile edits name and requests email confirmation", async ({ page }) => {
  const user = {
    id: "member",
    email: "old@example.com",
    full_name: "Old Name",
    is_active: true,
    is_superuser: false,
  };
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/login"))
      return route.fulfill({ json: { access_token: "test" } });
    if (path.endsWith("/users/me") && route.request().method() === "GET")
      return route.fulfill({ json: user });
    if (path.endsWith("/users/me") && route.request().method() === "PUT") {
      user.full_name = route.request().postDataJSON().full_name;
      return route.fulfill({ json: user });
    }
    if (path.endsWith("/email-change")) {
      expect(route.request().postDataJSON()).toEqual({
        email: "new@example.com",
        current_password: "current-password",
      });
      return route.fulfill({ status: 204, body: "" });
    }
    return route.fulfill({ json: [] });
  });
  await page.goto("/profile");
  await signIn(page);
  await page.getByRole("textbox", { name: "Display name" }).fill("New Name");
  await page.getByRole("button", { name: "Save name" }).click();
  await expect(page.getByRole("status")).toContainText("confirmation email");
  await page
    .getByRole("textbox", { name: "New email" })
    .fill("new@example.com");
  await page.locator('input[type="password"]').fill("current-password");
  await page.getByRole("button", { name: "Send confirmation" }).click();
  await expect(page.getByRole("status")).toContainText(
    "current address remains active",
  );
});

test("account setup fits a mobile viewport", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/set-password?token=example");
  await expect(page.getByText("Set your password")).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
