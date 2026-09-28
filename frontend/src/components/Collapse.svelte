<script lang="ts" module>
  /** localStorage prefix for persisted open states. */
  export const COLLAPSE_STORAGE_PREFIX = 'webdeck:collapse:';
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import SvgSlot from './SvgSlot.svelte';

  /**
   * Collapsible section built on native `<details>/<summary>`: no JS needed
   * to open/close, keyboard accessible, and form controls inside still
   * serialize (used for the settings groups; reusable anywhere).
   */

  interface CollapseInfo {
    href: string;
    title: string;
    iconSlot: number;
  }

  interface Props {
    /** Unique id; also the localStorage persistence key suffix. */
    id: string;
    /** Already-translated section title. */
    title: string;
    /** Markup default when nothing is persisted (default: closed). */
    open?: boolean | undefined;
    extraClass?: string | undefined;
    info?: CollapseInfo | undefined;
    children?: Snippet | undefined;
  }

  let { id, title, open = false, extraClass, info, children }: Props = $props();
  let detailsEl: HTMLDetailsElement | null = $state(null);

  $effect(() => {
    const details = detailsEl;
    if (!details) return;
    try {
      const stored = window.localStorage.getItem(COLLAPSE_STORAGE_PREFIX + id);
      if (stored === '1') details.open = true;
      else if (stored === '0') details.open = false;
    } catch {
      // Private mode / file:// — persistence is best-effort.
    }
    const onToggle = (): void => {
      try {
        window.localStorage.setItem(COLLAPSE_STORAGE_PREFIX + id, details.open ? '1' : '0');
      } catch {
        // Best-effort (see above).
      }
    };
    details.addEventListener('toggle', onToggle);
    // The info link lives inside <summary>: cancel the toggle and open
    // the tutorial explicitly (preventDefault alone would kill both).
    const link = details.querySelector('a.wd-collapse-info');
    const onLinkClick = (event: Event): void => {
      event.preventDefault();
      const href = link?.getAttribute('href') ?? '';
      if (href !== '') window.open(href, '_blank');
    };
    link?.addEventListener('click', onLinkClick);
    return () => {
      details.removeEventListener('toggle', onToggle);
      link?.removeEventListener('click', onLinkClick);
    };
  });
</script>

<details
  class="wd-collapse{extraClass ? ` ${extraClass}` : ''}"
  data-collapse={id}
  {open}
  bind:this={detailsEl}
>
  <summary class="wd-collapse-summary"><span class="wd-collapse-chevron" aria-hidden="true"></span><span class="wd-collapse-title">{title}</span>{#if info !== undefined}<a class="wd-collapse-info" href={info.href} target="_blank" title={info.title}><SvgSlot slot={info.iconSlot} /></a>{/if}</summary>
  <div class="wd-collapse-body">{#if children !== undefined}{@render children()}{/if}</div>
</details>
