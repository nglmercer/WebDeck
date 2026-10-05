import { writeFileSync } from 'node:fs';
import { gridCells, gridWindow } from '../../frontend/src/features/deck/deck.ts';

// Run with Node's native TypeScript support (Node 22.18+).
const workloads = [];
for (const count of [48, 1024, 4096, 16384]) {
  const buttons = Array.from({ length: count }, (_, i) => ({
    id: String(i), label: '', icon: '', color: '',
    action: { type: 'none' }, extensions: {},
  }));
  const times = [];
  for (let sample = 0; sample < 5; sample++) {
    const start = performance.now();
    const cells = gridCells(buttons, 16, 4);
    times.push(performance.now() - start);
    if (cells.filter(cell => cell.button).length !== count)
      throw new Error('Placement lost buttons');
  }
  workloads.push({ buttons: count, times_ms: times });
}
const sparseWorkloads = [];
for (const { name, cell, columns, rows, offset } of [
  { name: 'distant anchor', cell: 2 ** 40, columns: 4, rows: 1, offset: 0 },
  { name: 'crossing distant span', cell: 2 ** 40, columns: 4, rows: 128, offset: 1 },
  { name: 'final partial row', cell: Number.MAX_SAFE_INTEGER, columns: 3, rows: 1, offset: 0 },
]) {
  const buttons = [{
    id: name, label: '', icon: '', color: '', action: { type: 'none' },
    extensions: { appearance: { cell, rows } },
  }];
  const original = JSON.stringify(buttons);
  const startRow = Math.floor(cell / columns) + offset;
  const times = [];
  let materializedCells = 0;
  for (let sample = 0; sample < 5; sample++) {
    const start = performance.now();
    const window = gridWindow(buttons, columns, 3, startRow, 128);
    times.push(performance.now() - start);
    materializedCells = window.cells.length;
    const anchors = [...window.crossing.map(([coordinate]) => coordinate),
      ...window.cells.filter(item => item.button).map(item => item.cell)];
    if (anchors.length !== 1 || anchors[0] !== cell || materializedCells > 128 * columns ||
        window.cells.some(item => !Number.isSafeInteger(item.cell)))
      throw new Error(`Sparse window lost or changed placement: ${name}`);
  }
  if (JSON.stringify(buttons) !== original) throw new Error(`Profile mutated source: ${name}`);
  sparseWorkloads.push({ name, cell, columns, rows, start_row: startRow,
    materialized_cells: materializedCells, times_ms: times });
}
const report = {
  profile: `${process.version}; implicit 1x1 buttons; 16 columns; five sequential samples per workload; sparse windows use 128 rows; excludes DOM rendering`,
  workloads,
  sparse_workloads: sparseWorkloads,
};
const json = JSON.stringify(report, null, 2) + '\n';
if (process.env.WEBDECK_GRID_OUTPUT) writeFileSync(process.env.WEBDECK_GRID_OUTPUT, json);
console.log(json);
