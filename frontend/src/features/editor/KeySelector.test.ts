import { expect, it, vi } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import KeySelector from './KeySelector.svelte';
it('preserves legacy strings on opening and captures only in the focused control', async () => {
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
  const target = document.createElement('div');
  document.body.append(target);
  const onchange = vi.fn();
  const component = mount(KeySelector, { target, props: { value: ['Control', 'c'], onchange } });
  await tick();
  expect(onchange).not.toHaveBeenCalled();
  expect(target.textContent).toContain('F12');
  const button = [...target.querySelectorAll('button')].find(
    (button) => button.textContent === 'Record shortcut',
  )!;
  button.click();
  await tick();
  const outside = vi.fn();
  window.addEventListener('keydown', outside);
  button.dispatchEvent(
    new KeyboardEvent('keydown', {
      key: 'F12',
      code: 'F12',
      ctrlKey: true,
      shiftKey: true,
      bubbles: true,
      cancelable: true,
    }),
  );
  await tick();
  expect(onchange).toHaveBeenCalledWith(['ctrl', 'shift', 'f12']);
  expect(outside).not.toHaveBeenCalled();
  expect(button.getAttribute('aria-pressed')).toBe('false');
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'x' }));
  expect(outside).toHaveBeenCalledTimes(1);
  button.click();
  await tick();
  button.dispatchEvent(
    new KeyboardEvent('keydown', {
      key: 'Escape',
      code: 'Escape',
      bubbles: true,
      cancelable: true,
    }),
  );
  await tick();
  expect(button.getAttribute('aria-pressed')).toBe('false');
  await unmount(component);
  window.removeEventListener('keydown', outside);
  target.remove();
  vi.unstubAllGlobals();
});
