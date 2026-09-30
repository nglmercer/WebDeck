<script lang="ts">
  import { uploadFile } from '../api/uploads';
  import ColorField from '../components/ColorField.svelte';
  import FileField from '../components/FileField.svelte';
  import SectionIcon from '../components/SectionIcon.svelte';
  import TrashIcon from '../components/TrashIcon.svelte';
  import { tx } from '../components/studio/labels';
  import StudioTabs from '../components/studio/StudioTabs.svelte';
  import { text } from '../framework/i18n';
  import { rep } from '../framework/types';
  import { calculateBrightness } from '../components/colors';

  interface Props {
    dark: string;
    /** Ordered background list (`//` prefix = disabled); bound to Config state. */
    backgrounds: string[];
    trashTitle: string;
  }

  let { dark, backgrounds = $bindable(), trashTitle }: Props = $props();

  const toggleLabel = tx('studio_bg_toggle', 'Toggle background');

  type BgView =
    | { kind: 'color'; stripped: string }
    | { kind: 'video'; src: string; file: string }
    | { kind: 'image'; file: string };

  function bgView(bg: string): BgView {
    const stripped = rep(bg, '//', '');
    if (stripped.startsWith('rgb(') || stripped.startsWith('#')) {
      return { kind: 'color', stripped };
    }
    if (bg.endsWith('.mp4')) {
      const file = rep(stripped, '**uploaded/', '');
      return { kind: 'video', src: `.config/user_uploads/${file}`, file };
    }
    return { kind: 'image', file: rep(stripped, '**uploaded/', '') };
  }

  /** Label ink over a color card (same brightness rule as before). */
  function textOn(bg: string): string {
    return calculateBrightness(rep(bg, '//', '')) > 125 ? '#141414' : '#fbfbfd';
  }

  /** Activate/deactivate one card (the last active one stays on). */
  function toggleBg(bg: string): void {
    const at = backgrounds.indexOf(bg);
    if (at === -1) return;
    const next = [...backgrounds];
    if (bg.startsWith('//')) {
      next[at] = bg.replace('//', '');
      backgrounds = next;
    } else if (backgrounds.filter((b) => !b.startsWith('//')).length !== 1) {
      next[at] = `//${bg}`;
      backgrounds = next;
    }
  }

  /** Delete one card (never the last background, never the last active one). */
  function deleteBg(bg: string): void {
    if (backgrounds.length === 1) return;
    const actives = backgrounds.filter((b) => !b.startsWith('//')).length;
    if (actives === 1 && !bg.startsWith('//')) return;
    const at = backgrounds.indexOf(bg);
    if (at === -1) return;
    backgrounds = [...backgrounds.slice(0, at), ...backgrounds.slice(at + 1)];
  }

  let colorHex = $state('');

  function addColorBg(): void {
    if (colorHex === '') return;
    backgrounds = [...backgrounds, colorHex];
  }

  async function uploadBg(file: File): Promise<void> {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('info', 'background_image');
    try {
      await uploadFile(formData);
    } catch {
      console.error('Failed to download file.');
      return;
    }
    backgrounds = [...backgrounds, `**uploaded/${file.name}`];
  }

  // Add-background composer tabs. Panes stay mounted (hidden only) so the
  // file input keeps its selection while switching tabs.
  let addTab = $state('color');
  // svelte-ignore state_referenced_locally
  const addTabs = [
    { id: 'color', label: text('background_color') },
    { id: 'file', label: text('select_your_file') },
  ];
</script>

