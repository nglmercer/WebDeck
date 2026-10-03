<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  import { localActionNames } from '../../lib/action-labels';
  import { modal } from '../../lib/modal';
  import { onDestroy } from 'svelte';
  import { AssetCache } from '../../lib/assets';
  import { asset } from '../../lib/api/client';
  import type {
    Button,
    ButtonAction,
    Command,
    Layout,
    CatalogResponse,
    PluginManifest,
  } from '../../lib/contracts';
  import { commandSchema, defaultValue, upload, type Schema } from '../../lib/api/client';
  import Fields from './Fields.svelte';
  import ActionFields from './ActionFields.svelte';
  import ButtonContent from '../deck/ButtonContent.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import { appearanceOf, setAppearance as writeAppearance, tileColors } from './appearance';
  import { contract } from '../../lib/schema';
  let {
    button = $bindable(),
    buttonFolder = $bindable(),
    layout,
    catalog,
    assetUrls,
    moveButton,
    removeButton,
    duplicateButton,
    commitButton,
    close,
    attempt,
  }: {
    button: Button;
    buttonFolder: string;
    layout: Layout;
    catalog: CatalogResponse | null;
    assetUrls: Record<string, string>;
    moveButton: (delta: number) => void;
    removeButton: () => void;
    duplicateButton: () => void;
    commitButton: () => void;
    close: () => void;
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();
  const defaultCommand = () => defaultValue(commandSchema) as Command;
  let destroyed = false;
  const previewAssets = new AssetCache(asset);
  let previewUrl = $state<string | undefined>();
  $effect(() => {
    let disposed = false;
    const icon = button.icon;
    if (!icon.startsWith('asset:')) {
      previewUrl = undefined;
      return;
    }
    const id = icon.slice(6);
    const borrowed = assetUrls[id];
    if (borrowed) {
      previewUrl = borrowed;
      return;
    }
    void previewAssets
      .load([id])
      .then((result) => {
        if (!disposed && result) previewUrl = result.urls[id];
      })
      .catch(() => {
        if (!disposed) formError = 'Image preview could not be loaded. Your draft is unchanged.';
      });
    return () => {
      disposed = true;
    };
  });
  onDestroy(() => {
    destroyed = true;
    previewAssets.dispose();
  });
  let formError = $state('');
  const appearance = $derived(appearanceOf(button));
  const colors = $derived(tileColors(button.color));
  function setAppearance(key: string, value: unknown) {
    writeAppearance(button, key, value);
  }
  function apply(event: SubmitEvent) {
    event.preventDefault();
    try {
      contract('Button', button);
      formError = '';
      commitButton();
    } catch (error) {
      formError = error instanceof Error ? error.message : String(error);
    }
  }
  const selectedPlugin = $derived(
    button?.action.type === 'command' && button.action.command.type === 'plugin'
      ? catalog?.plugins.find(
          (m) =>
            button?.action.type === 'command' &&
            button.action.command.type === 'plugin' &&
            m.id === button.action.command.plugin_id,
        )
      : undefined,
  );
  function pluginActionSchema(m: PluginManifest, actionId: string): Schema {
    const a = m.actions.find((a) => a.id === actionId);
    return {
      type: 'object',
      additionalProperties: false,
      properties: Object.fromEntries(
        Object.entries(a?.arguments ?? {}).map(([k, v]) => [k, { type: v.type }]),
      ),
    };
  }
  function chooseCommand(c: Command) {
    if (!button) return;
    button.action = { type: 'command', command: c };
    if (c.type === 'plugin') {
      const m = catalog?.plugins[0];
      if (m) pluginCommand(m);
    }
  }
  async function image(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f && button) {
      const candidate = button;
      const s = await upload(f);
      if (!destroyed && button === candidate && s.type === 'asset') button.icon = `asset:${s.id}`;
    }
  }
  function pluginCommand(m: PluginManifest) {
    const action = m.actions[0];
    if (!action || !button) return;
    button.action = {
      type: 'command',
      command: {
        type: 'plugin',
        plugin_id: m.id,
        version: m.version,
        action_id: action.id,
        args: Object.fromEntries(
          Object.entries(action.arguments).map(([k, v]) => [
            k,
            v.type === 'boolean'
              ? false
              : v.type === 'number'
                ? 0
                : v.type === 'array'
                  ? []
                  : v.type === 'object'
                    ? {}
                    : '',
          ]),
        ),
      },
    };
  }
