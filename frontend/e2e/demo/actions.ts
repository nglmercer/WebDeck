import type { Page } from '@playwright/test';

/** Visible click: glide the (rendered) cursor to the target, then click. */
export async function vclick(page: Page, selector: string): Promise<void> {
  const loc = page.locator(selector).first();
  await loc.waitFor({ state: 'visible' });
  await loc.scrollIntoViewIfNeeded();
  const box = await loc.boundingBox();
  if (!box) {
    await loc.click();
    return;
  }
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y, { steps: 12 });
  await page.waitForTimeout(150);
  await page.mouse.click(x, y);
}

/** Dismiss the save-result alert (cursor glide reads on video). */
export async function dismissAlert(page: Page): Promise<void> {
  await vclick(page, '[data-testid="alert-ok"]');
  await page.locator('[data-testid="alert-ok"]').waitFor({ state: 'detached' });
}

/** Glide the cursor to a field, then fill it (cursor stays on screen). */
export async function vfill(page: Page, selector: string, value: string): Promise<void> {
  const loc = page.locator(selector).first();
  await loc.waitFor({ state: 'visible' });
  await loc.scrollIntoViewIfNeeded();
  const box = await loc.boundingBox();
  if (box) {
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 12 });
    await page.waitForTimeout(150);
  }
  await loc.fill(value);
}
