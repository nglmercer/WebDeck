<script lang="ts">
  import { provideTranslations } from './lib/i18n';

  import { onMount } from 'svelte';
  import type { Button } from './lib/contracts';
  import { ApiError, setToken } from './lib/api/client';
  import { Session } from './lib/session.svelte';
  import { ButtonDraft } from './features/editor/button-draft.svelte';
  import Modal from './components/Modal.svelte';
  import ButtonEditorDialog from './features/editor/ButtonEditorDialog.svelte';
  import DeckView from './features/deck/DeckView.svelte';
  import SettingsView from './features/settings/SettingsView.svelte';
  import PairingView from './features/pairing/PairingView.svelte';
  import PairingApprovals from './features/pairing/PairingApprovals.svelte';
  import Feedback from './components/Feedback.svelte';
  import DeckControls from './features/deck/DeckControls.svelte';
  import FirstUseHint from './features/deck/FirstUseHint.svelte';
  import EditorToolbar from './features/editor/EditorToolbar.svelte';
  import { UsageMonitor } from './features/deck/usage.svelte';
  import { Navigation } from './lib/navigation.svelte';
  import { DeckInteractions } from './features/deck/interactions.svelte';
  import { Execution, permissionReason } from './features/deck/execution.svelte';
  import { appearanceOf } from './features/editor/appearance';
  let error = $state(''),
    notice = $state(''),
    editing = $state(false),
    discarding = $state(false),
    settingsTab = $state('appearance'),
    transport = $state<'http' | 'socket'>('http');
  const session = new Session({
    error: (message) => (error = message),
    notice: (message) => (notice = message),
  });
  const layout = $derived(session.layout);
  const current = $derived(
    layout?.folders.find((folder) => folder.id === navigation.active) ?? layout?.folders[0],
  );
  const draft = new ButtonDraft(
    () => session.editor,
    () => current,
  );
  const execution = new Execution();
  const t = provideTranslations(() => session.translations);
  $effect(() => {
    document.documentElement.lang = session.translations.lang_code || 'en';
  });
  const navigation = new Navigation(
    () => layout,
    () => !!session.editor,
  );
  const interactions = new DeckInteractions({
    enabled: () => !!session.deck && navigation.panel === 'deck',
    hasDialog: () => !!draft.button,
    closeEditor: () => {
      closeButtonEditor();
      if (!draft.button && navigation.panel === 'settings') navigation.navigate(navigation.active);
    },
    toggleEdit: () => {
      void attempt(toggleEdit);
    },
    settings: () => openSettings(),
    back: () => navigation.back(),
  });
  const appearance = $derived(appearanceOf(layout));
  const monitor = new UsageMonitor(
    () => current?.buttons ?? [],
    () => navigation.panel === 'deck',
  );
  function navigate(folder: string, replace = false) {
    navigation.navigate(folder, replace);
    interactions.controls = false;
  }
  function openSettings() {
    navigation.settings();
    interactions.controls = false;
  }
  function closeButtonEditor() {
    if (draft.dirty) discarding = true;
    else draft.button = null;
  }
  async function toggleEdit() {
    if (!session.editor) return;
    if (editing && (session.editor.dirty || session.editor.saving)) {
      await persist();
      if (session.editor.dirty)
        throw new Error('Changes were added during saving. Save again before leaving edit mode.');
    }
    interactions.controls = false;
    navigation.panel = 'deck';
    editing = !editing;
  }
  async function attempt(work: () => Promise<void>) {
    error = '';
    try {
      await work();
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) session.pairing = true;
      if (e instanceof ApiError && e.status === 409 && session.editor)
        session.editor.conflict = true;
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function load() {
    error = '';
    if (await session.load()) navigation.route();
  }
  async function assets() {
    await session.assets();
  }
  async function invoke(b: Button) {
    if (interactions.controls) return;
    if (editing) {
      if (b.action.type === 'folder') {
        navigate(b.action.folder_id);
        return;
      }
      if (b.action.type === 'back') {
        navigation.back();
        return;
      }
      draft.open(b);
      return;
    }
    if (execution.running[b.id]) return;
    const a = b.action;
    if (a.type === 'folder') {
      navigate(a.folder_id);
      return;
    }
    if (a.type === 'reload') {
      await load();
      return;
    }
    if (a.type === 'fullscreen') {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await document.documentElement.requestFullscreen();
      return;
    }
    if (a.type === 'settings') {
      openSettings();
      return;
    }
    if (a.type === 'back') {
      navigation.back();
      return;
    }
    if (a.type === 'edit') {
      await toggleEdit();
      return;
    }
    if (a.type === 'none') return;
    if (a.type === 'usage' || a.type === 'metric') {
      await monitor.refresh();
      return;
    }
    await execution.run(b, transport);
  }
  function newFolder() {
    if (session.editor && current) navigate(session.editor.createFolder(current.id).id);
  }
  function removeFolder() {
    if (!session.editor || !current || current.id === navigation.root || current.id === 'home')
      return;
    void attempt(async () => {
      session.editor!.removeFolder(current!.id);
      navigate(session.editor!.draft.layout.folders[0]!.id, true);
    });
  }
  async function persist() {
    if (!session.editor) return;
    notice = '';
    await session.editor.persist();
    notice = t('ui_saved');
    await session.refreshAfterSave();
  }
  onMount(() => {
    void load();
    return () => {
      session.dispose();
      interactions.dispose();
      execution.dispose();
    };
  });
</script>

<svelte:window
  onbeforeunload={(event) => {
    if (session.editor?.dirty || draft.dirty) {
      event.preventDefault();
      event.returnValue = '';
    }
  }}
  onkeydown={interactions.keyboard}
  onpopstate={() => navigation.route()}
  onhashchange={() => navigation.route()}
  oncontextmenu={interactions.context}
  onpointerdown={interactions.startHold}
  onpointerup={interactions.stopHold}
  onpointermove={interactions.moveHold}
  onpointercancel={interactions.stopHold}
/>

<svelte:head
  ><title>{t('ui_webdeck_v2')}</title>{#each session.themeUrls as href}<link
      rel="stylesheet"
      {href}
    />{/each}</svelte:head
>
<div
  class="application"
  style:--feedback-bottom={`${navigation.panel === 'deck' && session.editor ? (editing ? 180 : 100) : 12}px`}
  style:--feedback-mobile-bottom={`${navigation.panel === 'deck' && session.editor ? 100 : 12}px`}
  style:background-image={session.backgroundUrls[0]
    ? `linear-gradient(#10121acc,#10121acc), url("${session.backgroundUrls[0]}")`
    : undefined}
>
  <Feedback bind:error bind:notice>
    {#if !session.pairing && !session.loading && session.deck && navigation.panel === 'deck' && !editing}<FirstUseHint
        canEdit={!!session.editor}
      />{/if}
  </Feedback>
  {#if session.pairing}<PairingView
      connect={async (token, remember) => {
        setToken(token, remember);
        await attempt(load);
        return !session.pairing && !!session.deck;
      }}
    />
  {:else if session.loading}<main aria-busy="true"><p>{t('ui_loading_webdeck')}</p></main>
  {:else if session.deck}
    <main>
      {#if navigation.panel === 'deck'}
        <h1 class="sr-only">{current?.label ?? 'Deck'}</h1>
        {#if current && session.editor}<EditorToolbar
            {editing}
            editor={session.editor}
            {current}
            rootId={navigation.root}
            add={newFolder}
            settings={openSettings}
            remove={removeFolder}
            save={() => {
              void attempt(persist);
            }}
            done={() => {
              void attempt(toggleEdit);
            }}
          />{/if}
        {#if layout}<DeckView
            {layout}
            folderId={current?.id ?? ''}
            buttons={current?.buttons ?? []}
            {appearance}
            {editing}
            running={execution.running}
            outcomes={execution.states}
            outcomeMessages={execution.messages}
            assetUrls={session.assetUrls}
            usage={monitor.value}
            now={monitor.now}
            usageStatus={monitor.status}
            canRun={(b) => !permissionReason(b, session.deck, editing, t)}
            reason={(b) => permissionReason(b, session.deck, editing, t)}
            invoke={(b) => {
              void attempt(() => invoke(b));
            }}
            edit={(button) => draft.open(button)}
            remove={(button) => session.editor?.removeButton(button.id)}
            add={(cell) => draft.create(cell)}
          />{/if}
      {:else if navigation.panel === 'settings' && session.editor}
        <SettingsView
          bind:activeTab={settingsTab}
          editor={session.editor}
          languages={session.languages}
          bind:transport
          {assets}
          {attempt}
          {persist}
          back={() => navigate(navigation.active)}
          reload={() => attempt(load)}
          notify={(message) => (notice = message)}
        />
      {/if}
    </main>
  {:else}
    <main>
      <section role="status">
        <h1>{t('ui_loading_failed')}</h1>
        <p>{t('ui_loading_failed_help')}</p>
        <button
          onclick={() => {
            void load();
          }}>{t('ui_reload')}</button
        >
      </section>
    </main>
  {/if}
</div>
<DeckControls
  {interactions}
  canEdit={!!session.editor}
  back={() => navigation.back()}
  home={() => navigate(navigation.root)}
  settings={openSettings}
  reload={() => {
    void attempt(load);
  }}
/>
{#if draft.button && layout}
  <ButtonEditorDialog
    bind:button={draft.button}
    bind:buttonFolder={draft.folder}
    {layout}
    catalog={session.catalog}
    assetUrls={session.assetUrls}
    refreshAssets={(id) => session.refreshAsset(id)}
    moveButton={(delta) => draft.move(delta)}
    removeButton={() => draft.remove()}
    duplicateButton={() => {
      draft.duplicate();
      void attempt(assets);
    }}
    commitButton={() => {
      draft.commit();
      void attempt(assets);
    }}
    {attempt}
    close={closeButtonEditor}
  />
{/if}

{#if discarding}
  <Modal label={t('ui_unsaved_button_changes')} close={() => (discarding = false)}>
    <h2>{t('ui_unsaved_button_changes')}</h2>
    <p>{t('ui_discard_button_help')}</p>
    <button class="primary" onclick={() => (discarding = false)}>{t('ui_keep_editing')}</button>
    <button
      class="danger"
      onclick={() => {
        discarding = false;
        draft.button = null;
      }}>{t('ui_discard_changes')}</button
    >
  </Modal>
{/if}

{#if session.deck?.can_edit}<PairingApprovals
    capabilities={session.deck.capabilities}
    suggested={Object.values(session.deck.button_capabilities)}
    {attempt}
  />{/if}
