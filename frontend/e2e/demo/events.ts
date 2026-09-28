import type { Page } from '@playwright/test';

// Records the app lifecycle events (src/app/events.ts) into
// window.__webdeckEvents so the tour can wait on real state transitions
// (save finished, refresh rendered) instead of inferring from the DOM
// alone. Installed via addInitScript, so it survives page reloads.
const RECORDER_SCRIPT = `(() => {
  const types = [
    'boot:ready',
    'app:refreshed',
    'usage:updated',
    'editor:changed',
    'save:completed',
    'server:disconnected',
    'server:reconnected',
  ];
  const log = (window.__webdeckEvents = window.__webdeckEvents || []);
  for (const name of types) {
    window.addEventListener('webdeck:' + name, () => {
      log.push({ name, at: Date.now() });
    });
  }
})();`;

interface WindowWithEventLog {
  __webdeckEvents?: Array<{ name: string; at: number }>;
}

export async function installEventRecorder(page: Page): Promise<void> {
  await page.addInitScript({ content: RECORDER_SCRIPT });
}

/** Current log length: capture before an action, pass as fromIndex after. */
export async function appEventCount(page: Page): Promise<number> {
  return page.evaluate(
    () => (window as unknown as WindowWithEventLog).__webdeckEvents?.length ?? 0
  );
}

/** Resolve once `name` is recorded at or after fromIndex. */
export async function waitForAppEvent(
  page: Page,
  name: string,
  fromIndex = 0,
  timeout = 15_000
): Promise<void> {
  await page.waitForFunction(
    ({ name, fromIndex }) =>
      ((window as unknown as WindowWithEventLog).__webdeckEvents ?? [])
        .slice(fromIndex)
        .some((entry) => entry.name === name),
    { name, fromIndex },
    { timeout }
  );
}
