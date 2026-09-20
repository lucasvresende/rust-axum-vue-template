import { test, expect } from '@playwright/test'
test('login dashboard is usable', async ({ page }) => { await page.goto('/'); await expect(page.getByText('Axum + Vue')).toBeVisible(); await page.getByRole('button', { name: 'Sign in' }).click(); await expect(page.getByText('Good to see you')).toBeVisible(); await page.screenshot({ path: '../artifacts/dashboard.png', fullPage: true }); })
