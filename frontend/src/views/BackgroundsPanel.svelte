<script lang="ts">
  import ColorField from '../components/ColorField.svelte';
  import FileField from '../components/FileField.svelte';
  import SectionIcon from '../components/SectionIcon.svelte';
  import TrashIcon from '../components/TrashIcon.svelte';
  import { tx } from '../components/studio/labels';
  import { text } from '../framework/i18n';
  import type { BgEntry } from './config';

  interface Props {
    dark: string;
    backgrounds: BgEntry[];
    trashTitle: string;
  }

  let { dark, backgrounds, trashTitle }: Props = $props();

  const toggleLabel = tx('studio_bg_toggle', 'Toggle background');
</script>

{#snippet bgButtons(entry: BgEntry)}
  <div class="choose-bg-buttons">
    <button
      type="button"
      class={entry.activateCls}
      aria-label={toggleLabel}
      aria-pressed={entry.activateCls.includes('checked') ? 'true' : 'false'}
    >
      <SectionIcon name="check" size={13} />
    </button>
    <TrashIcon title={trashTitle} />
  </div>
{/snippet}

<div class="config-container choose-background {dark}" id="choose-background">
  <div class="wd2-panel-head">
    <h1 class="config-title"> {text('random_bg_menu_title')} </h1>
    <span class="wd2-count" id="bg-count">{backgrounds.length}</span>
  </div>
  <div id="create-bg-choices">
    <div class="wd2-bg-add-row">
      <ColorField
        dark={dark}
        containerClass="background-color-input-container"
        colorClass="background-color-input"
        colorId="background-color-input"
        hexClass="background-color-setting"
        hexId="background-color-hex"
        placeholder="{text('wallpaper_color')} (HEX)"
        value=""
      />
      <button class={dark} id="create-color-bg"> {text('add_background_color')} </button>
    </div>
    <p class="wd2-bg-or"><span>{text('or')}</span></p>
    <FileField
      id="create-image-bg"
      accept="image/jpeg, image/png, image/gif, video/mp4"
      browseLabel={text('select_your_file')}
      emptyLabel={text('no_file_chosen')}
      hint="JPG · PNG · GIF · MP4"
    />
  </div>
  <div id="choose-backgrounds-container">
    {#each backgrounds as entry}
      {#if entry.kind === 'color'}
        <div
          class="choose-bg-element choose-bg-element-color choose-bg-element-pageload {dark}"
          background={entry.bg}
          background_color_text={text('background_color')}
          style="background-color: {entry.stripped};"
        >
          <span class="choose-bg-swatch" style="background-color: {entry.stripped};" aria-hidden="true"></span>
          <div class="choose-bg-foot">
            <span class="choose-bg-label">{text('background_color')} · {entry.stripped}</span>
            {@render bgButtons(entry)}
          </div>
        </div>
      {:else if entry.kind === 'video'}
        <div class="choose-bg-element choose-bg-element-image" background={entry.bg}>
          <div class="choose-bg-thumb">
            <div class="video-container choose-bg-pseudo-element">
              <!-- svelte-ignore a11y_media_has_caption: 1:1 port, decorative ambient background. -->
              <video autoplay muted loop class="blurred-video">
                <source src={entry.src} type="video/mp4" />
                Your browser does not support the video tag...
              </video>
            </div>
            <div class="video-container">
              <!-- svelte-ignore a11y_media_has_caption: 1:1 port, decorative ambient background. -->
              <video autoplay muted loop>
                <source src={entry.src} type="video/mp4" />
                Your browser does not support the video tag...
              </video>
            </div>
          </div>
          <div class="choose-bg-foot">
            <span class="choose-bg-label">{entry.file}</span>
            {@render bgButtons(entry)}
          </div>
        </div>
      {:else}
        <div class="choose-bg-element choose-bg-element-image" background={entry.bg}>
          <div class="choose-bg-thumb">
            <div
              class="choose-bg-pseudo-element"
              style="background-image: url(&quot;.config/user_uploads/{entry.file}&quot;)"
            ></div>
            <img src=".config/user_uploads/{entry.file}" alt="" />
          </div>
          <div class="choose-bg-foot">
            <span class="choose-bg-label">{entry.file}</span>
            {@render bgButtons(entry)}
          </div>
        </div>
      {/if}
    {/each}
  </div>
</div>
