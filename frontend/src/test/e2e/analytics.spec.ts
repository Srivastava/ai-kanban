import { test, expect } from '@playwright/test';

test.describe('Analytics page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/analytics');
  });

  test('shows overview card labels', async ({ page }) => {
    await expect(page.locator('text=Total Tokens')).toBeVisible();
    await expect(page.locator('text=Estimated Cost')).toBeVisible();
  });

  test('shows Token Usage Over Time chart section', async ({ page }) => {
    await expect(page.locator('text=Token Usage Over Time')).toBeVisible();
  });

  test('time toggle buttons are visible', async ({ page }) => {
    await expect(page.locator('button', { hasText: 'Daily' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Weekly' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Monthly' })).toBeVisible();
  });

  test('clicking Weekly toggle works without error', async ({ page }) => {
    await page.click('button:has-text("Weekly")');
    // Verify no crash — page still has the chart heading
    await expect(page.locator('text=Token Usage Over Time')).toBeVisible();
  });

  test('shows tool breakdown chart section', async ({ page }) => {
    await expect(page.locator('text=Tokens per Tool Call')).toBeVisible();
  });

  test('session timeline has session dropdown', async ({ page }) => {
    await expect(page.locator('select')).toBeVisible();
    await expect(page.locator('text=Session Token Timeline')).toBeVisible();
  });
});
