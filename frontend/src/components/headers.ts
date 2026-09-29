/** One `Name: value` header row (shared by the editor widget and prefill). */

export interface HeaderRow {
  name: string;
  value: string;
}

/**
 * Parse `Name: value` lines into rows. Blank lines are skipped; a line
 * without a colon keeps its text as a valueless name so it round-trips
 * visibly instead of vanishing (the backend still rejects it on send,
 * with a line number).
 */
export function parseHeaderRows(text: string): HeaderRow[] {
  const rows: HeaderRow[] = [];
  for (const line of text.split('\n')) {
    const trimmed = line.trim();
    if (trimmed === '') continue;
    const colon = trimmed.indexOf(':');
    if (colon === -1) {
      rows.push({ name: trimmed, value: '' });
    } else {
      rows.push({ name: trimmed.slice(0, colon).trim(), value: trimmed.slice(colon + 1).trim() });
    }
  }
  return rows;
}

/**
 * Join rows back to `Name: value` lines. Rows with a blank name are
 * skipped (the editor's trailing blank row never submits); a blank
 * value serializes bare so malformed rows round-trip.
 */
export function serializeHeaderRows(rows: HeaderRow[]): string {
  return rows
    .filter((row) => row.name.trim() !== '')
    .map((row) => (row.value === '' ? row.name.trim() : `${row.name.trim()}: ${row.value}`))
    .join('\n');
}
