import { defineConfig } from '@playwright/test';
import { existsSync } from 'node:fs';
const executablePath = process.env.WEBDECK_CHROMIUM ?? (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined);
export default defineConfig({
  testDir:'./e2e/acceptance', timeout:45000, workers:1, fullyParallel:false,
  use:{baseURL:'http://127.0.0.1:59996', headless:true, trace:'retain-on-failure', launchOptions:{...(executablePath ? {executablePath} : {}), args:['--no-sandbox']}},
  webServer:{command:'node e2e/acceptance/server.mjs',url:'http://127.0.0.1:59996/api/boot',reuseExistingServer:false,timeout:30000},
});
