import { test, expect, type Locator, type Page } from "@playwright/test";

async function expectInsideMenu(menu: Locator, field: Locator) {
  const menuBounds = await menu.boundingBox();
  const fieldBounds = await field.boundingBox();
  expect(menuBounds).not.toBeNull();
  expect(fieldBounds).not.toBeNull();
  expect(fieldBounds!.x).toBeGreaterThanOrEqual(menuBounds!.x - 1);
  expect(fieldBounds!.x + fieldBounds!.width).toBeLessThanOrEqual(
    menuBounds!.x + menuBounds!.width + 1,
  );
}

async function openFilter(page: Page, field: string) {
  const button = page.getByRole("button", {
    name: `Filter ${field}`,
    exact: true,
  });
  // Finish horizontal scrolling before opening an anchored overlay: PrimeVue
  // dismisses the overlay on scroll, including a queued scroll from automation.
  await button.scrollIntoViewIfNeeded();
  await button.focus();
  await button.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  await button.press("Enter");
  return page.getByRole("dialog", { name: `Filter ${field}`, exact: true });
}
async function searchColumn(page: Page, field: string, value: string) {
  const menu = await openFilter(page, field);
  await menu
    .getByRole("textbox", { name: `Search ${field}`, exact: true })
    .fill(value);
  await menu.getByRole("button", { name: "Apply", exact: true }).click();
}
async function expectSearchCleared(page: Page, field: string) {
  const menu = await openFilter(page, field);
  await expect(
    menu.getByRole("textbox", { name: `Search ${field}`, exact: true }),
  ).toHaveValue("");
  await page.keyboard.press("Escape");
}

for (const view of ["items", "users"] as const) {
  test(`${view} supports search, column filters, reset and column selection`, async ({
    page,
  }) => {
    await page.route("**/api/v1/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      return route.fulfill({
        json: path.endsWith("/login")
          ? { access_token: "test" }
          : path.endsWith("/users/me")
            ? { id: "admin", full_name: "Admin", is_superuser: true }
            : Array.from({ length: 12 }, (_, i) => ({
                id: String(i),
                title: `Item ${i}`,
                description: i === 11 ? "Unique" : "Common",
                quantity: i + 1,
                created_at: "2026-09-20T12:00:00Z",
                full_name: `Person ${i}`,
                email: `person${i}@example.com`,
                is_superuser: i === 11,
              })),
      });
    });
    await page.goto(`/${view}`);
    await page.getByRole("button", { name: "Sign in" }).click();
    const rows = page.locator("tbody tr");
    const field = view === "items" ? "Title" : "Name";
    const value = view === "items" ? "Item 11" : "Person 11";
    const secondary = view === "items" ? "Description" : "Role";
    await page.getByRole("button", { name: "Next Page", exact: true }).click();
    await searchColumn(page, field, value);
    await expect(rows).toHaveCount(1);
    await expect(rows).toContainText(value);
    await searchColumn(page, secondary, view === "items" ? "Common" : "Member");
    await expect(rows).toContainText("No results match your filters.");
    await page.getByRole("button", { name: "Clear all filters" }).click();
    await expect(rows).toHaveCount(10);
    await expectSearchCleared(page, field);
    await expectSearchCleared(page, secondary);
    await page
      .getByRole("textbox", { name: `Search ${view}`, exact: true })
      .fill(value);
    await expect(rows).toHaveCount(1);
    await expect(rows).toContainText(value);
    await page.getByRole("button", { name: "Clear all filters" }).click();
    await expect(
      page.getByRole("textbox", { name: `Search ${view}`, exact: true }),
    ).toHaveValue("");
    await expect(rows).toHaveCount(10);
    await searchColumn(page, secondary, "no match");
    await page
      .getByRole("combobox", { name: `${view} columns` })
      .press("Space");
    await page.getByRole("option", { name: secondary, exact: true }).click();
    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("columnheader", { name: secondary, exact: true }),
    ).toHaveCount(0);
    await expect(rows).toHaveCount(10);
    await page
      .getByRole("combobox", { name: `${view} columns` })
      .press("Space");
    await page.getByRole("option", { name: secondary, exact: true }).click();
    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("columnheader", { name: secondary, exact: true }),
    ).toBeVisible();
    await expectSearchCleared(page, secondary);
  });
}

