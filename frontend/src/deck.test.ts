import { describe, expect, it } from 'vitest';
import { metricReading, gridCells, preserveCells } from './deck';
import type { Button, UsageSnapshot } from './contracts';
const usage: UsageSnapshot = {
  cpu_percent: 37.5,
  cpus: [{ name: 'CPU 0', usage: 52 }],
  memory_used: 2,
  memory_total: 8,
  disks: [{ name: '/', total: 100, available: 40 }],
  gpus: [{ name: 'GPU', usage: 20, memory_used: 3, memory_total: 4 }],
};
const button = (metric: 'cpu' | 'memory' | 'gpu' | 'gpu_memory' | 'disk', target = ''): Button => ({
  id: 'metric',
  label: 'Metric',
  icon: '',
  color: '',
  extensions: {},
  action: { type: 'metric', metric, target, interval_ms: 1000 },
});
describe('live button metrics', () => {
  it('selects host, per-core, GPU, memory and disk values', () => {
    expect(metricReading(button('cpu'), usage).text).toBe('37.5%');
    expect(metricReading(button('cpu', '0'), usage).text).toBe('52.0%');
    expect(metricReading(button('memory'), usage).percent).toBe(25);
    expect(metricReading(button('gpu', 'GPU'), usage).percent).toBe(20);
    expect(metricReading(button('gpu_memory'), usage).percent).toBe(75);
    expect(metricReading(button('disk', '/'), usage).percent).toBe(60);
  });
  it('distinguishes unavailable devices from zero usage', () => {
    expect(metricReading(button('gpu', 'missing'), usage).text).toBe('Unavailable');
    expect(metricReading(button('cpu'), undefined).text).toBe('—');
    expect(metricReading(button('memory'), { ...usage, memory_total: 0 }).percent).toBe(0);
  });
});

it('keeps empty cells after deletion and reserves explicit positions before new buttons', () => {
  const buttons = [
    button('cpu'),
    { ...button('gpu'), id: 'gpu' },
    { ...button('memory'), id: 'memory' },
  ];
  preserveCells(buttons, 4, 3);
  buttons.splice(1, 1);
  const cells = gridCells(buttons, 4, 3);
  expect(cells).toHaveLength(12);
  expect(cells[1]?.button).toBeUndefined();
  expect(cells[2]?.button?.id).toBe('memory');
  const newButton = { ...button('disk'), id: 'new' };
  expect(gridCells([newButton, ...buttons], 4, 3)[1]?.button?.id).toBe('new');
});
