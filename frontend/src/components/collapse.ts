import { html, raw, type Html } from '../framework/html';
import { q } from '../query';

/**
 * Collapsible section built on native `<details>/<summary>`: no JS needed
 * to open/close, keyboard accessible, and form controls inside still
 * serialize (used for the settings groups; reusable anywhere).
 */
/** Optional tutorial/help link pinned to the section summary. */
export interface CollapseInfo {
  href: string;
  title: string;
  icon: Html;
}

export interface CollapseOptions {
  /** Unique id; also the localStorage persistence key suffix. */
  id: string;
  /** Already-translated section title. */
  title: string;
  /** Section body markup. */
  body: Html;
  /** Markup default when nothing is persisted (default: closed). */
  open?: boolean;
  extraClass?: string | undefined;
  info?: CollapseInfo;
}

export function collapseSection(opts: CollapseOptions): Html {
  const info =
    opts.info !== undefined
      ? html`<a class="wd-collapse-info" href="${opts.info.href}" target="_blank" title="${opts.info.title}">${opts.info.icon}</a>`
      : raw('');
  return html`<details class="wd-collapse${opts.extraClass ? raw(` ${opts.extraClass}`) : raw('')}" data-collapse="${opts.id}"${opts.open ? raw(' open') : raw('')}>
    <summary class="wd-collapse-summary"><span class="wd-collapse-chevron" aria-hidden="true"></span><span class="wd-collapse-title">${opts.title}</span>${info}</summary>
    <div class="wd-collapse-body">${opts.body}</div>
  </details>`;
}

/** localStorage prefix for persisted open states. */
export const COLLAPSE_STORAGE_PREFIX = 'webdeck:collapse:';

function readCollapseState(id: string): boolean | null {
  try {
    const value = window.localStorage.getItem(COLLAPSE_STORAGE_PREFIX + id);
    if (value === '1') return true;
    if (value === '0') return false;
    return null;
  } catch {
    return null;
  }
}

function writeCollapseState(id: string, open: boolean): void {
  try {
    window.localStorage.setItem(COLLAPSE_STORAGE_PREFIX + id, open ? '1' : '0');
  } catch {
    // Private mode / file:// — persistence is best-effort.
  }
}

/**
 * Restore persisted open states and persist future toggles. Safe to call
 * when no collapsible sections exist; `scope` limits wiring to a subtree.
 */
export function wireCollapses(scope?: ParentNode): void {
  const root: ParentNode = scope ?? document;
  root.querySelectorAll('details[data-collapse]').forEach((el) => {
    const details = el as HTMLDetailsElement;
    const id = details.getAttribute('data-collapse') ?? '';
    if (id === '') return;
    const stored = readCollapseState(id);
    if (stored !== null) details.open = stored;
    q(details).on('toggle', () => writeCollapseState(id, details.open));
    // The info link lives inside <summary>: cancel the toggle and open
    // the tutorial explicitly (preventDefault alone would kill both).
    q(details)
      .find('a.wd-collapse-info')
      .toArray()
      .forEach((link) => {
        q(link).on('click', function (event) {
          event.preventDefault();
          const href = q(link).attr('href') ?? '';
          if (href !== '') window.open(href, '_blank');
        });
      });
  });
}
