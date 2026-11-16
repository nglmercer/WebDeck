import { describe, expect, it } from 'vitest';
import { commandSchema, resolve } from '../../lib/schema';
import { registeredIcon } from '../../lib/icons';
import {
  actionIcons,
  actionOptions,
  pickerActionOptions,
  pluginActionOptions,
  pluginArgumentsSchema,
  pluginOptionId,
} from './action-options';
import type { PluginManifest } from '../../lib/contracts';

describe('action icon coverage', () => {
  it('provides a registered icon for every command and button action in the contracts', () => {
    const icons: Record<string, string> = actionIcons;
    for (const schema of [commandSchema, resolve({ $ref: '#/$defs/ButtonAction' })]) {
      for (const variant of schema.oneOf ?? []) {
        const type = String(variant.properties?.type?.const);
        expect(icons[type], type).toBeTruthy();
        expect(registeredIcon(icons[type]!), type).toBeDefined();
      }
    }
    for (const option of actionOptions)
      expect(registeredIcon(option.icon), option.id).toBeDefined();
  });
});

const result = { type: 'object', required: true } as const;
const sonora = {
  schema_version: 2,
  id: 'sonora',
  version: '1.0.0',
  entry: 'index.js',
  backend: 'sandbox_js',
  digest: 'x',
  origin: 'local',
  contract: '',
  actions: [
    {
      id: 'current',
      label: 'Sonora: Current track',
      capabilities: ['read'],
      arguments: {},
      result,
    },
    {
      id: 'volume',
      label: 'Sonora: Set volume',
      capabilities: ['audio'],
      arguments: { percent: { type: 'number', required: true } },
      result,
    },
  ],
} as unknown as PluginManifest;

describe('plugin actions in the picker', () => {
  it('lists every action of a loaded plugin under its own id, with the label as written', () => {
    const options = pluginActionOptions([sonora]);
    expect(options.map((option) => option.id)).toEqual([
      pluginOptionId('sonora', 'current'),
      pluginOptionId('sonora', 'volume'),
    ]);
    expect(options[1]).toMatchObject({
      label: 'Sonora: Set volume',
      detail: 'sonora · 1.0.0',
      category: 'plugin',
      plugin: { pluginId: 'sonora', actionId: 'volume' },
    });
  });

  it('keeps the built-in actions and drops the generic plugin entries', () => {
    const options = pickerActionOptions([sonora]);
    expect(options.some((option) => option.command === 'plugin' && !option.plugin)).toBe(false);
    expect(options.some((option) => option.type === 'plugin')).toBe(false);
    expect(options.some((option) => option.id === 'command:volume')).toBe(true);
    expect(options.filter((option) => option.plugin).length).toBe(2);
  });

  it('offers no plugin entries while no plugin is loaded', () => {
    expect(pluginActionOptions([])).toEqual([]);
    expect(pickerActionOptions([]).some((option) => option.plugin)).toBe(false);
  });

  it('describes the arguments of one action as the form schema', () => {
    expect(pluginArgumentsSchema(sonora, 'volume').properties).toEqual({
      percent: { type: 'number' },
    });
    expect(pluginArgumentsSchema(sonora, 'current').properties).toEqual({});
    expect(pluginArgumentsSchema(sonora, 'missing').properties).toEqual({});
  });
});

describe('declared choices in the form', () => {
  it('turns a list of choices into the enum of the form field', () => {
    const plugin = {
      id: 'sonora',
      version: '1.0.0',
      actions: [
        {
          id: 'search',
          label: 'Search',
          capabilities: ['network'],
          arguments: {
            kind: { type: 'string', required: true, enum: ['song', 'album'] },
          },
          result,
        },
      ],
    } as unknown as PluginManifest;
    expect(pluginArgumentsSchema(plugin, 'search').properties?.kind).toEqual({
      type: 'string',
      enum: ['song', 'album'],
    });
  });
});

describe('declared bounds in the form', () => {
  it('passes the minimum and maximum of a number argument to its form field', () => {
    const plugin = {
      id: 'sonora',
      version: '1.0.0',
      actions: [
        {
          id: 'volume',
          label: 'Set volume',
          capabilities: ['audio'],
          arguments: { percent: { type: 'number', required: true, minimum: 0, maximum: 100 } },
          result,
        },
      ],
    } as unknown as PluginManifest;
    expect(pluginArgumentsSchema(plugin, 'volume').properties?.percent).toEqual({
      type: 'number',
      minimum: 0,
      maximum: 100,
    });
  });
});
