import { describe, expect, it } from 'vitest';
import { metricReading, gridCells, preserveCells, gridWindow } from './deck';
import type { Button, UsageSnapshot } from '../../lib/contracts';
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

it('keeps span holes available, resolves collisions, and extends past configured rows', () => {
  const tile = (id: string, appearance: Record<string, unknown>): Button => ({
    ...button('cpu'),
    id,
    extensions: { appearance, custom: { retained: true } },
  });
  const buttons = [
    tile('wide', { columns: 2 }),
    tile('small', {}),
    tile('fixed', { cell: 1, columns: 2, rows: 2 }),
    tile('collision', { cell: 1 }),
    tile('sparse', { cell: 20 }),
  ];
  const before = structuredClone(buttons);
  const cells = gridCells(buttons, 4, 2);
  const positions = Object.fromEntries(
    cells.filter((c) => c.button).map((c) => [c.button!.id, c.cell]),
  );
  expect(positions).toEqual({ fixed: 1, collision: 3, wide: 8, small: 0, sparse: 20 });
  expect(cells).toHaveLength(24);
  expect(cells[5]?.covered).toBe(true);
  expect(buttons).toEqual(before);
  preserveCells(buttons, 4, 2);
  expect(buttons.every((b) => b.extensions.custom)).toBe(true);
  expect(gridCells(buttons, 3, 2).filter((c) => c.button)).toHaveLength(5);
});

it('places a full canonical-size deck without duplicate cells or spreading occupied arguments', () => {
  const buttons = Array.from({ length: 16384 }, (_, i) => ({ ...button('cpu'), id: String(i) }));
  const cells = gridCells(buttons, 16, 4);
  expect(cells).toHaveLength(16384);
  expect(cells.every((c, i) => c.button?.id === String(i))).toBe(true);
});

it('preserves extremely sparse cells without allocating their empty gaps', () => {
  const fixed = button('cpu');
  fixed.extensions = {
    vendor: { retained: true },
    appearance: { cell: 2 ** 40, columns: 2, custom: 7 },
  };
  const implicit = { ...button('memory'), id: 'implicit', extensions: {} };
  const buttons = [fixed, implicit];
  preserveCells(buttons, 4, 3);
  expect(fixed.extensions).toEqual({
    vendor: { retained: true },
    appearance: { cell: 2 ** 40, columns: 2, custom: 7 },
  });
  expect(implicit.extensions).toEqual({ appearance: { cell: 0 } });
  expect(buttons.map((value) => value.id)).toEqual([fixed.id, 'implicit']);
});

it('numeric-limit collisions and spans fail without changing stored placement', () => {
  const first = button('cpu');
  const second = { ...button('memory'), id: 'second' };
  first.extensions = { appearance: { cell: Number.MAX_SAFE_INTEGER } };
  second.extensions = { appearance: { cell: Number.MAX_SAFE_INTEGER } };
  const original = JSON.stringify([first, second]);
  expect(() => preserveCells([first, second], 4, 3)).toThrow('supported integer range');
  expect(JSON.stringify([first, second])).toBe(original);
  first.extensions = { appearance: { cell: Number.MAX_SAFE_INTEGER, rows: 2 } };
  const span = JSON.stringify(first);
  expect(() => preserveCells([first], 4, 3)).toThrow('supported integer range');
  expect(JSON.stringify(first)).toBe(span);
});

it('materializes only a distant row window and retains crossing spans', () => {
  const sparse = button('cpu');
  sparse.extensions.appearance = { cell: 2 ** 40, rows: 3, columns: 2 };
  const window = gridWindow([sparse], 4, 3, 2 ** 38, 16);
  expect(window.cells.length).toBe(12);
  expect(window.cells[0]!.cell).toBe(2 ** 40);
  expect(window.cells[0]!.button?.id).toBe(sparse.id);
  const crossed = gridWindow([sparse], 4, 3, 2 ** 38 + 1, 1);
  expect(crossed.cells.length).toBe(4);
  expect(crossed.crossing[0]![0]).toBe(2 ** 40);
  expect(crossed.cells[0]!.covered).toBe(true);
});

it('clips the final numeric-boundary window before checking coordinate headroom', () => {
  const fixed = button('cpu');
  fixed.extensions.appearance = { cell: Number.MAX_SAFE_INTEGER };
  const row = Math.floor(Number.MAX_SAFE_INTEGER / 4);
  const window = gridWindow([fixed], 4, 3, row, 128);
  expect(window.cells.length).toBe(4);
  expect(window.cells[3]!.cell).toBe(Number.MAX_SAFE_INTEGER);
  expect(window.cells[3]!.button?.id).toBe(fixed.id);
});

it('renders the last supported cell for every canonical column count without unsafe padding', () => {
  const fixed = button('cpu');
  fixed.extensions.appearance = { cell: Number.MAX_SAFE_INTEGER };
  const original = structuredClone(fixed);
  for (let columns = 1; columns <= 128; columns++) {
    const row = Math.floor(Number.MAX_SAFE_INTEGER / columns);
    const window = gridWindow([fixed], columns, 3, row, 128);
    expect(window.cells.length).toBeGreaterThan(0);
    expect(window.cells.length).toBeLessThanOrEqual(columns);
    expect(window.cells.every((cell) => Number.isSafeInteger(cell.cell))).toBe(true);
    expect(window.cells.at(-1)?.cell).toBe(Number.MAX_SAFE_INTEGER);
    expect(window.cells.at(-1)?.button?.id).toBe(fixed.id);
  }
  expect(fixed).toEqual(original);
});

it('fits the master demo proportionally without changing column placement', async () => {
  const { fitDeck } = await import('./deck');
  expect(fitDeck(1036, 620, 1440, 900)).toBeCloseTo(1440 / 1036);
  expect(fitDeck(1036, 620, 360, 900)).toBeCloseTo(360 / 1036);
  expect(fitDeck(1036, 620, 1440, 400)).toBeCloseTo(400 / 620);
  expect(fitDeck(0, 0, 360, 900)).toBe(1);
});
