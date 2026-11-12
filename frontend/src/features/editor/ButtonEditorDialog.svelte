<script lang="ts">
  import SwitchField from '../../components/SwitchField.svelte';
  import GuideButton from '../learn/GuideButton.svelte';
  import Icon from '../../components/Icon.svelte';
  import { useTranslations, common, keys } from '../../lib/i18n';
  const t = useTranslations();
  let actionValid = $state(true);
  let moreOpen = $state(false);
  let moreActions = $state<HTMLDivElement>();
  let moreTrigger = $state<HTMLButtonElement>();

  import { modal } from '../../lib/modal';
  import { onDestroy, onMount, tick, untrack } from 'svelte';
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
  import ActionPickerModal from './ActionPickerModal.svelte';
  import { actionOptions, type ActionOption } from './action-options';
  import SegmentedControl from '../../components/SegmentedControl.svelte';
  import { fileSize, type MediaMetadata } from './audio';
  import ImageSourceFields from './ImageSourceFields.svelte';
  let {
    button = $bindable(),
    activeTab = $bindable('content'),
    suspended = false,
    tutorial,
    layout,
    catalog,
    assetUrls,
    refreshAssets,
    moveButton,
    removeButton,
    protectedSettings = false,
    duplicateButton,
    commitButton,
    close,
    attempt,
  }: {
    button: Button;
    activeTab?: string;
    suspended?: boolean;
    tutorial?:
      | {
          step: string;
          detail?: string | undefined;
          audio?: AudioBuffer | undefined;
          images?: Record<string, string> | undefined;
        }
      | undefined;
    layout: Layout;
    catalog: CatalogResponse | null;
    assetUrls: Record<string, string>;
    refreshAssets: (id: string) => Promise<void>;
    moveButton: (delta: number) => void;
    removeButton: () => void;
    protectedSettings?: boolean;
    duplicateButton: () => void;
    commitButton: () => void;
    close: () => void;
    attempt: (work: () => Promise<void>) => Promise<void>;
  } = $props();
  let visualSource = $state(button.icon.startsWith('asset:') ? 'image' : 'icon');
  let lastIcon = $state(button.icon.startsWith('asset:') ? 'icon:grid' : button.icon);
  let lastImage = $state(button.icon.startsWith('asset:') ? button.icon : '');
  let actionPickerOpen = $state(false);
  $effect(() => {
    if (tutorial) actionPickerOpen = tutorial.detail === 'action-picker';
  });
  let mobilePickerOpen = $state(false);
  let editorDialog: HTMLDialogElement;
  let editorBody: HTMLDivElement;
  let editorScrollBeforePicker = 0;
  function keyPickerStep(open: boolean) {
    if (open) {
      editorScrollBeforePicker = editorBody?.scrollTop ?? 0;
      activeTab = 'action';
      mobilePickerOpen = true;
      return;
    }
    mobilePickerOpen = false;
    void tick().then(() => {
      requestAnimationFrame(() => {
        if (editorDialog?.isConnected) editorBody.scrollTop = editorScrollBeforePicker;
      });
    });
  }
  const selectedActionId = $derived(
    button.action.type === 'command' ? `command:${button.action.command.type}` : button.action.type,
  );
  const selectedAction = $derived(actionOptions.find((option) => option.id === selectedActionId));
  const actionSchema = $derived(
    button.action.type === 'command'
      ? selectedAction?.schema
      : resolve({ $ref: '#/$defs/ButtonAction' }).oneOf?.find(
          (option) => option.properties?.type?.const === button.action.type,
        ),
  );
  const hasConfiguration = $derived(
    Object.keys(actionSchema?.properties ?? {}).some((key) => key !== 'type'),
  );
  let imageIds = $state<string[]>(untrack(() => Object.keys(tutorial?.images ?? {})));
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
  const pageIds = $derived(imageIds.slice(imagePage * 12, (imagePage + 1) * 12));
  const media = $derived((button.extensions.media ?? {}) as Record<string, MediaMetadata>);
  const imageMetadata = $derived(
    media.image?.source === button.icon.slice(6) ? media.image : undefined,
  );
  function saveMetadata(kind: string, metadata: MediaMetadata) {
    button.extensions.media = { ...media, [kind]: metadata };
  }
  function changeVisual(source: string) {
    if (visualSource === 'icon') lastIcon = button.icon;
    else if (button.icon.startsWith('asset:')) lastImage = button.icon;
    visualSource = source;
    button.icon = source === 'icon' ? lastIcon : lastImage;
  }
  onMount(() => {
    void request<ImageAssetList>('assets', 'ImageAssetList')
      .then((value) => {
        if (!destroyed) {
          imageIds = [...new Set([...imageIds, ...value.images])];
          liveIds = [...new Set([...liveIds, ...(value.live_images ?? [])])];
        }
      })
      .catch(() => {
        if (!destroyed) libraryError = 'icons.iconLibraryUnavailable';
      });
  });
  $effect(() => {
    void refreshTick;
    let stale = false;
    const borrowed = tutorial?.images ?? {};
    void libraryAssets.load(pageIds.filter((id) => !borrowed[id])).then((result) => {
      if (!stale && result) libraryUrls = { ...borrowed, ...result.urls };
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
        if (!disposed) formError = 'images.previewFailed';
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
    if (imageBusy || !actionValid) return;
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
  function selectAction(option: ActionOption) {
    if (!button) return;
    actionValid = true;
    const type = option.type;
    if (type === 'command' && option.schema) {
      const command = defaultValue(option.schema) as Command;
      if (command.type === 'play_sound') command.volume = 100;
      chooseCommand(command);
    } else if (type === 'workflow')
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
          args: defaultValue(pluginActionSchema(manifest, manifest.actions[0]!.id)) as Record<
            string,
            unknown
          >,
        };
    } else if (type === 'metric')
      button.action = { type, metric: 'cpu', target: '', interval_ms: 1000 };
    else if (type === 'folder')
      button.action = { type, folder_id: layout?.folders[0]?.id ?? 'home' };
    else button.action = { type } as ButtonAction;
  }
  function chooseCommand(c: Command) {
    if (!button) return;
    button.action = { type: 'command', command: c };
    if (c.type === 'plugin') {
      const m = catalog?.plugins[0];
      if (m) pluginCommand(m);
    }
  }
  async function image(file: File) {
    if (imageBusy) return;
    const candidate = button;
    imageBusy = true;
    try {
      const result = await upload(file);
      if (!destroyed && button === candidate && result.type === 'asset') {
        button.icon = lastImage = `asset:${result.id}`;
        saveMetadata('image', { source: result.id, name: file.name, size: file.size });
        imageIds = [result.id, ...imageIds.filter((id) => id !== result.id)];
        imagePage = 0;
      }
    } finally {
      if (!destroyed) imageBusy = false;
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
        if (button === candidate) {
          button.icon = lastImage = `asset:${result.id}`;
          saveMetadata('image', {
            source: result.id,
            name:
              source.type === 'url'
                ? source.url.split('/').pop()?.split('?')[0] || result.id
                : source.path.split(/[\\/]/).pop() || result.id,
          });
        }
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
      await importImage({ type: 'local', path: result.source.path });
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

<svelte:window
  onpointerdown={(event) => {
    if (moreOpen && !moreActions?.contains(event.target as Node)) moreOpen = false;
  }}
/>
<div class="overlay" hidden={suspended}>
  <dialog
    bind:this={editorDialog}
    use:modal
    oncancel={(event) => {
      event.preventDefault();
      if (moreOpen) {
        moreOpen = false;
        moreTrigger?.focus();
      } else close();
    }}
    aria-modal="true"
    aria-labelledby="button-title"
    class="editor"
    class:picker-step={mobilePickerOpen}
  >
    <div class="editor-header">
      <div class="heading">
        <h2 id="button-title">
          {mobilePickerOpen ? t('keyboard.chooseKeys') : t('editor.editButton')}
        </h2>
        <GuideButton
          guide={activeTab === 'appearance'
            ? 'button-appearance'
            : selectedActionId === 'command:key'
              ? 'keyboard'
              : selectedActionId === 'command:play_sound'
                ? 'audio'
                : activeTab === 'action'
                  ? 'actions'
                  : 'content'}
          label={t('editor.editButton')}
        />
        <button aria-label={t('editor.closeEditor')} onclick={close}><Icon name="close" /></button>
      </div>
      {#if !mobilePickerOpen}
        <div class="preview" data-guide="button-preview">
          <div
            class="deck-button preview-tile"
            role="img"
            aria-label={t('editor.buttonPreview')}
            style={`--button-color:${button.color};--button-foreground:${colors.foreground};--icon-size:${appearance.icon_size ?? 42}px;`}
          >
            <ButtonContent {button} assetUrl={previewUrl} showLabels={false} />
          </div>
          <strong class="preview-name">{button.label}</strong>
        </div>
      {/if}
    </div>
    <form onsubmit={apply}>
      {#if !mobilePickerOpen}
        <div role="tablist" aria-label={t('editor.editButton')} class="editor-tabs">
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
              onclick={() => (activeTab = tab)}
              >{t(
                (
                  {
                    content: keys.editor.content,
                    appearance: keys.editor.appearance,
                    action: keys.editor.action,
                  } as const
                )[tab as 'content' | 'appearance' | 'action'],
              )}</button
            >{/each}
        </div>
      {/if}
      <div class="editor-scroll" bind:this={editorBody}>
        {#if !mobilePickerOpen}
          <div
            role="tabpanel"
            id="editor-panel-content"
            aria-labelledby="editor-tab-content"
            hidden={activeTab !== 'content'}
          >
            <div class="basics-grid">
              <label data-guide="button-label"
                >{t('editor.label')}<input bind:value={button.label} maxlength="256" /></label
              >
              <span class="field-label">{t('editor.buttonVisual')}</span>
              <SegmentedControl
                label={t('editor.buttonVisual')}
                value={visualSource}
                options={[
                  { value: 'icon', label: t('editor.icon') },
                  { value: 'image', label: t('editor.visualImage') },
                ]}
                onchange={changeVisual}
              />
              {#if visualSource === 'icon'}
                <IconField value={button.icon} onSelect={(value) => (button.icon = value)} />
              {:else}
                <ImageSourceFields
                  guideSource={tutorial
                    ? tutorial.detail === 'image-url'
                      ? 'url'
                      : null
                    : undefined}
                  bind:imageUrl
                  bind:imagePath
                  bind:liveImage
                  busy={imageBusy}
                  filename={button.icon.startsWith('asset:')
                    ? (imageMetadata?.name ?? button.icon.slice(6))
                    : ''}
                  metadata={imageMetadata?.size
                    ? `${imageMetadata.name.split('.').pop()?.toUpperCase()} · ${fileSize(imageMetadata.size)}`
                    : ''}
                  preview={previewUrl}
                  upload={image}
                  importUrl={() =>
                    attempt(() =>
                      importImage({ type: 'url', url: imageUrl.trim(), live: liveImage }),
                    )}
                  importLocal={() =>
                    attempt(() => importImage({ type: 'local', path: imagePath.trim() }))}
                  chooseLocal={() => attempt(chooseLocalImage)}
                />
                {#if button.icon.startsWith('asset:') && liveIds.includes(button.icon.slice(6))}
                  <button type="button" disabled={imageBusy} onclick={() => attempt(refreshImage)}
                    >{t('images.refreshLiveImage')}</button
                  >
                {/if}
                {#if imageIds.length}
                  <h3 class="library-title">{t('images.imageLibrary')}</h3>
                  <div class="image-library">
                    {#each pageIds as id}<button
                        type="button"
                        aria-label={`${t('images.selectUploadedIcon')} ${id}`}
                        aria-pressed={button.icon === `asset:${id}`}
                        onclick={() => (button.icon = lastImage = `asset:${id}`)}
                      >
                        {#if libraryUrls[id]}<img
                            src={libraryUrls[id]}
                            alt=""
                            width="72"
                            height="72"
                          />{:else}<span class="image-loading" aria-hidden="true"></span>{/if}
                      </button>{/each}
                  </div>
                  {#if imageIds.length > 12}<div class="library-pages">
                      <button type="button" disabled={imagePage === 0} onclick={() => imagePage--}
                        >{t('common.previous')}</button
                      >
                      <button
                        type="button"
                        disabled={(imagePage + 1) * 12 >= imageIds.length}
                        onclick={() => imagePage++}>{t('common.next')}</button
                      >
                    </div>{/if}
                {/if}
                {#if libraryError}<p role="status">{t.message(libraryError)}</p>{/if}
              {/if}
            </div>
          </div>
          <div
            role="tabpanel"
            id="editor-panel-appearance"
            aria-labelledby="editor-tab-appearance"
            hidden={activeTab !== 'appearance'}
          >
            <label class="background-color"
              >{t('editor.backgroundColor')}<span class="color-row"
                ><input
                  class="color-input"
                  type="color"
                  bind:value={button.color}
                  aria-label={t('editor.backgroundColor')}
                /><input
                  aria-label={t('editor.colorHex')}
                  bind:value={button.color}
                  maxlength="7"
                  required
                /></span
              ></label
            >
            <div class="appearance-fields">
              <NumberField
                label={t('editor.columnSpan')}
                value={Number(appearance.columns ?? 1)}
                min={1}
                max={layout.columns}
                onchange={(value) => setAppearance('columns', value)}
              />
              <NumberField
                label={t('editor.rowSpan')}
                value={Number(appearance.rows ?? 1)}
                min={1}
                max={128}
                onchange={(value) => setAppearance('rows', value)}
              />
              <NumberField
                label={t('editor.iconSize')}
                value={Number(appearance.icon_size ?? 42)}
                min={0}
                max={200}
                onchange={(value) => setAppearance('icon_size', value)}
              />
              <SwitchField
                label={t('editor.showLabel')}
                checked={appearance.show_label !== false}
                onchange={(checked) => setAppearance('show_label', checked)}
              />
            </div>
          </div>
        {/if}
        <div
          role="tabpanel"
          id="editor-panel-action"
          data-guide="button-action"
          aria-labelledby={mobilePickerOpen ? 'button-title' : 'editor-tab-action'}
          hidden={activeTab !== 'action' && !mobilePickerOpen}
        >
          {#if !mobilePickerOpen}
            <div class="action-summary">
              <Icon name={selectedAction?.icon ?? 'settings'} />
              <span>{selectedAction ? t.message(selectedAction.label) : button.action.type}</span>
              <button
                type="button"
                disabled={protectedSettings}
                title={protectedSettings ? t('editor.settingsProtected') : undefined}
                onclick={() => (actionPickerOpen = true)}>{t('actions.changeAction')}</button
              >
            </div>
            {#if !hasConfiguration}<p class="field-help">
                {t('actions.noConfigurationNeeded')}
              </p>{/if}
            {#if actionPickerOpen}<ActionPickerModal
                value={selectedActionId}
                close={() => (actionPickerOpen = false)}
                select={selectAction}
              />{/if}
          {/if}
          {#if button.action.type === 'workflow' || button.action.type === 'script' || button.action.type === 'plugin'}
            <Fields
              schema={resolve({ $ref: '#/$defs/ButtonAction' }).oneOf?.find(
                (option) => option.properties?.type?.const === button!.action.type,
              ) ?? {}}
              value={button.action}
              onchange={(action) => {
                if (button) button.action = action as ButtonAction;
              }}
              label={t('editor.action')}
            />
          {:else if button.action.type === 'metric'}
            <label
              >{t('editor.metric')}<select bind:value={button.action.metric}
                >{#each ['cpu', 'memory', 'gpu', 'gpu_memory', 'disk', 'clock'] as metric}<option
                    value={metric}>{metric}</option
                  >{/each}</select
              ></label
            >
            <label
              >{t('editor.deviceNameOrIndexEmptyUsesDefault')}<input
                bind:value={button.action.target}
              /></label
            >
            <NumberField
              label={t('fields.intervalMs')}
              value={button.action.interval_ms}
              min={250}
              max={3600000}
              onchange={(value) => {
                if (button.action.type === 'metric') button.action.interval_ms = value;
              }}
            />
          {:else if button.action.type === 'folder'}<label
              >{t('editor.destination')}<select bind:value={button.action.folder_id}
                >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option
                  >{/each}</select
              ></label
            >{:else if button.action.type === 'command'}<ActionFields
              configurationOnly
              {tutorial}
              active={!suspended && (activeTab === 'action' || mobilePickerOpen)}
              onKeyPickerStep={keyPickerStep}
              mediaMetadata={media[
                button.action.command.type === 'play_sound' ? 'audio' : 'script'
              ]}
              onMediaMetadata={(metadata) => {
                if (button.action.type === 'command')
                  saveMetadata(
                    button.action.command.type === 'play_sound' ? 'audio' : 'script',
                    metadata,
                  );
              }}
              onValidity={(valid) => (actionValid = valid)}
              value={button.action.command}
              onchange={(c) => chooseCommand(c as Command)}
            />{#if button.action.command.type === 'plugin'}<label
                >{t('editor.plugin')}<select
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
                  >{t('editor.pluginAction')}<select
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
                    if (
                      button?.action.type === 'command' &&
                      button.action.command.type === 'plugin'
                    )
                      button.action.command.args = args as Record<string, unknown>;
                  }}
                  label={t('editor.pluginArguments')}
                />{:else}<p>{t('editor.noV2PluginsAreInstalled')}</p>{/if}{/if}{/if}
        </div>
        {#if formError}<p role="alert">{t.message(formError)}</p>{/if}
      </div>
      {#if !mobilePickerOpen}<div class="actions">
          <div class="more-actions" bind:this={moreActions}>
            <button
              bind:this={moreTrigger}
              type="button"
              class="more-trigger"
              aria-label={t('editor.moreButtonActions')}
              aria-expanded={moreOpen}
              aria-controls="button-more-actions"
              onclick={() => (moreOpen = !moreOpen)}
              ><Icon name="more" size={20} /><span>{t('editor.editorMore')}</span></button
            >
            {#if moreOpen}<div
                id="button-more-actions"
                class="menu"
                role="group"
                aria-label={t('editor.moreButtonActions')}
              >
                <button type="button" onclick={() => moveButton(-1)}>{t('editor.moveLeft')}</button>
                <button type="button" onclick={() => moveButton(1)}>{t('editor.moveRight')}</button>
                <button type="button" onclick={duplicateButton}>{t('editor.duplicate')}</button>
                <button
                  type="button"
                  class="danger"
                  disabled={protectedSettings || button.action.type === 'settings'}
                  title={protectedSettings || button.action.type === 'settings'
                    ? t('editor.settingsProtected')
                    : undefined}
                  onclick={removeButton}>{t(common.delete)}</button
                >
              </div>{/if}
          </div>
          <span class="spacer"></span><button type="button" onclick={close}
            >{t(common.cancel)}</button
          ><button
            data-guide="button-apply"
            type="submit"
            class="primary"
            disabled={imageBusy || !actionValid}>{t('editor.applyToDraft')}</button
          >
        </div>
      {/if}
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
    display: flex;
    flex-direction: column;
    width: min(620px, 100%);
    max-height: 92dvh;
    padding: 0;
    overflow: hidden;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow-dialog);
  }
  .editor::backdrop {
    background: transparent;
  }
  .editor-header {
    padding: 16px 22px 0;
    flex-shrink: 0;
  }
  .heading {
    margin: 0 0 12px;
    flex-wrap: nowrap;
  }
  h2 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 600;
  }
  .heading h2 {
    flex: 1;
  }
  .heading button {
    display: grid;
    place-items: center;
    width: 44px;
    min-height: 44px;
    padding: 5px;
    border: 0;
    background: transparent;
  }
  .preview {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 10px;
  }
  .preview-tile {
    width: 56px;
    height: 56px;
    min-height: 56px;
    flex: 0 0 56px;
    border-radius: 10px;
    padding: 7px;
    --icon-size: 32px !important;
    overflow: hidden;
  }
  .preview-name {
    border-left: 1px solid var(--border);
    padding-left: 16px;
    font-size: 1rem;
    font-weight: 550;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  form {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .editor-tabs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin: 0 22px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .editor-tabs button {
    min-width: 0;
    padding: 11px 6px;
    border: 0;
    border-bottom: 2px solid transparent;
    border-radius: 8px 8px 0 0;
    font-size: 0.85rem;
    color: var(--text-muted);
    background: transparent;
  }
  .editor-tabs [aria-selected='true'] {
    background: var(--accent-surface);
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .editor-scroll {
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    padding: 16px 22px;
  }
  .basics-grid {
    display: grid;
    gap: 14px;
  }
  .basics-grid > label {
    margin: 0 0 2px;
  }
  .field-label {
    font-size: 0.85rem;
    font-weight: 550;
    margin-bottom: -6px;
  }
  .background-color {
    margin: 0 0 18px;
  }
  .color-row {
    display: flex;
    align-items: center;
    width: min(240px, 100%);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    overflow: hidden;
    background: var(--canvas);
  }
  .color-row input {
    border: 0;
    background: transparent;
  }
  .color-row .color-input {
    width: 44px;
    height: 40px;
    flex: 0 0 44px;
    padding: 4px;
  }
  .image-library {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(68px, 1fr));
    gap: 10px;
  }
  .image-library button {
    display: grid;
    place-items: center;
    padding: 3px;
    aspect-ratio: 1;
    min-width: 0;
    overflow: hidden;
  }
  .image-library button[aria-pressed='true'] {
    border-color: var(--accent);
    outline: 1px solid var(--accent);
  }
  .image-library img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 5px;
  }
  .image-loading {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--surface-raised);
  }
  .library-title {
    margin: 4px 0 -4px;
    font-size: 0.85rem;
    font-weight: 550;
  }
  .library-pages {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .library-pages button {
    font-size: 0.8rem;
  }
  .action-summary {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    margin-bottom: 14px;
    background: var(--canvas);
  }
  .action-summary span {
    flex: 1;
    min-width: 0;
    font-size: 0.85rem;
    font-weight: 550;
  }
  .action-summary button {
    font-size: 0.75rem;
    padding: 7px 10px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 22px;
    border-top: 1px solid var(--border-subtle);
    flex-shrink: 0;
    background: var(--surface);
  }
  .actions > button {
    font-size: 0.8rem;
    padding: 10px 12px;
  }
  .spacer {
    flex: 1;
  }
  .more-actions {
    position: relative;
  }
  .more-trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    border: 0;
    background: transparent;
    font-size: 0.8rem;
  }
  .menu {
    position: absolute;
    bottom: calc(100% + 14px);
    left: 0;
    z-index: 5;
    display: grid;
    min-width: 160px;
    padding: 6px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: 9px;
    box-shadow: var(--shadow-dialog);
  }
  .menu button {
    text-align: left;
    font-size: 0.85rem;
    border: 0;
  }
  .appearance-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 0 12px;
  }
  @media (max-width: 520px) {
    .overlay {
      padding: 8px;
    }
    .editor {
      max-height: 94dvh;
    }
    .editor-header {
      padding: 16px 14px 0;
    }
    .editor-tabs {
      margin: 0 14px;
    }
    .editor-scroll {
      padding: 16px 14px;
    }
    .actions {
      padding: 12px 14px;
      gap: 6px;
    }
    .more-trigger span {
      display: none;
    }
    .actions > button {
      padding: 10px;
    }
  }
</style>
