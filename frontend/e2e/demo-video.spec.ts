import { promises as fs } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { injectCursor } from './demo/cursor';
import { installEventRecorder } from './demo/events';
import { setupDemoMocks } from './demo/mocks';
import {
  addButtonStep,
  bootStep,
  configLibraryStep,
  folderNavStep,
  renameButtonStep,
  saveExitStep,
  swapStep,
} from './demo/steps';

// Full-tour demo recording (720p). One step per tour section; edit
// ./demo/steps.ts to reorder/extend the tour, ./demo/mocks.ts for the
// backend stubs, ./demo/actions.ts for cursor motion, ./demo/cursor.ts
// for the rendered pointer.
const HERE = path.dirname(fileURLToPath(import.meta.url));
const DEMO_VIDEO = path.join(HERE, '..', 'demo', 'webdeck-demo-720p.webm');
const DEMO_DIR = path.dirname(DEMO_VIDEO);

test.setTimeout(300_000);
test('webdeck demo tour', async ({ page }) => {
  // Save flows use alert()/confirm(); accept so the tour never stalls.
  page.on('dialog', (dialog) => void dialog.accept());
  await injectCursor(page);
  await installEventRecorder(page);
  await setupDemoMocks(page);

  await bootStep(page);
  await folderNavStep(page);
  await configLibraryStep(page);
  await renameButtonStep(page);
  await addButtonStep(page);
  await swapStep(page);
  await saveExitStep(page);

  // Publish the recording at a stable path.
  const video = page.video();
  expect(video).not.toBeNull();
  await page.close();
  await fs.mkdir(DEMO_DIR, { recursive: true });
  await video?.saveAs(DEMO_VIDEO);
});
