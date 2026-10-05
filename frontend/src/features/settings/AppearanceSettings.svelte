<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import type { Editor } from '../editor/editor.svelte';
  import { appearanceBounds, appearanceOf } from '../editor/appearance';
  import { appearanceNumber } from '../deck/deck';
  import { upload } from '../../lib/api/client';
  import NumberField from '../../components/NumberField.svelte';
  let {
    editor,
    languages,
    assets,
    attempt,
  }: {
    editor: Editor;
    languages: string[];
    assets: () => Promise<void>;
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();
  const appearance = $derived(appearanceOf(editor.draft.layout));
  const properties = [
    ['button_height', 'ui_button_height', 140],
    ['gap', 'ui_gap', 10],
    ['radius', 'ui_corner_radius', 8],
    ['icon_size', 'ui_icon_size', 42],
  ] as const;
  async function uploadBackground(event: Event, theme = false) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const source = await upload(file);
    if (source.type === 'asset') {
      editor.addAsset(theme ? 'themes' : 'backgrounds', source.id);
      await assets();
    }
  }
</script>

<section id="settings-appearance">
  <h2>{t('ui_appearance')}</h2>
  <div class="appearance-fields">
    {#each properties as [key, label, fallback]}<NumberField
        label={t(label)}
        value={appearanceNumber(
          appearance[key],
          fallback,
          appearanceBounds[key][0],
          appearanceBounds[key][1],
        )}
        min={appearanceBounds[key][0]}
        max={appearanceBounds[key][1]}
        onchange={(value) => editor.updateAppearance(key, value)}
      />{/each}
    <label class="check"
      ><input
        type="checkbox"
        checked={appearance.show_labels !== false}
        onchange={(event) => editor.updateAppearance('show_labels', event.currentTarget.checked)}
      />{t('ui_show_button_labels')}</label
    >
    <NumberField
      label={t('ui_columns')}
      value={editor.draft.layout.columns}
      min={1}
      max={128}
      onchange={(value) => editor.setDimensions('columns', value)}
    />
    <NumberField
      label={t('ui_rows')}
      value={editor.draft.layout.rows}
      min={1}
      max={128}
      onchange={(value) => editor.setDimensions('rows', value)}
    />
    <label
      >{t('ui_language')}<select
        bind:value={editor.draft.settings.language}
        onchange={() => editor.change()}
        >{#each languages as language}<option value={language}>{language}</option>{/each}</select
      ></label
    >
  </div>
  <label
    >{t('ui_upload_a_background')}<input
      type="file"
      accept="image/*"
      onchange={(event) => attempt(() => uploadBackground(event))}
    /></label
  >
  {#each editor.draft.layout.backgrounds as background}<div class="row">
      <span>{background}</span><button
        onclick={() => {
          editor.removeAsset('backgrounds', background);
          void assets();
        }}>{t('ui_remove')}</button
      >
    </div>{/each}
  <label
    >{t('ui_upload_a_theme')}<input
      type="file"
      accept=".css"
      onchange={(event) => attempt(() => uploadBackground(event, true))}
    /></label
  >
  {#each editor.draft.layout.themes as theme}<div class="row">
      <span>{theme}</span><button
        onclick={() => {
          editor.removeAsset('themes', theme);
          void assets();
        }}>{t('ui_remove')}</button
      >
    </div>{/each}
</section>

<style>
  .appearance-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 0 16px;
  }
</style>
