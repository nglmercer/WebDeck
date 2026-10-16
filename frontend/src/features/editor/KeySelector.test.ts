import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import KeySelector from './KeySelector.svelte';
import KeySelectorHarness from '../../../test-support/KeySelectorHarness.svelte';

const mounted: Array<() => Promise<void>> = [];

beforeEach(() => {
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            catalog_version: 1,
            platform: 'linux_x11',
            availability: 'desktop_access_unverified',
            fn_status: 'unsupported',
          }),
        ),
    ),
  );
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({ matches: true })),
  );
});

afterEach(async () => {
  for (const cleanup of mounted.splice(0)) await cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

async function settle() {
  await tick();
  await Promise.resolve();
  await tick();
}

function button(target: HTMLElement, label: string): HTMLButtonElement {
  const found = [...target.querySelectorAll('button')].find(
    (element) => element.textContent?.trim() === label,
  );
  if (!found) throw new Error(`Missing button: ${label}`);
  return found;
}

function renderSelector(value: string[] = ['Control', 'c'], onchange = vi.fn()) {
  const target = document.createElement('div');
  document.body.append(target);
  const onMobileStep = vi.fn();
  const component = mount(KeySelector, {
    target,
    props: { value, onchange, onMobileStep },
  });
  mounted.push(async () => {
    await unmount(component);
    target.remove();
  });
  return { target, onchange, onMobileStep };
}

function renderHarness(onchange = vi.fn()) {
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(KeySelectorHarness, { target, props: { onchange } });
  mounted.push(async () => {
    await unmount(component);
    target.remove();
  });
  return { target, onchange };
}

function keydown(target: HTMLElement, init: KeyboardEventInit) {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  return event;
}

async function openPicker(target: HTMLElement) {
  button(target, 'Choose keys').click();
  await settle();
  return target.querySelector<HTMLElement>('.picker')!;
}

it('shows legacy aliases without normalizing or committing, and restores focus after cancel', async () => {
  const { target, onchange, onMobileStep } = renderSelector(['Control', 'c']);
  await settle();
  expect(target.querySelector('output')?.textContent).toBe('Ctrl + C');
  expect(onchange).not.toHaveBeenCalled();

  const trigger = button(target, 'Choose keys');
  const panel = await openPicker(target);
  expect(panel.textContent).toContain('Ctrl');
  expect(panel.textContent).toContain('C');
  expect(onMobileStep).toHaveBeenCalledWith(true);
  button(panel, 'Use shortcut').click();
  await settle();
  expect(onchange).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(trigger);
  expect(target.querySelector('.picker')).toBeNull();
});

it('records Ctrl+Shift+F12 once from the focused target and ignores unrelated keys', async () => {
  const { target, onchange } = renderSelector();
  await settle();
  const record = button(target, 'Record shortcut');
  record.click();
  await settle();
  expect(record.getAttribute('aria-pressed')).toBe('true');
  expect(target.textContent).toContain('Press your shortcut now');

  keydown(record, { key: 'Control', code: 'ControlLeft' });
  keydown(record, { key: 'Shift', code: 'ShiftLeft', ctrlKey: true });
  keydown(record, {
    key: 'F12',
    code: 'F12',
    ctrlKey: true,
    shiftKey: true,
  });
  await settle();
  expect(onchange).toHaveBeenCalledTimes(1);
  expect(onchange).toHaveBeenCalledWith(['ctrl', 'shift', 'f12']);
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(target.textContent).toContain('Shortcut captured in this browser.');

  const outside = vi.fn();
  window.addEventListener('keydown', outside);
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'x' }));
  expect(outside).toHaveBeenCalledTimes(1);
  window.removeEventListener('keydown', outside);
});

it('does not commit modifier-only, repeated, IME or AltGraph events', async () => {
  const { target, onchange } = renderSelector();
  await settle();
  const record = button(target, 'Record shortcut');
  record.click();
  await settle();
  keydown(record, { key: 'Control', code: 'ControlLeft' });
  keydown(record, { key: 'x', code: 'KeyX', ctrlKey: true, repeat: true });
  keydown(record, { key: 'x', code: 'KeyX', ctrlKey: true, isComposing: true });
  expect(onchange).not.toHaveBeenCalled();
  expect(record.getAttribute('aria-pressed')).toBe('true');

  keydown(record, { key: 'AltGraph', code: 'AltRight', ctrlKey: true, altKey: true });
  await settle();
  expect(onchange).not.toHaveBeenCalled();
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(target.textContent).toContain('AltGr cannot be reproduced reliably');

  record.click();
  await settle();
  keydown(record, { key: '+', code: 'NumpadAdd' });
  await settle();
  expect(onchange).not.toHaveBeenCalled();
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(target.textContent).toContain('This key cannot be recorded here');
});

it('rejects capture when the reported host does not support keyboard input', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            catalog_version: 1,
            platform: 'unsupported',
            availability: 'unsupported',
            fn_status: 'unsupported',
          }),
        ),
    ),
  );
  const { target, onchange } = renderSelector();
  await settle();
  const record = button(target, 'Record shortcut');
  record.click();
  await settle();
  keydown(record, { key: 'F12', code: 'F12' });
  await settle();
  expect(onchange).not.toHaveBeenCalled();
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(target.textContent).toContain('This key cannot be recorded here');
});