{#snippet bgButtons(bg: string)}
  {@const active = !bg.startsWith('//')}
  <div class="choose-bg-buttons">
    <button
      type="button"
      class={active
        ? 'choose-bg-activate-button choose-bg-activate-button-checked'
        : 'choose-bg-activate-button'}
      aria-label={toggleLabel}
      aria-pressed={active ? 'true' : 'false'}
      onclick={() => toggleBg(bg)}
    >
      <SectionIcon name="check" size={13} />
    </button>
    <TrashIcon title={trashTitle} onclick={() => deleteBg(bg)} />
  </div>
{/snippet}

<div class="config-container choose-background {dark}" id="choose-background">
  <div class="wd2-panel-head">
    <h1 class="config-title"> {text('random_bg_menu_title')} </h1>
    <span class="wd2-count" id="bg-count">{backgrounds.length}</span>
  </div>
  <div id="create-bg-choices">
    <div class="wd2-bg-add-tabs">
      <StudioTabs
        tabs={addTabs}
        selected={addTab}
        onSelect={(id) => (addTab = id)}
        idPrefix="bg-add"
      />
    </div>
    <div
      role="tabpanel"
      id="bg-add-pane-color"
      aria-labelledby="bg-add-tab-color"
      hidden={addTab !== 'color'}
    >
      <div class="wd2-bg-add-row">
        <ColorField
          dark={dark}
          containerClass="background-color-input-container"
          colorClass="background-color-input"
          colorId="background-color-input"
          hexClass="background-color-setting"
          hexId="background-color-hex"
          placeholder="{text('wallpaper_color')} (HEX)"
          bind:value={colorHex}
        />
        <button type="button" class={dark} id="create-color-bg" onclick={addColorBg}>
          {text('add_background_color')}
        </button>
      </div>
    </div>
    <div
      role="tabpanel"
      id="bg-add-pane-file"
      aria-labelledby="bg-add-tab-file"
      hidden={addTab !== 'file'}
    >
      <FileField
        id="create-image-bg"
        accept="image/jpeg, image/png, image/gif, video/mp4"
        browseLabel={text('select_your_file')}
        emptyLabel={text('no_file_chosen')}
        hint="JPG · PNG · GIF · MP4"
        onFile={(file) => void uploadBg(file)}
      />
    </div>
  </div>
  <div id="choose-backgrounds-container">
    {#each backgrounds as bg, i (i)}
      {@const view = bgView(bg)}
      {#if view.kind === 'color'}
        <div
          class="choose-bg-element choose-bg-element-color {dark}"
          data-background={bg}
          style="background-color: {view.stripped}; color: {textOn(bg)};"
        >
          <span class="choose-bg-swatch" style="background-color: {view.stripped};" aria-hidden="true"></span>
          <div class="choose-bg-foot">
            <span class="choose-bg-label">{text('background_color')} · {view.stripped}</span>
            {@render bgButtons(bg)}
          </div>
        </div>
      {:else if view.kind === 'video'}
        <div class="choose-bg-element choose-bg-element-image" data-background={bg}>
          <div class="choose-bg-thumb">
            <div class="video-container choose-bg-pseudo-element">
              <!-- svelte-ignore a11y_media_has_caption: 1:1 port, decorative ambient background. -->
              <video autoplay muted loop class="blurred-video">
                <source src={view.src} type="video/mp4" />
                Your browser does not support the video tag...
              </video>
            </div>
            <div class="video-container">
              <!-- svelte-ignore a11y_media_has_caption: 1:1 port, decorative ambient background. -->
              <video autoplay muted loop>
                <source src={view.src} type="video/mp4" />
                Your browser does not support the video tag...
              </video>
            </div>
          </div>
          <div class="choose-bg-foot">
            <span class="choose-bg-label">{view.file}</span>
            {@render bgButtons(bg)}
          </div>
        </div>
      {:else}
        <div class="choose-bg-element choose-bg-element-image" data-background={bg}>
          <div class="choose-bg-thumb">
            <div
              class="choose-bg-pseudo-element"
              style="background-image: url(&quot;.config/user_uploads/{view.file}&quot;)"
            ></div>
            <img src=".config/user_uploads/{view.file}" alt="" />
          </div>
          <div class="choose-bg-foot">
            <span class="choose-bg-label">{view.file}</span>
            {@render bgButtons(bg)}
          </div>
        </div>
      {/if}
    {/each}
  </div>
</div>
