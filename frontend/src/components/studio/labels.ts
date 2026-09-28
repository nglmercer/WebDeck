import { text } from '../../framework/i18n';

/**
 * Translated label with an English fallback. New studio chrome uses keys
 * that older language packs may not define yet; when the lookup misses
 * (returns the query or ''), the fallback renders instead of the raw key.
 */
export function tx(key: string, fallback: string): string {
  const value = text(key);
  return value === key || value === '' ? fallback : value;
}
