import { afterEach, expect, it, vi } from 'vitest';
import { Execution } from './execution.svelte';
import type { Button, CommandEvent } from '../../lib/contracts';
const button: Button = {
  id: 'test',
  label: 'Test',
  icon: '',
  color: '',
  extensions: {},
  action: { type: 'command', command: { type: 'debug', data: {} } },
};
const completed: CommandEvent = {
  api_version: 2,
  request_id: 'test',
  state: 'completed',
  result: {},
};
afterEach(() => vi.useRealTimers());
it('disposal clears feedback timers and refuses later execution', async () => {
  vi.useFakeTimers();
  const runner = vi.fn(async () => completed);
  const execution = new Execution(runner);
  await execution.run(button, 'http');
  expect(execution.states.test).toBe('completed');
  expect(vi.getTimerCount()).toBe(1);
  execution.dispose();
  expect(vi.getTimerCount()).toBe(0);
  await execution.run(button, 'http');
  expect(runner).toHaveBeenCalledTimes(1);
});
it('pending completion after disposal cannot schedule feedback or replay work', async () => {
  vi.useFakeTimers();
  let finish!: (event: CommandEvent) => void;
  const runner = vi.fn(
    () =>
      new Promise<CommandEvent>((resolve) => {
        finish = resolve;
      }),
  );
  const execution = new Execution(runner);
  const pending = execution.run(button, 'socket');
  await execution.run(button, 'socket');
  execution.dispose();
  finish(completed);
  await pending;
  expect(runner).toHaveBeenCalledTimes(1);
  expect(execution.states.test).toBeUndefined();
  expect(vi.getTimerCount()).toBe(0);
});
