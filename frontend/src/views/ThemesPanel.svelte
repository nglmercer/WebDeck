<script lang="ts">
  import SvgSlot from '../components/SvgSlot.svelte';
  import { text } from '../framework/i18n';
  import type { ThemeMeta } from './config';

  interface Props {
    dark: string;
    /** Ordered theme list (`//` prefix = disabled); bound to Config state. */
    themes: string[];
    /** Card metadata by stripped filename. */
    themeMeta: Record<string, ThemeMeta>;
    infoSlot: number;
  }

  let { dark, themes = $bindable(), themeMeta, infoSlot }: Props = $props();

  const DEFAULT_THEME = 'static/css/style.css';
  const metaFallback: ThemeMeta = { icon: '', title: '', desc: '' };

  const disabled = $derived(themes.filter((t) => t.startsWith('//')));
  const enabled = $derived(themes.filter((t) => !t.startsWith('//')));

  /** Hovered row (drives the `invisible` arrow toggling). */
  let hovered = $state<string | null>(null);

  function metaOf(theme: string): ThemeMeta {
    return themeMeta[theme.replace('//', '')] ?? metaFallback;
  }

  /** Enable/disable every matching entry (moved to the front, like before). */
  function toggleTheme(theme: string): void {
    const path = theme.replace('//', '');
    const next = [...themes];
    for (let i = 0; i < next.length; i++) {
      if ((next[i] ?? '').replace('//', '') === path) {
        const toggled = (next[i] ?? '').startsWith('//')
          ? (next[i] ?? '').replace('//', '')
          : `//${next[i] ?? ''}`;
        next.splice(i, 1);
        next.unshift(toggled);
      }
    }
    themes = next;
  }

  /** Swap with the same-list neighbor (never across or past the default theme). */
  function moveTheme(theme: string, dir: -1 | 1): void {
    const list = theme.startsWith('//') ? disabled : enabled;
    const at = list.indexOf(theme);
    if (at === -1) return;
    const other = list[at + dir];
    if (other === undefined || other === DEFAULT_THEME) return;
    const ai = themes.indexOf(theme);
    const bi = themes.indexOf(other);
    const next = [...themes];
    const temp = next[ai] as string;
    next[ai] = next[bi] as string;
    next[bi] = temp;
    themes = next;
  }

  /** Boundary markers for the row arrows (same rules as before). */
  function arrowCls(theme: string, dir: -1 | 1): string {
    const list = theme.startsWith('//') ? disabled : enabled;
    const at = list.indexOf(theme);
    const base = dir === -1 ? 'arrow-up-hitbox' : 'arrow-down-hitbox';
    const blocked =
      dir === -1
        ? at <= 0 || list[0] === DEFAULT_THEME
        : at === -1 || at >= list.length - 1 || list[list.length - 1] === DEFAULT_THEME;
    return blocked ? `${base} disabled` : base;
  }

  function openThemesFolder(): void {
    window.send_data?.('/openfolder /.config/themes');
  }
</script>

{#snippet themeRow(theme: string, isEnabled: boolean)}
  {@const meta = metaOf(theme)}
  {@const isDefault = theme === DEFAULT_THEME}
  <div
    class="theme-container"
    filename={theme}
    defaulttheme={isDefault ? '' : undefined}
    onmouseenter={() => (hovered = theme)}
    onmouseleave={() => (hovered = null)}
  >
    {#if !isDefault}
      <span
        class={isEnabled ? 'disable-theme-hitbox' : 'enable-theme-hitbox'}
        onclick={() => toggleTheme(theme)}
      ></span>
      <span class={(isEnabled ? 'disable-theme' : 'enable-theme') + (hovered === theme ? '' : ' invisible')}></span>
      <div class={'arrows-container' + (isEnabled && hovered === theme ? '' : ' invisible')}>
        <span class={arrowCls(theme, -1)} onclick={() => moveTheme(theme, -1)}></span>
        <span class="arrow-up"></span>
        <span class={arrowCls(theme, 1)} onclick={() => moveTheme(theme, 1)}></span>
        <span class="arrow-down"></span>
      </div>
    {/if}
    <div class="theme-icon-container">
      {#if meta.icon !== ''}
        <!-- svelte-ignore a11y_missing_attribute: 1:1 port, decorative theme icon. -->
        <img class="theme-icon" src={meta.icon} />
      {/if}
    </div>
    <div class="theme-texts">
      <div class="theme-title">
        {meta.title}
      </div>
      <div class="theme-desc">
        {meta.desc}
      </div>
    </div>
  </div>
{/snippet}

<div class="config-container choose-themes {dark}" id="choose-themes">
  <div class="wd2-panel-head">
    <h1 class="config-title"> {text('themes_menu_title')} </h1>
    <a class="wd2-tutorial" href={text('link_themes')} target="_blank" title={text('themes_tutorial')}>
      <SvgSlot slot={infoSlot} />
    </a>
    <button type="button" onclick={openThemesFolder} class="button" id="open-themes-folder">
      {text('open_themes_folder')}
    </button>
  </div>
  <div id="choose-themes-titles">
    <h2 class="disabled-themes-title"> {text('disabled_themes')} </h2>
    <h2 class="enabled-themes-title"> {text('enabled_themes')} </h2>
  </div>
  <div id="choose-themes-container">
    <div id="disabled-themes">
      {#each disabled as theme (theme)}
        {@render themeRow(theme, false)}
      {/each}
    </div>
    <div id="enabled-themes">
      {#each enabled as theme (theme)}
        {@render themeRow(theme, true)}
      {/each}
    </div>
  </div>
</div>
