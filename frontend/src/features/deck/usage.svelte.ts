import type { Button, UsageResponse } from '../../lib/contracts';
import { request } from '../../lib/api/client';
import { untrack } from 'svelte';

/** Polls only the visible deck's metrics and owns its request/timer lifecycle. */
export class UsageMonitor {
  value = $state<UsageResponse | null>(null);
  now = $state(Date.now());
  status = $state<'idle' | 'loading' | 'live' | 'stale'>('idle');

  constructor(buttons: () => Button[], visible: () => boolean) {
    $effect(() => {
      const metrics = buttons().filter(
        (button) => button.action.type === 'usage' || button.action.type === 'metric',
      );
      if (!visible() || !metrics.length) return;
      const interval = Math.min(
        ...metrics.map((button) =>
          button.action.type === 'metric' ? button.action.interval_ms : 1000,
        ),
      );
      const needsUsage = metrics.some(
        (button) => button.action.type !== 'metric' || button.action.metric !== 'clock',
      );
      const controller = new AbortController();
      let inFlight = false;
      const update = async () => {
        this.now = Date.now();
        if (inFlight || !needsUsage || document.hidden) return;
        inFlight = true;
        try {
          await this.refresh(controller.signal);
        } finally {
          inFlight = false;
        }
      };
      // Refresh reads/writes monitor state; it must not become a setup dependency.
      untrack(() => void update());
      const timer = setInterval(() => {
        void update();
      }, interval);
      document.addEventListener('visibilitychange', update);
      return () => {
        controller.abort();
        clearInterval(timer);
        document.removeEventListener('visibilitychange', update);
      };
    });
  }

  async refresh(signal?: AbortSignal) {
    if (!this.value) this.status = 'loading';
    try {
      const result = await request<UsageResponse>('usage', 'UsageResponse', {
        signal: signal
          ? AbortSignal.any([signal, AbortSignal.timeout(5000)])
          : AbortSignal.timeout(5000),
      });
      if (signal?.aborted) return;
      this.value = result;
      this.status = 'live';
    } catch {
      if (!signal?.aborted) this.status = 'stale';
    }
  }
}
