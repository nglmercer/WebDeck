import type { Button, UsageSnapshot } from '../../lib/contracts';
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

export function placeButtons(buttons: Button[], columns: number, rows: number) {
  const occupied = new Set<number>();
  const placed = new Map<number, { button: Button; columns: number; rows: number }>();
  const ordered = [...buttons].sort(
    (a, b) =>
      Number(typeof (b.extensions.appearance as Record<string, unknown>)?.cell === 'number') -
      Number(typeof (a.extensions.appearance as Record<string, unknown>)?.cell === 'number'),
  );
  let firstFree = 0;
  let count = columns * rows;
  for (const button of ordered) {
    const a = button.extensions.appearance as Record<string, unknown> | undefined;
    const width = Math.floor(appearanceNumber(a?.columns, 1, 1, columns));
    const height = Math.floor(appearanceNumber(a?.rows, 1, 1, 128));
    const fixed = typeof a?.cell === 'number' && Number.isSafeInteger(a.cell) && a.cell >= 0;
    let cell = fixed ? (a!.cell as number) : firstFree;
    const fits = (start: number) => {
      if (
        !Number.isSafeInteger(start) ||
        start > Number.MAX_SAFE_INTEGER - ((height - 1) * columns + width - 1)
      )
        throw new Error('Grid position exceeds the supported integer range.');
      if ((start % columns) + width > columns) return false;
      for (let y = 0; y < height; y++)
        for (let x = 0; x < width; x++) if (occupied.has(start + y * columns + x)) return false;
      return true;
    };
    while (!fits(cell)) cell++;
    for (let y = 0; y < height; y++)
      for (let x = 0; x < width; x++) {
        const index = cell + y * columns + x;
        occupied.add(index);
        count = Math.max(count, index + 1);
      }
    // Keep holes available for narrower buttons; only skip definitely occupied cells.
    while (occupied.has(firstFree)) firstFree++;
    placed.set(cell, { button, columns: width, rows: height });
  }
  return { placed, occupied, count, firstFree };
}
export function gridCells(buttons: Button[], columns: number, rows: number) {
  const { placed, occupied, count } = placeButtons(buttons, columns, rows);
  return Array.from({ length: Math.ceil(count / columns) * columns }, (_, cell) => ({
    cell,
    button: placed.get(cell)?.button,
    columns: placed.get(cell)?.columns ?? 1,
    rows: placed.get(cell)?.rows ?? 1,
    covered: occupied.has(cell) && !placed.has(cell),
  }));
}
export function preserveCells(buttons: Button[], columns: number, rows: number) {
  for (const [cell, { button }] of placeButtons(buttons, columns, rows).placed) {
    button.extensions.appearance = {
      ...((button.extensions.appearance as Record<string, unknown>) ?? {}),
      cell,
    };
  }
}

/** Materializes a bounded row range while keeping every cell's original coordinate. */
export function gridWindow(
  buttons: Button[],
  columns: number,
  rows: number,
  startRow: number,
  rowCount: number,
) {
  if (
    !Number.isSafeInteger(startRow) ||
    startRow < 0 ||
    !Number.isSafeInteger(rowCount) ||
    rowCount < 1 ||
    rowCount > 128
  )
    throw new Error('Invalid grid row range.');
  const start = startRow * columns;
  const placement = placeButtons(buttons, columns, rows);
  const totalRows = Math.ceil(placement.count / columns);
  // The final partial row may end at the last exact coordinate. Do not materialize
  // padding cells beyond that boundary merely to complete the CSS grid row.
  const length = Math.min(
    Math.max(0, Math.min(rowCount, totalRows - startRow)) * columns,
    Math.max(0, Number.MAX_SAFE_INTEGER - start + 1),
  );
  if (
    !Number.isSafeInteger(start) ||
    (length > 0 && start > Number.MAX_SAFE_INTEGER - (length - 1))
  )
    throw new Error('Grid position exceeds the supported integer range.');
  const cells = Array.from({ length }, (_, index) => {
    const cell = start + index,
      item = placement.placed.get(cell);
    return {
      cell,
      button: item?.button,
      columns: item?.columns ?? 1,
      rows: item?.rows ?? 1,
      covered: placement.occupied.has(cell) && !item,
    };
  });
  const crossing = [...placement.placed].filter(
    ([cell, item]) => cell < start && Math.floor(cell / columns) + item.rows > startRow,
  );
  return { cells, crossing, totalRows, startRow };
}

/** Keep the configured columns intact while fitting ordinary decks like master. */
export function fitDeck(
  width: number,
  height: number,
  availableWidth: number,
  availableHeight: number,
) {
  if (
    ![width, height, availableWidth, availableHeight].every(Number.isFinite) ||
    width <= 0 ||
    height <= 0
  )
    return 1;
  return Math.max(0.01, Math.min(availableWidth / width, availableHeight / height));
}
