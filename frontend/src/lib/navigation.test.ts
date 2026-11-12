import { beforeEach, expect, it, vi } from 'vitest';
import { Navigation } from './navigation.svelte';
import initial from '../../../webdeck/config_default.json';
import type { Layout } from './contracts';
const layout = initial.layout as Layout;
beforeEach(() => history.replaceState(null, '', '/'));
it('opens allowlisted settings sections and tutorials after host authorization', () => {
  const section = vi.fn(),
    tutorials = vi.fn();
  const navigation = new Navigation(
    () => layout,
    () => true,
    { section, tutorials },
  );
  for (const target of ['appearance', 'integrations', 'devices', 'backups', 'runtime']) {
    history.replaceState(null, '', `/?view=settings&section=${target}`);
    navigation.route();
    expect(navigation.panel).toBe('settings');
    expect(section).toHaveBeenLastCalledWith(target);
  }
  history.replaceState(null, '', '/?view=settings&section=tutorials');
  navigation.route();
  expect(tutorials).toHaveBeenCalledOnce();
  expect(section).toHaveBeenLastCalledWith('appearance');
  history.replaceState(null, '', '/?view=settings&section=unknown');
  navigation.route();
  expect(section).toHaveBeenLastCalledWith('appearance');
});
it('cannot expose local settings or tutorials to a paired viewer', () => {
  const section = vi.fn(),
    tutorials = vi.fn();
  const navigation = new Navigation(
    () => layout,
    () => false,
    { section, tutorials },
  );
  history.replaceState(null, '', '/?view=settings&section=tutorials');
  navigation.route();
  expect(navigation.panel).toBe('deck');
  expect(section).not.toHaveBeenCalled();
  expect(tutorials).not.toHaveBeenCalled();
});
it('preserves folder hashes and removes consumed settings parameters when navigating', () => {
  const navigation = new Navigation(
    () => layout,
    () => true,
  );
  const folder = layout.folders[0]!.id;
  history.replaceState(
    null,
    '',
    `/?view=settings&section=tutorials&theme=custom#folder=${encodeURIComponent(folder)}`,
  );
  navigation.route();
  expect(navigation.panel).toBe('deck');
  navigation.navigate(folder);
  expect(location.search).toBe('?theme=custom');
  expect(location.hash).toBe(`#folder=${encodeURIComponent(folder)}`);
  navigation.settings();
  expect(location.hash).toBe('#settings');
  navigation.route();
  expect(navigation.panel).toBe('settings');
  navigation.back();
  navigation.route();
  expect(navigation.panel).toBe('deck');
});
