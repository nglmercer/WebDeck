import { UsageMonitor } from './usage.svelte';
import type { Button } from '../../lib/contracts';

export function monitorHarness(buttons: Button[]) {
  let monitor!: UsageMonitor;
  const dispose = $effect.root(() => {
    monitor = new UsageMonitor(
      () => buttons,
      () => true,
    );
  });
  return { monitor, dispose };
}
