import { describe, expect, it } from 'vitest';
import type { JsonObject } from '../framework/types';
import { resolveLanguage } from './config';

function langs(): JsonObject[] {
  return [{ code: 'en_US' }, { code: 'es_ES' }, { code: 'fr_FR' }];
}

describe('resolveLanguage', () => {
  it('matches exact, short, and sibling locales', () => {
    expect(resolveLanguage('es_ES', langs())).toBe('es_ES');
    expect(resolveLanguage('ES_es', langs())).toBe('es_ES');
    expect(resolveLanguage('es', langs())).toBe('es_ES');
    expect(resolveLanguage('es_PE', langs())).toBe('es_ES');
    expect(resolveLanguage('es-PE', langs())).toBe('es_ES');
  });

  it('falls back to en_US without a linguistic match', () => {
    expect(resolveLanguage('xx_YY', langs())).toBe('en_US');
    expect(resolveLanguage('', langs())).toBe('en_US');
  });
});
