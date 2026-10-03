import type { Button, UsageSnapshot } from './contracts';
export type Reading = { text: string; percent?: number };
export function metricReading(
  button: Button,
  usage: UsageSnapshot | undefined,
  now = Date.now(),
): Reading {
  const action = button.action;
  const metric = action.type === 'metric' ? action.metric : 'cpu';
  const target = action.type === 'metric' ? action.target : '';
  if (metric === 'clock')
    return { text: new Date(now).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) };
  if (!usage) return { text: '—' };
  const percentage = (used: number, total: number) => (total > 0 ? (100 * used) / total : 0);
  let value: number | undefined;
  if (metric === 'cpu')
    value = target
      ? usage.cpus.find((c, i) => c.name === target || String(i) === target)?.usage
      : usage.cpu_percent;
  else if (metric === 'memory') value = percentage(usage.memory_used, usage.memory_total);
  else if (metric === 'gpu' || metric === 'gpu_memory') {
    const gpu = target
      ? usage.gpus.find((g, i) => g.name === target || String(i) === target)
      : usage.gpus[0];
    if (gpu) value = metric === 'gpu' ? gpu.usage : percentage(gpu.memory_used, gpu.memory_total);
  } else if (metric === 'disk') {
    const disk = target
      ? usage.disks.find((d, i) => d.name === target || String(i) === target)
      : usage.disks[0];
    if (disk) value = percentage(disk.total - disk.available, disk.total);
  }
  if (value === undefined || !Number.isFinite(value)) return { text: 'Unavailable' };
  value = Math.max(0, Math.min(100, value));
  return { text: `${value.toFixed(1)}%`, percent: value };
}
export function appearanceNumber(
  value: unknown,
  fallback: number,
  min: number,
  max: number,
): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.max(min, Math.min(max, value))
    : fallback;
}

export function gridCells(buttons: Button[], columns: number, rows: number) {
  const occupied = new Set<number>();
  const placed = new Map<number, { button: Button; columns: number; rows: number }>();
  const ordered = [...buttons].sort(
    (a, b) =>
      Number(typeof (b.extensions.appearance as Record<string, unknown>)?.cell === 'number') -
      Number(typeof (a.extensions.appearance as Record<string, unknown>)?.cell === 'number'),
  );
  for (const button of ordered) {
    const a = button.extensions.appearance as Record<string, unknown> | undefined;
    const width = Math.floor(appearanceNumber(a?.columns, 1, 1, columns));
    const height = Math.floor(appearanceNumber(a?.rows, 1, 1, 128));
    let cell = typeof a?.cell === 'number' && Number.isInteger(a.cell) && a.cell >= 0 ? a.cell : 0;
    const footprint = (start: number) =>
      Array.from({ length: height }, (_, y) =>
        Array.from({ length: width }, (_, x) => start + y * columns + x),
      ).flat();
    while ((cell % columns) + width > columns || footprint(cell).some((i) => occupied.has(i)))
      cell++;
    for (const i of footprint(cell)) occupied.add(i);
    placed.set(cell, { button, columns: width, rows: height });
  }
  const count = Math.max(columns * rows, ...[...occupied].map((i) => i + 1));
  return Array.from({ length: Math.ceil(count / columns) * columns }, (_, cell) => ({
    cell,
    button: placed.get(cell)?.button,
    columns: placed.get(cell)?.columns ?? 1,
    rows: placed.get(cell)?.rows ?? 1,
    covered: occupied.has(cell) && !placed.has(cell),
  }));
}
export function preserveCells(buttons: Button[], columns: number, rows: number) {
  for (const { cell, button } of gridCells(buttons, columns, rows)) {
    if (button)
      button.extensions.appearance = {
        ...((button.extensions.appearance as Record<string, unknown>) ?? {}),
        cell,
      };
  }
}
