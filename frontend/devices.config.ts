import { defineConfig, devices } from '@playwright/test';
import base from './acceptance.config';
export default defineConfig({
  ...base,
  testMatch: '**/experience.spec.ts',
  reporter: [['list'], ['html', { outputFolder: 'device-report', open: 'never' }]],
  use: { baseURL: 'http://127.0.0.1:59996', headless: true, screenshot: 'only-on-failure', trace: 'retain-on-failure' },
  projects: [
    { name: 'android-chromium', use: { ...devices['Pixel 7'], launchOptions: base.use?.launchOptions } },
    { name: 'tablet-chromium', use: { ...devices['iPad Mini'], browserName: 'chromium', launchOptions: base.use?.launchOptions } },
    { name: 'iphone-webkit', use: { ...devices['iPhone 13'] } },
    { name: 'ipad-webkit', use: { ...devices['iPad Mini'] } },
    { name: 'desktop-firefox', use: { ...devices['Desktop Firefox'] } },
  ],
});
