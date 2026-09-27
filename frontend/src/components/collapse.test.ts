import { beforeEach, describe, expect, it, vi } from 'vitest';
import { html } from '../framework/html';
import { COLLAPSE_STORAGE_PREFIX, collapseSection, wireCollapses } from './collapse';

beforeEach(() => {
  document.body.innerHTML = '';
  window.localStorage.clear();
});

describe('collapseSection', () => {
  it('renders a details/summary group with title and body', () => {
    const out = collapseSection({
      id: 'general',
      title: 'General',
      body: html`<p>hello</p>`,
    }).value;
    expect(out).toContain('<details');
    expect(out).toContain('data-collapse="general"');
    expect(out).toContain('<summary');
    expect(out).toContain('General');
    expect(out).toContain('<p>hello</p>');
    expect(out).not.toContain(' open');
  });

  it('marks default-open sections and escapes the title', () => {
    const out = collapseSection({
      id: 'x',
      title: '<b>evil</b>',
      body: html``,
      open: true,
    }).value;
    expect(out).toContain(' open');
    expect(out).toContain('&lt;b&gt;evil&lt;/b&gt;');
    expect(out).not.toContain('<b>evil</b>');
  });
});

describe('wireCollapses', () => {
  function mount(openAttr: boolean): HTMLDetailsElement {
    document.body.innerHTML = collapseSection({
      id: 'general',
      title: 'General',
      body: html`<input type="text" name="a" />`,
      open: openAttr,
    }).value;
    const details = document.body.querySelector('details');
    if (!details) throw new Error('details missing');
    return details as HTMLDetailsElement;
  }

  it('restores a persisted open state over the markup default', () => {
    window.localStorage.setItem(`${COLLAPSE_STORAGE_PREFIX}general`, '1');
    const details = mount(false);
    wireCollapses();
    expect(details.open).toBe(true);
  });

  it('restores a persisted closed state over the markup default', () => {
    window.localStorage.setItem(`${COLLAPSE_STORAGE_PREFIX}general`, '0');
    const details = mount(true);
    wireCollapses();
    expect(details.open).toBe(false);
  });

  it('keeps the markup default when nothing is persisted', () => {
    const details = mount(true);
    wireCollapses();
    expect(details.open).toBe(true);
  });

  it('persists toggles to localStorage', () => {
    const details = mount(false);
    wireCollapses();
    details.open = true;
    details.dispatchEvent(new Event('toggle'));
    expect(window.localStorage.getItem(`${COLLAPSE_STORAGE_PREFIX}general`)).toBe('1');
    details.open = false;
    details.dispatchEvent(new Event('toggle'));
    expect(window.localStorage.getItem(`${COLLAPSE_STORAGE_PREFIX}general`)).toBe('0');
  });

  it('scopes wiring to a subtree when given one', () => {
    window.localStorage.setItem(`${COLLAPSE_STORAGE_PREFIX}general`, '1');
    const details = mount(false);
    const other = document.createElement('div');
    wireCollapses(other);
    expect(details.open).toBe(false);
  });

  it('renders an info link that opens without toggling', () => {
    document.body.innerHTML = collapseSection({
      id: 'soundboard',
      title: 'Soundboard',
      body: html``,
      info: { href: 'https://example.com/docs', title: 'Tutorial', icon: html`<svg></svg>` },
    }).value;
    const link = document.body.querySelector('a.wd-collapse-info');
    expect(link?.getAttribute('href')).toBe('https://example.com/docs');
    wireCollapses();
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    try {
      const click = new MouseEvent('click', { bubbles: true, cancelable: true });
      link?.dispatchEvent(click);
      expect(click.defaultPrevented).toBe(true);
      expect(open).toHaveBeenCalledWith('https://example.com/docs', '_blank');
    } finally {
      open.mockRestore();
    }
  });
});
