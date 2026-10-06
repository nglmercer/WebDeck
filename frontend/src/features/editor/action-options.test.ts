import { describe, expect, it } from 'vitest';
import { commandSchema, resolve } from '../../lib/schema';
import { registeredIcon } from '../../lib/icons';
import { actionIcons, actionOptions } from './action-options';

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
