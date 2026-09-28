<script lang="ts">
  import SvgSlot from '../components/SvgSlot.svelte';
  import { text } from '../framework/i18n';
  import type { ThemeEntry } from './config';

  interface Props {
    dark: string;
    disabled: ThemeEntry[];
    enabled: ThemeEntry[];
    infoSlot: number;
  }

  let { dark, disabled, enabled, infoSlot }: Props = $props();

  function openThemesFolder(): void {
    window.send_data?.('/openfolder /.config/themes');
  }
</script>

<div class="config-container choose-themes {dark}" id="choose-themes" style="display: none;">
  <!-- NOTE: the lone "<" below mirrors an upstream typo (renders as text). -->
  {'<'}
  <button type="button" id="setting-themes-back" class="button {dark}">
    {text('back')}
  </button>
  <div id="themes-tutorial-container">
    <a href={text('link_themes')} target="_blank" title={text('themes_tutorial')}>
      <SvgSlot slot={infoSlot} />
    </a>
    <button type="button" onclick={openThemesFolder} class="button" id="open-themes-folder">
      {text('open_themes_folder')}
    </button>
  </div>
  <h1 class="config-title"> {text('themes_menu_title')} </h1>
  <div class="editorStyle-bar {dark}"></div>
  <div id="choose-themes-titles">
    <h2 class="disabled-themes-title"> {text('disabled_themes')} </h2>
    <h2 class="enabled-themes-title"> {text('enabled_themes')} </h2>
  </div>
  <div id="choose-themes-container">
    <div id="disabled-themes">
      {#each disabled as entry}
        <div class="theme-container" filename={entry.theme}>
          <span class="enable-theme-hitbox"></span>
          <span class="enable-theme invisible"></span>
          <div class="arrows-container invisible">
            <span class="arrow-up-hitbox"></span>
            <span class="arrow-up"></span>
            <span class="arrow-down-hitbox"></span>
            <span class="arrow-down"></span>
          </div>
          <div class="theme-icon-container">
            {#if entry.icon !== ''}
              <!-- svelte-ignore a11y_missing_attribute: 1:1 port, decorative theme icon. -->
              <img class="theme-icon" src={entry.icon} />
            {/if}
          </div>
          <div class="theme-texts">
            <div class="theme-title">
              {entry.title}
            </div>
            <div class="theme-desc">
              {entry.desc}
            </div>
          </div>
        </div>
      {/each}
    </div>
    <div id="enabled-themes">
      {#each enabled as entry}
        <div class="theme-container" filename={entry.theme} defaulttheme={entry.isDefault ? '' : undefined}>
          {#if !entry.isDefault}
            <span class="disable-theme-hitbox"></span>
            <span class="disable-theme invisible"></span>
            <div class="arrows-container invisible">
              <span class="arrow-up-hitbox"></span>
              <span class="arrow-up"></span>
              <span class="arrow-down-hitbox"></span>
              <span class="arrow-down"></span>
            </div>
          {/if}
          <div class="theme-icon-container">
            {#if entry.icon !== ''}
              <!-- svelte-ignore a11y_missing_attribute: 1:1 port, decorative theme icon. -->
              <img class="theme-icon" src={entry.icon} />
            {/if}
          </div>
          <div class="theme-texts">
            <div class="theme-title">
              {entry.title}
            </div>
            <div class="theme-desc">
              {entry.desc}
            </div>
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>
