<script lang="ts">
  import ColorField from '../components/ColorField.svelte';
  import TrashIcon from '../components/TrashIcon.svelte';
  import { text } from '../framework/i18n';
  import type { BgEntry } from './config';

  interface Props {
    dark: string;
    backgrounds: BgEntry[];
    trashTitle: string;
  }

  let { dark, backgrounds, trashTitle }: Props = $props();
</script>

<div class="config-container choose-background {dark}" id="choose-background" style="display: none;">
  <div>
    <button type="button" id="setting-background-back" class="button {dark}">
      ← {text('back')}
    </button>
  </div>
  <h1 class="config-title"> {text('random_bg_menu_title')} </h1>
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
  <div id="create-bg-choices">
    <button class={dark} id="create-color-bg"> {text('add_background_color')} </button>
    {text('or')}
    <input type="file" id="create-image-bg" class={dark} accept="image/jpeg, image/png, image/gif, video/mp4" />
  </div>
  <div class="editorStyle-bar {dark}"></div>
  <div id="choose-backgrounds-container">
    {#each backgrounds as entry}
      {#if entry.kind === 'color'}
        <div
          class="choose-bg-element choose-bg-element-color choose-bg-element-pageload {dark}"
          background={entry.bg}
          background_color_text={text('background_color')}
          style="background-color: {entry.stripped};"
        >
          {text('background_color')} : {entry.stripped}
          <div class="choose-bg-buttons">
            <div class={entry.activateCls}></div>
            <TrashIcon title={trashTitle} />
          </div>
        </div>
      {:else if entry.kind === 'video'}
        <div class="choose-bg-element choose-bg-element-image" background={entry.bg}>
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
          <div class="choose-bg-buttons">
            <div class={entry.activateCls}></div>
            <TrashIcon title={trashTitle} />
          </div>
        </div>
      {:else}
        <div class="choose-bg-element choose-bg-element-image" background={entry.bg}>
          <div
            class="choose-bg-pseudo-element"
            style="background-image: url(&quot;.config/user_uploads/{entry.file}&quot;)"
          ></div>
          <!-- svelte-ignore a11y_missing_attribute: 1:1 port, decorative background thumbnail. -->
          <img src=".config/user_uploads/{entry.file}" />
          <div class="choose-bg-buttons">
            <div class={entry.activateCls}></div>
            <TrashIcon title={trashTitle} />
          </div>
        </div>
      {/if}
    {/each}
  </div>
</div>
