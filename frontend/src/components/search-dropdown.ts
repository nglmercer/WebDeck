import { html, raw, type Html } from '../framework/html';

/**
 * Reusable searchable dropdown custom element (`<search-dropdown>`).
 *
 * A search box over a filterable option list: typing filters
 * case-insensitively, click or Enter selects, arrows move the highlight.
 * Selection dispatches {@link SEARCH_DROPDOWN_CHANGE} (bubbles) with the
 * value as `detail`. Light DOM on purpose, so the host's page theme
 * classes (e.g. `dark-theme`) cascade into the list.
 *
 * Styling hooks: `.sd-search` / `.sd-list` / `.sd-option` (+ `.active`,
 * `.selected`, `[aria-selected="true"]`); see
 * `static/css/components/search-dropdown.css`.
 */

/** Selection event name; `detail` is the selected value. */
export const SEARCH_DROPDOWN_CHANGE = 'search-dropdown-change';

export class SearchDropdown extends HTMLElement {
  private options: string[] = [];
  private selected: string | null = null;
  private activeValue: string | null = null;
  private search: HTMLInputElement | null = null;
  private list: HTMLDivElement | null = null;

  static get observedAttributes(): string[] {
    return ['placeholder', 'input-class'];
  }

  /** Currently selected value ('' when nothing is selected). */
  get value(): string {
    return this.selected ?? '';
  }

  set value(next: string) {
    if (next !== '' && !this.options.includes(next)) return;
    this.selected = next === '' ? null : next;
    this.activeValue = this.selected;
    this.paint();
  }

  /** Replace the option list (selection kept when still present). */
  setOptions(options: string[]): void {
    this.options = [...options];
    if (this.selected !== null && !this.options.includes(this.selected)) {
      this.selected = null;
    }
    this.activeValue = null;
    this.renderOptions();
  }

  /** Visible option values, in order. */
  visibleOptions(): string[] {
    const needle = (this.search?.value ?? '').toLowerCase();
    return this.options.filter((option) => option.toLowerCase().includes(needle));
  }

  connectedCallback(): void {
    if (this.search) return;
    const search = document.createElement('input');
    search.type = 'text';
    search.className = 'sd-search';
    search.setAttribute('autocomplete', 'off');
    search.setAttribute('role', 'combobox');
    search.setAttribute('aria-expanded', 'true');
    search.setAttribute('aria-autocomplete', 'list');
    const list = document.createElement('div');
    list.className = 'sd-list';
    list.setAttribute('role', 'listbox');
    const listId = `${this.id || 'sd'}-listbox`;
    list.id = listId;
    search.setAttribute('aria-controls', listId);
    this.append(search, list);
    this.search = search;
    this.list = list;
    this.applySearchAttrs();
    search.addEventListener('input', () => {
      this.activeValue = null;
      this.renderOptions();
    });
    search.addEventListener('keydown', (event) => this.onSearchKey(event as KeyboardEvent));
    list.addEventListener('click', (event) => {
      const option = (event.target as Element).closest?.('.sd-option') ?? null;
      const value = option?.getAttribute('data-value');
      if (value !== null && value !== undefined) this.select(value, true);
    });
    this.renderOptions();
  }

  attributeChangedCallback(name: string): void {
    if (name === 'placeholder' || name === 'input-class') this.applySearchAttrs();
  }

  private applySearchAttrs(): void {
    const search = this.search;
    if (!search) return;
    search.placeholder = this.getAttribute('placeholder') ?? '';
    search.className = ['sd-search', this.getAttribute('input-class')]
      .filter((cls) => cls !== null && cls !== '')
      .join(' ');
  }

  private onSearchKey(event: KeyboardEvent): void {
    const visible = this.visibleOptions();
    if (event.key === 'Enter') {
      const first = visible[0];
      if (first !== undefined) this.select(first, true);
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (visible.length === 0) return;
      const at = visible.indexOf(this.activeValue ?? '');
      const next =
        event.key === 'ArrowDown'
          ? visible[(at + 1) % visible.length]
          : visible[(at - 1 + visible.length) % visible.length];
      this.activeValue = next ?? null;
      this.paint();
      for (const el of this.list?.querySelectorAll('.sd-option') ?? []) {
        if (el.getAttribute('data-value') === this.activeValue) {
          el.scrollIntoView?.({ block: 'nearest' });
          break;
        }
      }
    } else if (event.key === 'Escape') {
      if (this.search) this.search.value = '';
      this.activeValue = null;
      this.renderOptions();
    }
  }

  private renderOptions(): void {
    const list = this.list;
    if (!list) return;
    const visible = new Set(this.visibleOptions());
    list.replaceChildren(
      ...this.options
        .filter((option) => visible.has(option))
        .map((option) => {
          const el = document.createElement('div');
          el.className = 'sd-option';
          el.setAttribute('role', 'option');
          el.textContent = option;
          el.setAttribute('data-value', option);
          return el;
        })
    );
    this.paint();
  }

  private paint(): void {
    for (const el of this.list?.querySelectorAll('.sd-option') ?? []) {
      const value = el.getAttribute('data-value');
      const selected = value !== null && value === this.selected;
      el.classList.toggle('selected', selected);
      el.classList.toggle('active', value !== null && value === this.activeValue && !selected);
      el.setAttribute('aria-selected', selected ? 'true' : 'false');
    }
  }

  private select(value: string, emit: boolean): void {
    this.selected = value;
    this.activeValue = value;
    this.paint();
    if (emit) {
      this.dispatchEvent(
        new CustomEvent<string>(SEARCH_DROPDOWN_CHANGE, { detail: value, bubbles: true })
      );
    }
  }
}

export function defineSearchDropdown(): void {
  if (!customElements.get('search-dropdown')) {
    customElements.define('search-dropdown', SearchDropdown);
  }
}

defineSearchDropdown();

export interface SearchDropdownOptions {
  dark: string;
  /** Host id (unique per use site). */
  id: string;
  placeholder: string;
  /** Extra class(es) for the internal search input (e.g. `key-aux`). */
  inputClass?: string;
}

export function searchDropdown(o: SearchDropdownOptions): Html {
  const extra = o.inputClass ? html` input-class="${o.inputClass}"` : raw('');
  return html`<search-dropdown class="${raw(o.dark)}" id="${o.id}" placeholder="${o.placeholder}"${extra}></search-dropdown>`;
}

/** Mount options + selection listener; no-op when the element is absent. */
export function wireSearchDropdown(
  id: string,
  options: string[],
  onSelect: (value: string) => void
): void {
  const el = document.getElementById(id);
  if (!(el instanceof SearchDropdown)) return;
  el.setOptions(options);
  el.addEventListener(SEARCH_DROPDOWN_CHANGE, (event) => {
    onSelect((event as CustomEvent<string>).detail);
  });
}
