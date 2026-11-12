<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import type { Editor } from '../editor/editor.svelte';
  import { appearanceBounds, appearanceOf } from '../editor/appearance';
  import { appearanceNumber } from '../deck/deck';
  import { upload } from '../../lib/api/client';
  import SettingsCard from './ui/SettingsCard.svelte';
  import ChoiceField from './ui/ChoiceField.svelte';
  import FileField from './ui/FileField.svelte';
  import SwitchField from './ui/SwitchField.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tileColors } from '../editor/appearance';
  import NumberField from '../../components/NumberField.svelte';
  let {
    editor,
    languages,
    assets,
    attempt,
    assetUrls = {},
  }: {
    editor: Editor;
    languages: string[];
    assets: () => Promise<void>;
    attempt: (work: () => Promise<void>) => Promise<void>;
    assetUrls?: Record<string, string>;
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
  function languageLabel(code: string) {
    try {
      return (
        new Intl.DisplayNames([code.replace('_', '-')], { type: 'language' }).of(
          code.split('_')[0]!,
        ) ?? code
      );
    } catch {
      return code;
    }
  }
</script>

<div id="settings-appearance" class="settings-page">
  <div class="settings-page-heading">
    <h2>{t('ui_appearance')}</h2>
    <p>{t('ui_appearance_description')}</p>
  </div>
  <div class="appearance-layout">
    <div class="settings-stack">
      <SettingsCard title={t('ui_layout')} icon="grid">
        <div class="settings-fields two-columns">
          <NumberField
            controls
            label={t('ui_columns')}
            value={editor.draft.layout.columns}
            min={1}
            max={128}
            onchange={(value) => editor.setDimensions('columns', value)}
          />
          <NumberField
            controls
            label={t('ui_rows')}
            value={editor.draft.layout.rows}
            min={1}
            max={128}
            onchange={(value) => editor.setDimensions('rows', value)}
          />
        </div>
      </SettingsCard>
      <SettingsCard title={t('ui_button_style')} icon="settings">
        <div class="settings-fields two-columns">
          {#each properties as [key, label, fallback]}<NumberField
              controls
              unit="px"
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
        </div>
        <SwitchField
          label={t('ui_show_button_labels')}
          checked={appearance.show_labels !== false}
          onchange={(value) => editor.updateAppearance('show_labels', value)}
        />
      </SettingsCard>
    </div>
    <div class="settings-stack">
      <SettingsCard title={t('ui_preview')} icon="monitor">
        <div
          class="appearance-preview"
          aria-hidden="true"
          style:background-image={editor.draft.layout.backgrounds[0] &&
          assetUrls[editor.draft.layout.backgrounds[0]]
            ? `url("${assetUrls[editor.draft.layout.backgrounds[0]]}")`
            : undefined}
        >
          {#each (editor.draft.layout.folders[0]?.buttons ?? []).slice(0, 8) as button}
            <div class="preview-cell">
              <div
                class="preview-tile"
                style:background={button.color}
                style:color={tileColors(button.color).foreground}
                style:border-radius={`${Math.min(28, appearanceNumber(appearance.radius, 8, 0, 100))}px`}
              >
                {#if button.icon.startsWith('asset:') && assetUrls[button.icon.slice(6)]}<img
                    src={assetUrls[button.icon.slice(6)]}
                    alt=""
                  />{:else}<Icon
                    name={button.icon}
                    size={Math.min(42, appearanceNumber(appearance.icon_size, 42, 0, 200))}
                  />{/if}
              </div>
              {#if appearance.show_labels !== false}<span>{button.label}</span>{/if}
            </div>
          {/each}
        </div>
      </SettingsCard>
      <SettingsCard title={t('ui_language')}>
        <ChoiceField
          hideLabel
          label={t('ui_language')}
          value={editor.draft.settings.language}
          options={languages.map((value) => ({ value, label: languageLabel(value) }))}
          onchange={(value) => {
            editor.draft.settings.language = value;
            editor.change();
          }}
        />
      </SettingsCard>
    </div>
  </div>
  <SettingsCard title={t('ui_files')} icon="image">
    <div class="settings-fields two-columns">
      {#each ['backgrounds', 'themes'] as assetType}{@const key = assetType as
          'backgrounds' | 'themes'}
        <div>
          <FileField
            label={t(key === 'themes' ? 'ui_upload_a_theme' : 'ui_upload_a_background')}
            accept={key === 'themes' ? '.css' : 'image/*'}
            onchange={(event) => void attempt(() => uploadBackground(event, key === 'themes'))}
          />
          {#each editor.draft.layout[key] as asset}<div class="settings-file-item">
              <span title={asset}>{asset}</span><button
                onclick={() => {
                  editor.removeAsset(key, asset);
                  void assets();
                }}>{t('ui_remove')}</button
              >
            </div>{/each}
        </div>{/each}
    </div>
  </SettingsCard>
</div>

<style>
  .appearance-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 20px;
  }
  .appearance-preview {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 14px 10px;
    background: var(--canvas);
    background-size: cover;
    padding: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
  }
  .preview-cell {
    min-width: 0;
    text-align: center;
  }
  .preview-tile {
    display: flex;
    align-items: center;
    justify-content: center;
    aspect-ratio: 1;
    min-height: 44px;
  }
  .preview-tile img {
    width: 55%;
    height: 55%;
    object-fit: contain;
  }
  .preview-cell > span {
    display: block;
    margin-top: 6px;
    font-size: 0.65rem;
    line-height: 1.4;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  @media (max-width: 1000px) {
    .appearance-layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