</script>

<div class="overlay">
  <dialog
    use:modal
    oncancel={close}
    aria-modal="true"
    aria-labelledby="button-title"
    class="editor"
  >
    <div class="heading">
      <h2 id="button-title">{t('ui_edit_button')}</h2>
      <button aria-label={t('ui_close_editor')} onclick={close}>×</button>
    </div>
    <div class="preview">
      <div
        class="deck-button preview-tile"
        role="img"
        aria-label={t('ui_button_preview')}
        style={`--button-color:${button.color};--button-foreground:${colors.foreground};--icon-size:${appearance.icon_size ?? 42}px;`}
      >
        <ButtonContent {button} assetUrl={previewUrl} />
      </div>
      <p class="field-help">
        {colors.contrast === null
          ? t('ui_choose_a_solid_color_to_check_readability')
          : t('ui_text_color_adjusts_to_keep_the_label_readable')}
      </p>
    </div>
    <form onsubmit={apply}>
      <p class="field-help">
        {t('ui_apply_changes_to_your_draft_then_save_the_deck_to_update_the_host')}
      </p>
      <h3>{t('ui_content')}</h3>
      <label
        >{t('ui_button_folder')}<select bind:value={buttonFolder}
          >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option>{/each}</select
        ></label
      >
      <label>{t('ui_label')}<input bind:value={button.label} /></label><label
        >{t('ui_icon')}<input bind:value={button.icon} /></label
      ><label
        >{t('ui_upload_image')}<input
          type="file"
          accept="image/*"
          onchange={(e) => attempt(() => image(e))}
        /></label
      ><label>{t('ui_color')}<input type="color" bind:value={button.color} /></label>
      <h3>{t('ui_appearance')}</h3>
      <div class="appearance-fields">
        <NumberField
          label={t('ui_column_span')}
          value={Number(appearance.columns ?? 1)}
          min={1}
          max={layout.columns}
          onchange={(value) => setAppearance('columns', value)}
        />
        <NumberField
          label={t('ui_row_span')}
          value={Number(appearance.rows ?? 1)}
          min={1}
          max={128}
          onchange={(value) => setAppearance('rows', value)}
        />
        <NumberField
          label={t('ui_icon_size')}
          value={Number(appearance.icon_size ?? 42)}
          min={0}
          max={200}
          onchange={(value) => setAppearance('icon_size', value)}
        />
        <label class="check"
          ><input
            type="checkbox"
            checked={appearance.show_label !== false}
            onchange={(e) => setAppearance('show_label', e.currentTarget.checked)}
          />{t('ui_show_label')}</label
        >
      </div>
      <h3>{t('ui_action')}</h3>
      <label
        >{t('ui_action')}<select
          value={button.action.type}
          onchange={(e) => {
            if (!button) return;
            const type = e.currentTarget.value;
            if (type === 'command') button.action = { type, command: defaultCommand() };
            else if (type === 'metric')
              button.action = { type, metric: 'cpu', target: '', interval_ms: 1000 };
            else if (type === 'folder')
              button.action = { type, folder_id: layout?.folders[0]?.id ?? 'home' };
            else button.action = { type } as ButtonAction;
          }}
          >{#each ['command', 'folder', 'back', 'reload', 'fullscreen', 'settings', 'metric', 'edit', 'none'] as type}<option
              value={type}>{type}</option
            >{/each}</select
        ></label
      >
      {#if button.action.type === 'metric'}
        <label
          >{t('ui_metric')}<select bind:value={button.action.metric}
            >{#each ['cpu', 'memory', 'gpu', 'gpu_memory', 'disk', 'clock'] as metric}<option
                value={metric}>{metric}</option
              >{/each}</select
          ></label
        >
        <label
          >{t('ui_device_name_or_index_empty_uses_default')}<input
            bind:value={button.action.target}
          /></label
        >
        <NumberField
          label={t('ui_update_interval_ms')}
          value={button.action.interval_ms}
          min={250}
          max={3600000}
          onchange={(value) => {
            if (button.action.type === 'metric') button.action.interval_ms = value;
          }}
        />
      {:else if button.action.type === 'folder'}<label
          >{t('ui_destination')}<select bind:value={button.action.folder_id}
            >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option
              >{/each}</select
          ></label
        >{:else if button.action.type === 'command'}<ActionFields
          value={button.action.command}
          onchange={(c) => chooseCommand(c as Command)}
        />{#if button.action.command.type === 'plugin'}<label
            >{t('ui_plugin')}<select
              value={button.action.command.plugin_id}
              onchange={(e) => {
                const m = catalog?.plugins.find((m) => m.id === e.currentTarget.value);
                if (m) pluginCommand(m);
              }}
              >{#each catalog?.plugins ?? [] as m}<option value={m.id}>{m.id} · {m.version}</option
                >{/each}</select
            ></label
          >{#if selectedPlugin}<label
              >{t('ui_plugin_action')}<select
                value={button.action.command.action_id}
                onchange={(e) => {
                  if (
                    button?.action.type === 'command' &&
                    button.action.command.type === 'plugin'
                  ) {
                    button.action.command.action_id = e.currentTarget.value;
                    button.action.command.args = defaultValue(
                      pluginActionSchema(selectedPlugin!, e.currentTarget.value),
                    ) as Record<string, unknown>;
                  }
                }}
                >{#each selectedPlugin.actions as a}<option value={a.id}>{a.label}</option
                  >{/each}</select
              ></label
            ><Fields
              schema={pluginActionSchema(selectedPlugin, button.action.command.action_id)}
              value={button.action.command.args}
              onchange={(args) => {
                if (button?.action.type === 'command' && button.action.command.type === 'plugin')
                  button.action.command.args = args as Record<string, unknown>;
              }}
              label={t('ui_plugin_arguments')}
            />{:else}<p>{t('ui_no_v2_plugins_are_installed')}</p>{/if}{/if}{/if}
      <div class="row">
        <button type="button" onclick={() => moveButton(-1)}>{t('ui_move_left')}</button><button
          type="button"
          onclick={() => moveButton(1)}>{t('ui_move_right')}</button
        ><button type="button" onclick={duplicateButton}>{t('ui_duplicate')}</button><button
          type="button"
          class="danger"
          onclick={removeButton}>{t('ui_delete')}</button
        ><button type="button" onclick={close}>{t('ui_cancel')}</button><button
          type="submit"
          class="primary">{t('ui_apply_to_draft')}</button
        >
      </div>
      {#if formError}<p role="alert">{t(formError)}</p>{/if}
    </form>
  </dialog>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: #0009;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    z-index: 20;
  }
  .editor {
    width: min(650px, 100%);
    max-height: 90dvh;
    overflow: auto;
    background: var(--surface);
    color: inherit;
    border: 1px solid #ffffff25;
    border-radius: 8px;
    padding: 20px;
    box-shadow: var(--shadow-dialog);
  }
  .editor::backdrop {
    background: transparent;
  }
  .preview {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 16px;
    padding: 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-panel);
  }
  .preview-tile {
    width: 140px;
    min-height: 140px;
  }
  .preview p {
    max-width: 250px;
  }
  .appearance-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 0 12px;
  }
</style>
