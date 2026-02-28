import { test, expect } from '@playwright/test';

test.describe('Logs page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/logs');
  });

  test('shows filter bar', async ({ page }) => {
    await expect(page.locator('button', { hasText: 'ALL' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'DEBUG' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'ERROR' })).toBeVisible();
  });

  test('shows source filter', async ({ page }) => {
    await expect(page.locator('button', { hasText: 'Frontend' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Backend' })).toBeVisible();
  });

  test('shows live toggle', async ({ page }) => {
    await expect(page.locator('text=Live')).toBeVisible();
  });

  test('search input is present', async ({ page }) => {
    await expect(page.locator('input[placeholder*="Search"]')).toBeVisible();
  });
});