it('cancels on Escape or blur without changing configured keys', async () => {
  const { target, onchange } = renderSelector();
  await settle();
  const record = button(target, 'Record shortcut');
  record.click();
  await settle();
  const escape = keydown(record, { key: 'Escape', code: 'Escape' });
  await settle();
  expect(escape.defaultPrevented).toBe(true);
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(onchange).not.toHaveBeenCalled();

  record.click();
  await settle();
  record.blur();
  await settle();
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(onchange).not.toHaveBeenCalled();
});

it('stops recording at timeout and reports the timeout locally', async () => {
  vi.useFakeTimers();
  const { target, onchange } = renderSelector();
  await settle();
  const record = button(target, 'Record shortcut');
  record.click();
  await settle();
  await vi.advanceTimersByTimeAsync(10000);
  await settle();
  expect(record.getAttribute('aria-pressed')).toBe('false');
  expect(target.textContent).toContain('Recording timed out. Try again.');
  expect(onchange).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
});

it('keeps manual selection transactional and Clear disables Use shortcut', async () => {
  const { target, onchange } = renderSelector();
  await settle();
  const panel = await openPicker(target);
  button(panel, 'Clear selection').click();
  await settle();
  expect(button(panel, 'Use shortcut').disabled).toBe(true);
  expect(panel.textContent).toContain('Choose a primary key to continue');
  expect(onchange).not.toHaveBeenCalled();
  button(panel, 'Cancel').click();
  await settle();
  expect(onchange).not.toHaveBeenCalled();
  expect(target.querySelector('output')?.textContent).toBe('Ctrl + C');
});

it('replaces the main key, keeps modifiers, and commits only on Use shortcut', async () => {
  const { target, onchange } = renderSelector(['ctrl', 'c']);
  await settle();
  const panel = await openPicker(target);
  const grid = panel.querySelector('.key-grid')!;
  const f1 = [...grid.querySelectorAll('button')].find(
    (item) => item.textContent?.trim() === 'F1',
  )!;
  const f24 = [...grid.querySelectorAll('button')].find(
    (item) => item.textContent?.trim() === 'F24',
  )!;
  expect(f24).toBeTruthy();
  f1.click();
  await settle();
  f24.click();
  await settle();
  expect(onchange).not.toHaveBeenCalled();
  button(panel, 'Use shortcut').click();
  await settle();
  expect(onchange).toHaveBeenCalledTimes(1);
  expect(onchange).toHaveBeenCalledWith(['ctrl', 'f24']);
});

it('preserves legacy simultaneous chords in explicit advanced mode', async () => {
  const { target, onchange } = renderSelector(['ctrl', 'f1', 'c']);
  await settle();
  const panel = await openPicker(target);
  const advanced = panel.querySelector<HTMLInputElement>('.advanced input')!;
  expect(advanced.checked).toBe(true);
  expect(panel.textContent).toContain('Sequences such as Ctrl+K');
  expect(onchange).not.toHaveBeenCalled();
  button(panel, 'Cancel').click();
  await settle();
  expect(onchange).not.toHaveBeenCalled();
});

it('supports Escape dismissal and restores focus from the in-dialog picker step', async () => {
  const { target, onchange } = renderSelector();
  await settle();
  const trigger = button(target, 'Choose keys');
  const panel = await openPicker(target);
  const search = panel.querySelector<HTMLInputElement>('input[type="search"]')!;
  keydown(search, { key: 'Escape', code: 'Escape' });
  await settle();
  expect(target.querySelector('.picker')).toBeNull();
  expect(document.activeElement).toBe(trigger);
  expect(onchange).not.toHaveBeenCalled();
});

it('does not clear selection when search changes and discards stale candidates after an external update', async () => {
  const { target, onchange } = renderHarness();
  await settle();
  const panel = await openPicker(target);
  button(panel, 'Letters & numbers').click();
  await settle();
  expect(panel.textContent).toContain('Ctrl');
  expect(panel.textContent).toContain('C');
  const search = panel.querySelector<HTMLInputElement>('input[type="search"]')!;
  search.value = 'F24';
  search.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
  expect(panel.textContent).toContain('Ctrl');
  expect(panel.textContent).toContain('C');
  expect(onchange).not.toHaveBeenCalled();

  target.querySelector<HTMLButtonElement>('[data-testid="external-update"]')!.click();
  await settle();
  expect(target.querySelector('.picker')).toBeNull();
  expect(target.querySelector('output')?.textContent).toBe('Alt + F24');
  expect(onchange).not.toHaveBeenCalled();
});

it('cleans the recording timeout when the selector unmounts', async () => {
  vi.useFakeTimers();
  const { target } = renderSelector();
  await settle();
  button(target, 'Record shortcut').click();
  await settle();
  expect(vi.getTimerCount()).toBe(1);
  await mounted.pop()!();
  expect(vi.getTimerCount()).toBe(0);
});

it('cancels the picker when its editor tab becomes inactive', async () => {
  const { target, onchange } = renderHarness();
  await settle();
  await openPicker(target);
  target.querySelector<HTMLButtonElement>('[data-testid="toggle-active"]')!.click();
  await settle();
  expect(target.querySelector('.picker')).toBeNull();
  expect(onchange).not.toHaveBeenCalled();
});
