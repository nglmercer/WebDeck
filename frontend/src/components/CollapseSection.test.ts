import type { ComponentProps } from 'svelte';
import { createRawSnippet, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Collapse from './Collapse.svelte';

// Mirrors Collapse.svelte's module export (tsc can't see .svelte named
// exports); the restore/persist tests below fail if the two ever diverge.
const COLLAPSE_STORAGE_PREFIX = 'webdeck:collapse:';

function body(html: string): ReturnType<typeof createRawSnippet> {
  return createRawSnippet(() => ({ render: () => html }));
}

describe('Collapse', () => {
  let host: HTMLElement | null = null;
  let app: Record<string, never> | null = null;

  beforeEach(() => {
    document.body.innerHTML = '';
    window.localStorage.clear();
  });

  afterEach(async () => {
    if (app) {
      await unmount(app);
      app = null;
    }
    host?.remove();
    host = null;
    document.body.innerHTML = '';
  });

  async function render(
    props: Omit<ComponentProps<typeof Collapse>, 'children'>,
    children = body('<input type="text" name="a" />')
  ): Promise<HTMLElement> {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(Collapse, { target: host, props: { ...props, children } }) as unknown as Record<
      string,
      never
    >;
    await tick();
    return host;
  }

  function details(host: HTMLElement): HTMLDetailsElement {
    const el = host.querySelector('details');
    if (!el) throw new Error('details missing');
    return el as HTMLDetailsElement;
  }

  it('renders a details/summary group with title and body', async () => {
    const el = await render({ id: 'general', title: 'General' }, body('<p>hello</p>'));
    const d = details(el);
    expect(d.getAttribute('data-collapse')).toBe('general');
    expect(d.querySelector('summary')).not.toBeNull();
    expect(d.querySelector('.wd-collapse-title')?.textContent).toBe('General');
    expect(d.querySelector('.wd-collapse-body p')?.textContent).toBe('hello');
    expect(d.open).toBe(false);
  });

  it('renders without a body when no snippet is passed', async () => {
    host = document.createElement('div');
    document.body.appendChild(host);
    app = mount(Collapse, {
      target: host,
      props: { id: 'x', title: 'X' },
    }) as unknown as Record<string, never>;
    await tick();
    expect(host.querySelector('.wd-collapse-body')).not.toBeNull();
  });

  it('marks default-open sections and escapes the title', async () => {
    const el = await render({ id: 'x', title: '<b>evil</b>', open: true }, body('<span></span>'));
    expect(details(el).open).toBe(true);
    expect(el.innerHTML).toContain('&lt;b&gt;evil&lt;/b&gt;');
    expect(el.querySelector('.wd-collapse-title b')).toBeNull();
  });

  it('restores a persisted open state over the markup default', async () => {
    window.localStorage.setItem(`${COLLAPSE_STORAGE_PREFIX}general`, '1');
    const el = await render({ id: 'general', title: 'General' });
    expect(details(el).open).toBe(true);
  });

  it('restores a persisted closed state over the markup default', async () => {
    window.localStorage.setItem(`${COLLAPSE_STORAGE_PREFIX}general`, '0');
    const el = await render({ id: 'general', title: 'General', open: true });
    expect(details(el).open).toBe(false);
  });

  it('keeps the markup default when nothing is persisted', async () => {
    const el = await render({ id: 'general', title: 'General', open: true });
    expect(details(el).open).toBe(true);
  });

  it('persists toggles to localStorage', async () => {
    const el = await render({ id: 'general', title: 'General' });
    const d = details(el);
    d.open = true;
    d.dispatchEvent(new Event('toggle'));
    expect(window.localStorage.getItem(`${COLLAPSE_STORAGE_PREFIX}general`)).toBe('1');
    d.open = false;
    d.dispatchEvent(new Event('toggle'));
    expect(window.localStorage.getItem(`${COLLAPSE_STORAGE_PREFIX}general`)).toBe('0');
  });

  it('renders a distinct leading icon with the chevron last', async () => {
    const el = await render({ id: 'soundboard', title: 'Soundboard', icon: 'speaker' });
    const summary = el.querySelector('summary')!;
    expect(summary.querySelector('.wd-collapse-icon svg')).not.toBeNull();
    const order = Array.from(summary.children).map((c) => c.className);
    expect(order[0]).toContain('wd-collapse-icon');
    expect(order[order.length - 1]).toContain('wd-collapse-chevron');
  });

  it('renders an info link that opens without toggling', async () => {
    const el = await render({
      id: 'soundboard',
      title: 'Soundboard',
      info: { href: 'https://example.com/docs', title: 'Tutorial', iconSlot: 7 },
    });
    const link = el.querySelector('a.wd-collapse-info') as HTMLAnchorElement;
    expect(link.getAttribute('href')).toBe('https://example.com/docs');
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    try {
      const click = new MouseEvent('click', { bubbles: true, cancelable: true });
      link.dispatchEvent(click);
      expect(click.defaultPrevented).toBe(true);
      expect(open).toHaveBeenCalledWith('https://example.com/docs', '_blank');
    } finally {
      open.mockRestore();
    }
  });
});