for (const mobile of [false, true]) {
  test(`navigation animates open and close on ${mobile ? "mobile" : "desktop"}`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: mobile ? 390 : 1280, height: 844 });
    await page.route("**/api/v1/**", (route) =>
      route.fulfill({
        json: route.request().url().endsWith("/login")
          ? { access_token: "test" }
          : route.request().url().endsWith("/users/me")
            ? { full_name: "Member", is_superuser: false }
            : [],
      }),
    );
    await page.goto("/");
    await page.getByRole("button", { name: "Sign in" }).click();
    const nav = page.getByRole("navigation", { name: "Workspace" });
    await expect(nav).toBeVisible();
    await page.evaluate(() => {
      (window as any).drawerTransitions = [];
      document.addEventListener("transitionrun", (event) => {
        if ((event.target as HTMLElement).id === "workspace-navigation")
          (window as any).drawerTransitions.push(
            (event as TransitionEvent).propertyName,
          );
      });
    });
    const toggle = page.getByRole("button", { name: "Toggle navigation" });
    await toggle.click();
    await expect(nav).toHaveCount(0);
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(
      await page.evaluate(() => (window as any).drawerTransitions),
    ).toContain("transform");
    await page.evaluate(() => {
      (window as any).drawerTransitions = [];
    });
    await toggle.click();
    await expect(nav).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => (window as any).drawerTransitions))
      .toContain("transform");
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await expect(page.locator(".workspace-drawer-enter-active")).toHaveCount(0);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.evaluate(() => {
      (window as any).drawerTransitions = [];
    });
    await toggle.click();
    await expect(nav).toHaveCount(0);
    await toggle.click();
    await expect(nav).toBeVisible();
    expect(
      await page.evaluate(() => (window as any).drawerTransitions),
    ).toEqual([]);
  });
}

async function filterFixture(page: Page) {
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    return route.fulfill({
      json: path.endsWith("/login")
        ? { access_token: "test" }
        : path.endsWith("/users/me")
          ? { id: "admin", full_name: "Admin", is_superuser: true }
          : Array.from({ length: 5 }, (_, i) => ({
              id: String(i),
              title: `Item ${i}`,
              description: i % 2 ? "Steel" : "Wood",
              quantity: i * 5,
              created_at: `2026-09-${20 + i}T12:00:00Z`,
              full_name: `Person ${i}`,
              email: `person${i}@example.com`,
              is_superuser: i === 0,
            })),
    });
  });
  await page.goto("/items");
  await page.getByRole("button", { name: "Sign in" }).click();
}

