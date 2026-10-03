import { describe, expect, it } from 'vitest';
import catalog from '../../../contracts/catalog.json';
import { contract } from './schema';

describe('server response contracts', () => {
  it('accepts the generated command catalog', () => {
    const response = { api_version: 2, commands: catalog, plugins: [] };
    expect(contract('CatalogResponse', response)).toEqual(response);
  });
});
