<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let activeTab = $state('content');

  import { localActionNames } from '../../lib/action-labels';
  import { modal } from '../../lib/modal';
  import { onDestroy, onMount } from 'svelte';
  import { AssetCache } from '../../lib/assets';
  import { asset, request } from '../../lib/api/client';
  import type {
    Button,
    ButtonAction,
    Command,
    Layout,
    CatalogResponse,
    PluginManifest,
    ImageAssetList,
    ImageImport,
    FileSource,
  } from '../../lib/contracts';
  import { commandSchema, defaultValue, resolve, upload, type Schema } from '../../lib/api/client';
  import Fields from './Fields.svelte';
  import ActionFields from './ActionFields.svelte';
  import ButtonContent from '../deck/ButtonContent.svelte';
  import NumberField from '../../components/NumberField.svelte';
  import { appearanceOf, setAppearance as writeAppearance, tileColors } from './appearance';
  import { contract } from '../../lib/schema';
  import IconField from './IconField.svelte';
  let {
    button = $bindable(),
    buttonFolder = $bindable(),
    layout,
    catalog,
    assetUrls,
    refreshAssets,
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
    refreshAssets: (id: string) => Promise<void>;
    moveButton: (delta: number) => void;
    removeButton: () => void;
    duplicateButton: () => void;
    commitButton: () => void;
    close: () => void;
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();
  let imageIds = $state<string[]>([]);
  let imagePage = $state(0);
  let imageUrl = $state('');
  let imagePath = $state('');
  let liveImage = $state(false);
  let liveIds = $state<string[]>([]);
  let imageBusy = $state(false);
  let refreshTick = $state(0);

  let libraryUrls = $state<Record<string, string>>({});
  let libraryError = $state('');
  const libraryAssets = new AssetCache(asset);
  const pageIds = $derived(imageIds.slice(imagePage * 24, (imagePage + 1) * 24));
  onMount(() => {
    void request<ImageAssetList>('assets', 'ImageAssetList')
      .then((value) => {
        if (!destroyed) {
          imageIds = [...new Set([...imageIds, ...value.images])];
          liveIds = [...new Set([...liveIds, ...(value.live_images ?? [])])];
        }
      })
      .catch(() => {
        if (!destroyed) libraryError = 'ui_icon_library_unavailable';
      });
  });
  $effect(() => {
    void refreshTick;
    let stale = false;
    void libraryAssets.load(pageIds).then((result) => {
      if (!stale && result) libraryUrls = result.urls;
    });
    return () => {
      stale = true;
    };
  });
  const defaultCommand = () => defaultValue(commandSchema) as Command;
  let destroyed = false;
  const previewAssets = new AssetCache(asset);
  let previewUrl = $state<string | undefined>();
  $effect(() => {
    void refreshTick;
    let disposed = false;
    const icon = button.icon;
    if (!icon.startsWith('asset:')) {
      previewUrl = undefined;
      return;
    }
    const id = icon.slice(6);
    const borrowed = refreshTick === 0 ? assetUrls[id] : undefined;
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
    libraryAssets.dispose();
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
      if (!destroyed && button === candidate && s.type === 'asset') {
        button.icon = `asset:${s.id}`;
        imageIds = [s.id, ...imageIds.filter((id) => id !== s.id)];
        imagePage = 0;
      }
    }
  }
  async function importImage(source: ImageImport) {
    if (imageBusy) return;
    const candidate = button;
    imageBusy = true;
    try {
      const result = await request<FileSource>('assets/import', 'FileSource', {
        method: 'POST',
        body: JSON.stringify(source),
      });
      if (!destroyed && result.type === 'asset') {
        imageIds = [result.id, ...imageIds.filter((id) => id !== result.id)];
        imagePage = 0;
        if (source.type === 'url' && source.live) liveIds = [...liveIds, result.id];
        if (button === candidate) button.icon = `asset:${result.id}`;
      }
    } finally {
      if (!destroyed) imageBusy = false;
    }
  }
  async function refreshImage() {
    if (imageBusy || !button.icon.startsWith('asset:')) return;
    const id = button.icon.slice(6);
    imageBusy = true;
    try {
      await request<FileSource>(`assets/${encodeURIComponent(id)}/refresh`, 'FileSource', {
        method: 'POST',
      });
      if (!destroyed) {
        libraryAssets.invalidate(id);
        previewAssets.invalidate(id);
        refreshTick++;
      }
      await refreshAssets(id);
    } finally {
      if (!destroyed) imageBusy = false;
    }
  }
  async function chooseLocalImage() {
    const result = await request<{ api_version: 2; source: FileSource | null }>(
      'native/selection',
      'SelectionResponse',
      {
        method: 'POST',
        body: JSON.stringify({ kind: 'file' }),
      },
    );
    if (!destroyed && result.source?.type === 'external') {
      imagePath = result.source.path;
      await importImage({ type: 'local', path: imagePath });
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
    oncancel={(event) => {
      event.preventDefault();
      close();
    }}
    aria-modal="true"
    aria-labelledby="button-title"
    class="editor"
  >
    <div class="heading">
      <h2 id="button-title">{t('ui_edit_button')}</h2>
      <button aria-label={t('ui_close_editor')} onclick={close}><Icon name="close" /></button>
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
    </div>
    <form onsubmit={apply}>
      <div role="tablist" aria-label={t('ui_edit_button')} class="editor-tabs">
        {#each ['content', 'appearance', 'action'] as tab}<button
            type="button"
            role="tab"
            id={`editor-tab-${tab}`}
            aria-controls={`editor-panel-${tab}`}
            aria-selected={activeTab === tab}
            tabindex={activeTab === tab ? 0 : -1}
            onkeydown={(event) => {
              const tabs = ['content', 'appearance', 'action'];
              const index = tabs.indexOf(activeTab);
              const next =
                event.key === 'ArrowRight'
                  ? (index + 1) % 3
                  : event.key === 'ArrowLeft'
                    ? (index + 2) % 3
                    : event.key === 'Home'
                      ? 0
                      : event.key === 'End'
                        ? 2
                        : -1;
              if (next >= 0) {
                event.preventDefault();
                activeTab = tabs[next]!;
                document.getElementById(`editor-tab-${activeTab}`)?.focus();
              }
            }}
            onclick={() => (activeTab = tab)}>{t(`ui_${tab}`)}</button
          >{/each}
      </div>
      <div
        role="tabpanel"
        id="editor-panel-content"
        aria-labelledby="editor-tab-content"
        hidden={activeTab !== 'content'}
      >
        <div class="basics-grid">
          <label
            >{t('ui_button_folder')}<select bind:value={buttonFolder}
              >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option
                >{/each}</select
            ></label
          >
          <label>{t('ui_label')}<input bind:value={button.label} /></label>
          <div class="row icon-color">
            <IconField value={button.icon} onSelect={(value) => (button.icon = value)} /><label
              >{t('ui_color')}<input
                class="color-input"
                type="color"
                bind:value={button.color}
              /></label
            >
          </div>
        </div>
        <details class="custom-images">
          <summary>{t('ui_custom_images')}</summary>
          <label
            >{t('ui_upload_image')}<input
              type="file"
              accept="image/*"
              onchange={(e) => attempt(() => image(e))}
            /></label
          >
          <label
            >{t('ui_image_url')}<input type="text" inputmode="url" bind:value={imageUrl} /></label
          >
          <label><input type="checkbox" bind:checked={liveImage} />{t('ui_live_image')}</label>
          <button
            type="button"
            disabled={imageBusy || !imageUrl.trim()}
            onclick={() =>
              attempt(() => importImage({ type: 'url', url: imageUrl.trim(), live: liveImage }))}
            >{t('ui_import_image_url')}</button
          >
          <label>{t('ui_local_image_path')}<input bind:value={imagePath} /></label>
          <button
            type="button"
            disabled={imageBusy || !imagePath.trim()}
            onclick={() => attempt(() => importImage({ type: 'local', path: imagePath.trim() }))}
            >{t('ui_import_local_image')}</button
          >
          <button type="button" disabled={imageBusy} onclick={() => attempt(chooseLocalImage)}
            >{t('ui_choose_local_image')}</button
          >
          <p>{t('ui_image_import_help')}</p>
          {#if button.icon.startsWith('asset:') && liveIds.includes(button.icon.slice(6))}
            <button type="button" disabled={imageBusy} onclick={() => attempt(refreshImage)}
              >{t('ui_refresh_live_image')}</button
            >
            <p>{t('ui_refresh_live_image_help')}</p>
          {/if}
          {#if imageBusy}<p role="status">{t('ui_importing_image')}</p>{/if}
          <h3>{t('ui_uploaded_icons')}</h3>
          {#if libraryError}<p role="status">{t(libraryError)}</p>{/if}
          <div class="icon-picker">
            {#each pageIds as id}<button
                type="button"
                aria-label={`${t('ui_select_uploaded_icon')} ${id}`}
                aria-pressed={button.icon === `asset:${id}`}
                onclick={() => (button.icon = `asset:${id}`)}
              >
                {#if libraryUrls[id]}<img
                    src={libraryUrls[id]}
                    alt=""
                    width="32"
                    height="32"
                  />{:else}<Icon name="image" />{/if}
              </button>{/each}
          </div>
          {#if imageIds.length > 24}
            <button type="button" disabled={imagePage === 0} onclick={() => imagePage--}
              >{t('ui_previous')}</button
            >
            <button
              type="button"
              disabled={(imagePage + 1) * 24 >= imageIds.length}
              onclick={() => imagePage++}>{t('ui_next')}</button
            >
          {/if}
        </details>
      </div>
      <div
        role="tabpanel"
        id="editor-panel-appearance"
        aria-labelledby="editor-tab-appearance"
        hidden={activeTab !== 'appearance'}
      >
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
      </div>
      <div
        role="tabpanel"
        id="editor-panel-action"
        aria-labelledby="editor-tab-action"
        hidden={activeTab !== 'action'}
      >
        <label
          >{t('ui_action')}<select
            value={button.action.type}
            onchange={(e) => {
              if (!button) return;
              const type = e.currentTarget.value;
              if (type === 'command') button.action = { type, command: defaultCommand() };
              else if (type === 'workflow')
                button.action = {
                  type,
                  workflow: {
                    type: 'sequence',
                    steps: [{ type: 'command', command: defaultCommand() }],
                  },
                };
              else if (type === 'script')
                button.action = {
                  type,
                  language: 'javascript',
                  source: { type: 'inline', code: '' },
                };
              else if (type === 'plugin') {
                const manifest = catalog?.plugins[0];
                if (manifest)
                  button.action = {
                    type,
                    plugin_id: manifest.id,
                    version: manifest.version,
                    action_id: manifest.actions[0]!.id,
                    args: defaultValue(
                      pluginActionSchema(manifest, manifest.actions[0]!.id),
                    ) as Record<string, unknown>,
                  };
              } else if (type === 'metric')
                button.action = { type, metric: 'cpu', target: '', interval_ms: 1000 };
              else if (type === 'folder')
                button.action = { type, folder_id: layout?.folders[0]?.id ?? 'home' };
              else button.action = { type } as ButtonAction;
            }}
            >{#each ['command', 'workflow', 'script', 'plugin', 'folder', 'back', 'reload', 'fullscreen', 'settings', 'metric', 'edit', 'none'] as type}<option
                value={type}>{type}</option
              >{/each}</select
          ></label
        >
        {#if button.action.type === 'workflow' || button.action.type === 'script' || button.action.type === 'plugin'}
          <Fields
            schema={resolve({ $ref: '#/$defs/ButtonAction' }).oneOf?.find(
              (option) => option.properties?.type?.const === button!.action.type,
            ) ?? {}}
            value={button.action}
            onchange={(action) => {
              if (button) button.action = action as ButtonAction;
            }}
            label={t('ui_action')}
          />
        {:else if button.action.type === 'metric'}
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
                >{#each catalog?.plugins ?? [] as m}<option value={m.id}
                    >{m.id} · {m.version}</option
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
      </div>
      <div class="actions">
        <details class="more-actions">
          <summary aria-label={t('ui_more_button_actions')}><Icon name="more" /></summary>
          <div class="menu">
            <button type="button" onclick={() => moveButton(-1)}>{t('ui_move_left')}</button><button
              type="button"
              onclick={() => moveButton(1)}>{t('ui_move_right')}</button
            ><button type="button" onclick={duplicateButton}>{t('ui_duplicate')}</button><button
              type="button"
              class="danger"
              onclick={removeButton}>{t('ui_delete')}</button
            >
          </div>
        </details>
        <span class="spacer"></span><button type="button" onclick={close}>{t('ui_cancel')}</button
        ><button type="submit" class="primary">{t('ui_apply_to_draft')}</button>
      </div>
      {#if formError}<p role="alert">{t(formError)}</p>{/if}
    </form>
  </dialog>
</div>

<style>
  .icon-picker {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 8px;
    margin: 12px 0;
  }
  .icon-picker button {
    display: flex;
    justify-content: center;
  }
  .icon-picker [aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .editor-tabs {
    display: flex;
    gap: 6px;
    margin-bottom: 16px;
  }
  .editor-tabs button {
    flex: 1;
  }
  .editor-tabs [aria-selected='true'] {
    background: var(--accent-surface);
    color: white;
  }
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
    width: min(520px, 100%);
    max-height: 90dvh;
    overflow: auto;
    background: var(--surface);
    color: inherit;
    border: 1px solid #ffffff25;
    border-radius: 8px;
    padding: 16px;
    box-shadow: var(--shadow-dialog);
  }
  .editor::backdrop {
    background: transparent;
  }
  .preview {
    display: flex;
    justify-content: center;
    padding: 8px;
    margin-bottom: 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-panel);
  }
  .preview-tile {
    width: 92px;
    min-height: 82px;
    padding: 8px;
  }
  .basics-grid {
    display: grid;
    gap: 10px;
  }
  .row.icon-color {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 100px;
    align-items: end;
    gap: 10px;
  }
  .color-input {
    height: 42px;
    padding: 4px;
  }
  .custom-images {
    margin-top: 10px;
    border-top: 1px solid var(--border-subtle);
    padding-top: 9px;
  }
  .custom-images summary {
    cursor: pointer;
    color: var(--text-muted);
  }
  .custom-images[open] {
    display: grid;
    gap: 8px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
  }
  .spacer {
    flex: 1;
  }
  .more-actions {
    position: relative;
  }
  .more-actions summary {
    list-style: none;
    cursor: pointer;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
  }
  .more-actions summary::-webkit-details-marker {
    display: none;
  }
  .menu {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0;
    z-index: 5;
    display: grid;
    min-width: 150px;
    padding: 6px;
    background: var(--surface);
    border: 1px solid var(--border-subtle);
    border-radius: 9px;
    box-shadow: var(--shadow-dialog);
  }
  .menu button {
    text-align: left;
  }
  .editor-tabs {
    margin: 10px 0;
  }
  @media (max-width: 520px) {
    .overlay {
      padding: 8px;
    }
    .editor {
      max-height: 94dvh;
      padding: 12px;
    }
    .row.icon-color {
      grid-template-columns: minmax(0, 1fr) 82px;
    }
  }
  .appearance-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 0 12px;
  }
</style>