for (const mobile of [false, true]) {
  test(`filter menus support inclusive ranges and value selection on ${mobile ? "mobile" : "desktop"}`, async ({
    page,
  }) => {
    if (mobile) await page.setViewportSize({ width: 390, height: 844 });
    await filterFixture(page);
    const rows = page.locator("tbody tr");
    const titles = rows.locator("td:nth-child(2)");
    await expect(page.locator("thead tr")).toHaveCount(1);
    let menu = await openFilter(page, "Quantity");
    const bounds = await menu.boundingBox();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(mobile ? 390 : 1280);
    await expectInsideMenu(
      menu,
      menu.getByRole("spinbutton", { name: "Quantity minimum" }),
    );
    await expectInsideMenu(
      menu,
      menu.getByRole("spinbutton", { name: "Quantity maximum" }),
    );
    await menu.getByRole("spinbutton", { name: "Quantity minimum" }).fill("5");
    await menu.getByRole("spinbutton", { name: "Quantity maximum" }).fill("15");
    await menu
      .getByRole("spinbutton", { name: "Quantity maximum" })
      .press("Tab");
    // Menu edits remain a draft until Apply.
    await expect(rows).toHaveCount(5);
    await menu.getByRole("button", { name: "Apply", exact: true }).click();
    await expect(titles).toHaveText(["Item 1", "Item 2", "Item 3"]);
    menu = await openFilter(page, "Quantity");
    await expect(
      menu.getByRole("spinbutton", { name: "Quantity minimum" }),
    ).toHaveValue("5");
    await menu.getByRole("button", { name: "Clear", exact: true }).click();
    await expect(rows).toHaveCount(5);
    menu = await openFilter(page, "Quantity");
    await menu.getByRole("spinbutton", { name: "Quantity maximum" }).fill("0");
    await menu
      .getByRole("spinbutton", { name: "Quantity maximum" })
      .press("Tab");
    await menu.getByRole("button", { name: "Apply", exact: true }).click();
    await expect(titles).toHaveText(["Item 0"]);
    await page.getByRole("button", { name: "Clear all filters" }).click();

    menu = await openFilter(page, "Title");
    await menu
      .getByRole("combobox", { name: "Title filter type" })
      .press("Space");
    await page
      .getByRole("option", { name: "Selected values", exact: true })
      .click();
    await menu
      .getByRole("combobox", { name: "Select Title values" })
      .press("Space");
    await page
      .getByRole("searchbox", { name: "Find Title values" })
      .fill("Item 1");
    await expect(
      page.getByRole("option", { name: "Item 0", exact: true }),
    ).toHaveCount(0);
    await page.getByRole("option", { name: "Item 1", exact: true }).click();
    await page
      .getByRole("searchbox", { name: "Find Title values" })
      .fill("Item 3");
    await page.getByRole("option", { name: "Item 3", exact: true }).click();
    await page.keyboard.press("Escape");
    await menu.getByRole("button", { name: "Apply", exact: true }).click();
    await expect(titles).toHaveText(["Item 1", "Item 3"]);
    await page
      .getByRole("textbox", { name: "Search items", exact: true })
      .fill("Wood");
    await expect(rows).toContainText("No results match your filters.");
    await page.getByRole("button", { name: "Clear all filters" }).click();
    await expect(rows).toHaveCount(5);
  });
}

test("date ranges include whole boundary days and combine with other columns", async ({
  page,
}) => {
  await filterFixture(page);
  const menu = await openFilter(page, "Creation date");
  await expectInsideMenu(
    menu,
    menu.getByLabel("Creation date from", { exact: true }),
  );
  await menu
    .getByLabel("Creation date from", { exact: true })
    .fill("2026-09-21");
  await menu.getByLabel("Creation date from", { exact: true }).press("Tab");
  await menu.getByLabel("Creation date to", { exact: true }).fill("2026-09-23");
  await menu.getByLabel("Creation date to", { exact: true }).press("Tab");
  await menu.getByRole("button", { name: "Apply", exact: true }).click();
  const titles = page.locator("tbody tr td:nth-child(2)");
  await expect(titles).toHaveText(["Item 1", "Item 2", "Item 3"]);
  await searchColumn(page, "Description", "Steel");
  await expect(titles).toHaveText(["Item 1", "Item 3"]);
  await page.getByRole("button", { name: "Clear all filters" }).click();
  await expect(titles).toHaveCount(5);
});

test("user role filter offers distinct values and clears selections", async ({
  page,
}) => {
  await filterFixture(page);
  await page.getByRole("link", { name: "Users", exact: true }).click();
  let menu = await openFilter(page, "Role");
  await menu.getByRole("combobox", { name: "Role filter type" }).press("Space");
  await page
    .getByRole("option", { name: "Selected values", exact: true })
    .click();
  await menu
    .getByRole("combobox", { name: "Select Role values" })
    .press("Space");
  await expect(
    page.getByRole("option", { name: "Member", exact: true }),
  ).toHaveCount(1);
  await page.getByRole("option", { name: "Member", exact: true }).click();
  await page.keyboard.press("Escape");
  await menu.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(page.locator("tbody tr")).toHaveCount(4);
  menu = await openFilter(page, "Role");
  await menu.getByRole("button", { name: "Clear", exact: true }).click();
  await expect(page.locator("tbody tr")).toHaveCount(5);
});
