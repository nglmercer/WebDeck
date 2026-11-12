import { test, expect } from './fixtures';
import AxeBuilder from '@axe-core/playwright';

test('compact connection overview keeps QR and diagnostics off the main page', async ({ page, request }, info) => {
  expect((await request.post('/api/v2/connection/start')).ok()).toBe(true);
  expect((await request.post('/api/v2/devices', { data: { name: 'My phone', capabilities: ['read', 'audio'], ttl_seconds: 2592000 } })).ok()).toBe(true);
  let qrRequests = 0;
  page.on('request', request => { if (request.url().endsWith('/api/v2/network/qr')) qrRequests++; });
  await page.goto('/#settings');
  await page.getByRole('tab', { name: 'Devices & Connections', exact: true }).click();
  const phone = page.locator('.device-card').filter({ hasText: 'My phone' });
  await expect(phone).toBeVisible();
  await expect(page.getByAltText('QR code to open the deck on your phone')).toHaveCount(0);
  expect(qrRequests).toBe(0);
  for (const width of [1440, 390, 360]) {
    await page.setViewportSize({ width, height: 960 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (width === 1440) {
      const devices = await page.locator('#settings-devices').boundingBox();
      const network = await page.locator('.network-card').boundingBox();
      expect(Math.abs(devices!.y - network!.y)).toBeLessThan(2);
      expect(network!.x).toBeGreaterThan(devices!.x + devices!.width);
      expect((await page.locator('.connection-page').boundingBox())!.height).toBeLessThan(850);
    }
    await page.screenshot({ path: info.outputPath(`devices-${width}.png`), fullPage: true });
  }
  const showQr = page.getByRole('button', { name: 'Show QR code', exact: true });
  await showQr.click();
  const qrDialog = page.getByRole('dialog', { name: 'Show QR code', exact: true });
  await expect(qrDialog.getByRole('img')).toBeVisible();
  await expect.poll(() => qrRequests).toBe(1);
  await page.screenshot({ path: info.outputPath('qr-mobile.png'), fullPage: true });
  await page.keyboard.press('Escape');
  await expect(qrDialog).toHaveCount(0);
  await expect(showQr).toBeFocused();
  await page.getByRole('button', { name: 'Connection troubleshooting', exact: true }).click();
  const help = page.getByRole('dialog', { name: 'Connection troubleshooting', exact: true });
  await expect(help).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(help).toHaveCount(0);
  await phone.getByRole('button', { name: 'Manage device', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Manage device', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
  expect(results.violations).toEqual([]);
});
