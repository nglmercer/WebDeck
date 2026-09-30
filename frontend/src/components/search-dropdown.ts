import { flushSync, mount, unmount } from 'svelte';
import SearchDropdownView from './SearchDropdownView.svelte';

/**
 * Reusable searchable dropdown custom element (`<search-dropdown>`).
 *
 * A search box over a filterable option list: typing filters
 * case-insensitively, click or Enter selects, arrows move the highlight.
 * Selection dispatches {@link SEARCH_DROPDOWN_CHANGE} (bubbles) with the
 * value as `detail`. Light DOM on purpose, so the host's page theme
 * classes (e.g. `dark-theme`) cascade into the list.
 *
 * The element is a thin shell: rendering + interaction state live in the
 * Svelte interior (`SearchDropdownView.svelte`), mounted here. The public
 * API (`value`, `setOptions`, `visibleOptions`, observed attributes) is
 * unchanged, and every mutation is wrapped in `flushSync` so callers keep
 * the synchronous DOM semantics the imperative version had.
 *
 * Styling hooks: `.sd-search` / `.sd-list` / `.sd-option` (+ `.active`,
 * `.selected`, `[aria-selected="true"]`); see
 * `static/css/components/search-dropdown.css`.
 */

/** Selection event name; `detail` is the selected value. */
export const SEARCH_DROPDOWN_CHANGE = 'search-dropdown-change';

/** Imperative API the Svelte interior exposes. */
interface SearchDropdownExports {
  setOptions(options: string[]): void;
  getValue(): string;
  setValue(next: string): void;
  setAttrs(placeholder: string, inputClass: string): void;
  visibleOptions(): string[];
}

export class SearchDropdown extends HTMLElement {
  private app: SearchDropdownExports | null = null;
  // Pre-connect writes are queued: `setOptions`/value assignment must work
  // before `connectedCallback` mounts the interior (as before).
  private pendingOptions: string[] | null = null;
  private pendingValue: string | null = null;

  static get observedAttributes(): string[] {
    return ['placeholder', 'input-class'];
  }

  /** Currently selected value ('' when nothing is selected). */
  get value(): string {
    return this.app?.getValue() ?? this.pendingValue ?? '';
  }

  set value(next: string) {
    if (this.app) {
      const app = this.app;
      flushSync(() => app.setValue(next));
    } else {
      this.pendingValue = next;
    }
  }

  /** Replace the option list (selection kept when still present). */
  setOptions(options: string[], synchronous = true): void {
    if (this.app) {
      const app = this.app;
      if (synchronous) flushSync(() => app.setOptions(options));
      else app.setOptions(options);
    } else {
      this.pendingOptions = [...options];
    }
  }

  /** Visible option values, in order. */
  visibleOptions(): string[] {
    return this.app?.visibleOptions() ?? [];
  }

  connectedCallback(): void {
    if (this.app) return;
    const app = mount(SearchDropdownView, {
      target: this,
      props: {
        hostId: this.id,
        placeholder: this.getAttribute('placeholder') ?? '',
        inputClass: this.getAttribute('input-class') ?? '',
        onSelect: (value: string) => {
          this.dispatchEvent(
            new CustomEvent<string>(SEARCH_DROPDOWN_CHANGE, { detail: value, bubbles: true })
          );
        },
      },
    }) as unknown as SearchDropdownExports;
    this.app = app;
    // Skip the flush when there is nothing queued: this element upgrades
    // inside App's subtree (a nested mount sharing the outer batch), and
    // a gratuitous flushSync would settle + null that batch before the
    // outer mount's boundary resolves (transfer_effects crash).
    if (this.pendingOptions === null && this.pendingValue === null) return;
    flushSync(() => {
      if (this.pendingOptions) {
        app.setOptions(this.pendingOptions);
        this.pendingOptions = null;
      }
      if (this.pendingValue !== null) {
        app.setValue(this.pendingValue);
        this.pendingValue = null;
      }
    });
  }

  disconnectedCallback(): void {
    if (!this.app) return;
    this.pendingValue = this.app.getValue();
    const app = this.app;
    this.app = null;
    void unmount(app as unknown as Record<string, never>);
  }

  attributeChangedCallback(name: string): void {
    if ((name === 'placeholder' || name === 'input-class') && this.app) {
      const app = this.app;
      flushSync(() =>
        app.setAttrs(
          this.getAttribute('placeholder') ?? '',
          this.getAttribute('input-class') ?? ''
        )
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

/** Mount options + selection listener; no-op when the element is absent. */
export function wireSearchDropdown(
  id: string,
  options: string[],
  onSelect: (value: string) => void,
  synchronous = true
): () => void {
  const el = document.getElementById(id);
  if (!(el instanceof SearchDropdown)) return () => {};
  el.setOptions(options, synchronous);
  const listener = (event: Event): void => onSelect((event as CustomEvent<string>).detail);
  el.addEventListener(SEARCH_DROPDOWN_CHANGE, listener);
  return () => el.removeEventListener(SEARCH_DROPDOWN_CHANGE, listener);
}
