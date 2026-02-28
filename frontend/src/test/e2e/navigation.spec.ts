import { test, expect } from '@playwright/test';

test.describe('Navigation', () => {
  test('home page loads', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('aside')).toBeVisible();
  });

  test('kanban page loads from sidebar', async ({ page }) => {
    await page.goto('/');
    await page.click('text=Kanban Board');
    await expect(page).toHaveURL('/kanban');
  });

  test('analytics page loads from sidebar', async ({ page }) => {
    await page.goto('/');
    await page.click('text=Analytics');
    await expect(page).toHaveURL('/analytics');
    await expect(page.locator('h1', { hasText: 'Analytics' })).toBeVisible();
  });

  test('logs page loads from sidebar', async ({ page }) => {
    await page.goto('/');
    await page.click('text=Logs');
    await expect(page).toHaveURL('/logs');
    await expect(page.locator('h1', { hasText: 'Logs' })).toBeVisible();
  });
});
