import { describe, expect, it } from 'vitest';
import catalog from '../../../contracts/catalog.json';
import canonical from '../../../contracts/v2.schema.json';
import compact from './generated-schema';
import { contract } from './schema';

describe('server response contracts', () => {
  it('keeps the compact browser schema identical to the canonical definitions', () => {
    expect(compact.$defs).toEqual(canonical.$defs);
  });
  it('accepts the generated command catalog', () => {
    const response = { api_version: 2, commands: catalog, plugins: [] };
    expect(contract('CatalogResponse', response)).toEqual(response);
  });
});
