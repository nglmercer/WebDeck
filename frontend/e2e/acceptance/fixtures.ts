import { test as base } from '@playwright/test';
import { rmSync } from 'node:fs';
import {
  createAcceptanceHost,
  reservePort,
  waitForBoot,
} from '../../../tools/validation/acceptance-host.mjs';
import { stopChild } from '../../../tools/validation/child-process.mjs';
export * from '@playwright/test';

// Every test owns its host, including pairing limits, sessions, uploads and config.
export const test = base.extend({
  baseURL: async ({}, use) => {
    const port = await reservePort();
    const host = createAcceptanceHost(port);

    // Surface a startup crash immediately instead of reporting 133 identical
    // setup failures after the readiness poll times out. Same diagnostics path
    // as the CI preflight, so a locally reproduced crash is easy to compare.
    let ready = false;
    const exited = new Promise<never>((_, reject) => {
      host.child.once('error', () =>
        reject(Error(host.diagnostics('Acceptance host failed to spawn'))),
      );
      host.child.once('exit', () => {
        if (!ready) reject(Error(host.diagnostics()));
      });
    });
    // Readiness normally wins the race; swallow a later exit so it is not an
    // unhandled rejection while the test is using the host.
    exited.catch(() => {});

    try {
      await Promise.race([waitForBoot(host, 30000), exited]);
      ready = true;
      await use(host.url);
    } finally {
      await stopChild(host.child);
      rmSync(host.directory, { recursive: true, force: true });
    }
  },
});
