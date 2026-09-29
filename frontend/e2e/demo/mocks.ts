import { promises as fs } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';

// Live mode (WEBDECK_DEMO_BASE_URL set) records a real server as-is.
const LIVE = Boolean(process.env.WEBDECK_DEMO_BASE_URL);

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.join(HERE, '..', 'fixtures');
const REPO_ROOT = path.join(HERE, '..', '..', '..');

async function readJson(name: string): Promise<unknown> {
  return JSON.parse(await fs.readFile(path.join(FIXTURES, name), 'utf8'));
}

function ok(body: unknown) {
  return { status: 200, contentType: 'application/json', body: JSON.stringify(body) };
}

/**
 * Mock the Rust backend from e2e/fixtures so the shoot is deterministic
 * and triggers no real button actions on the host. No-op in live mode.
 *
 * The mocks are stateful: button saves capture the posted grid and later
 * /api/boot + /get_config calls replay it, so added/renamed/swapped
 * buttons persist across the tour's in-place refreshes like a real server.
 */
export async function setupDemoMocks(page: Page): Promise<void> {
  if (LIVE) return;

  const boot = (await readJson('boot.json')) as {
    config: { front: { buttons: unknown } };
  };
  const usage = await readJson('usage.json');
  let liveButtons: unknown = boot.config.front.buttons;
  const liveConfig = () => ({
    ...boot.config,
    front: { ...boot.config.front, buttons: liveButtons },
  });

  await page.route('**/api/boot', (route) =>
    route.fulfill(ok({ ...boot, config: liveConfig() }))
  );
  // Editor enter uses GET, boot uses POST: same config payload either way.
  await page.route('**/get_config', (route) => route.fulfill(ok(liveConfig())));
  await page.route('**/usage', (route) => route.fulfill(ok(usage)));

  // Button saves capture the posted grid so the next boot replays it.
  for (const endpoint of ['**/save_buttons_only', '**/save_config']) {
    await page.route(endpoint, (route) => {
      try {
        const body = route.request().postDataJSON() as {
          front?: { buttons?: unknown };
        } | null;
        if (body?.front?.buttons !== undefined) liveButtons = body.front.buttons;
      } catch {
        // Non-JSON save body: keep the previous grid.
      }
      return route.fulfill(ok({ success: true, message: 'demo mode' }));
    });
  }
  // Single-button saves patch one slot (folder addressed by index, like the server).
  await page.route('**/save_single_button', (route) => {
    try {
      const body = route.request().postDataJSON() as {
        location_Folder?: number | string;
        location_Id?: number | string;
        content?: unknown;
      } | null;
      const grid = liveButtons as Record<string, unknown[]> | undefined;
      const folder = grid ? Object.keys(grid)[Number(body?.location_Folder)] : undefined;
      const slot = Number(body?.location_Id);
      if (grid && folder !== undefined && body?.content !== undefined && Number.isInteger(slot)) {
        grid[folder]![slot] = body.content;
      }
    } catch {
      // Non-JSON save body: keep the previous grid.
    }
    return route.fulfill(ok({ success: true, message: 'demo mode' }));
  });
  for (const endpoint of ['**/send-data', '**/COMPLETE_save_config', '**/create_folder']) {
    await page.route(endpoint, (route) =>
      route.fulfill(ok({ success: true, message: 'demo mode' }))
    );
  }
  // Vite dev does not serve the Rust-owned /static tree; serve the repo
  // copy so button icons and CSS render in the recording.
  await page.route('**/static/**', (route) => {
    const file = path.join(
      REPO_ROOT,
      new URL(route.request().url()).pathname.replace(/^\/+/, '')
    );
    return route.fulfill({ path: file }).catch(() => route.fulfill({ status: 404 }));
  });
}
