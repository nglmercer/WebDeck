<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../../components/Icon.svelte';
  import { useTranslations, common } from '../../lib/i18n';
  const t = useTranslations();

  import Modal from '../../components/Modal.svelte';
  import type { DeckInteractions } from './interactions.svelte';
  let {
    interactions,
    canEdit,
    back,
    home,
    settings,
    reload,
    learn,
    edit,
    connect,
    children,
  }: {
    interactions: DeckInteractions;
    canEdit: boolean;
    back: () => void;
    home: () => void;
    settings: () => void;
    reload: () => void;
    learn: () => void;
    edit: () => void;
    connect: () => void;
    children?: Snippet;
  } = $props();
</script>

{#if interactions.controls || interactions.help}
  <Modal
    label={t(interactions.help ? 'deck.keyboardShortcuts' : 'deck.deckControls')}
    close={() => {
      interactions.controls = false;
      interactions.help = false;
    }}
  >
    <div
      data-guide="controls-menu"
      role="tablist"
      aria-label={t('deck.deckControls')}
      class="control-tabs"
    >
      {#each ['controls', 'help'] as tab}<button
          role="tab"
          id={`controls-tab-${tab}`}
          aria-controls={`controls-panel-${tab}`}
          aria-selected={tab === 'help' ? interactions.help : !interactions.help}
          tabindex={(tab === 'help') === interactions.help ? 0 : -1}
          onclick={() => {
            interactions.help = tab === 'help';
            interactions.controls = tab === 'controls';
          }}
          onkeydown={(event) => {
            if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) {
              event.preventDefault();
              interactions.help =
                event.key === 'Home' ? false : event.key === 'End' ? true : !interactions.help;
              interactions.controls = !interactions.help;
              document
                .getElementById(`controls-tab-${interactions.help ? 'help' : 'controls'}`)
                ?.focus();
            }
          }}>{t(tab === 'help' ? 'deck.shortcuts' : 'deck.deckControls')}</button
        >{/each}
    </div>
    <button
      aria-label={t(interactions.help ? 'common.close' : 'deck.closeControls')}
      onclick={() => {
        interactions.controls = false;
        interactions.help = false;
      }}><Icon name="close" /></button
    >
    <div
      role="tabpanel"
      id="controls-panel-controls"
      aria-labelledby="controls-tab-controls"
      hidden={interactions.help}
      class="control-actions"
    >
      <button
        data-guide="controls-navigation"
        onclick={() => {
          back();
          interactions.controls = false;
        }}><Icon name="back" />{t(common.back)}</button
      >
      <button onclick={home}><Icon name="home" />{t(common.home)}</button>
      {#if canEdit}<button
          onclick={() => {
            interactions.controls = false;
            edit();
          }}><Icon name="edit" />{t(common.edit)}</button
        >
        <button onclick={connect}><Icon name="devices" />{t('connections.connectDevice')}</button>
        <button onclick={settings}><Icon name="settings" />{t(common.settings)}</button>{/if}
      <button
        onclick={() => {
          interactions.controls = false;
          reload();
        }}><Icon name="refresh" />{t(common.reload)}</button
      >
      {@render children?.()}
      <button onclick={learn}><Icon name="grid" />{t('learn.learn')}</button>
      <p>
        {t(canEdit ? 'deck.controlsHelp' : 'deck.viewerHelp')}
      </p>
    </div>
    <div
      role="tabpanel"
      id="controls-panel-help"
      aria-labelledby="controls-tab-help"
      hidden={!interactions.help}
    >
      <h2>{t('deck.shortcuts')}</h2>
      {@render children?.()}
      <button onclick={learn}>{t('learn.learn')}</button>
      {#if canEdit}<p>{t('deck.hints.edit')}</p>
        <p>{t('deck.hints.settings')}</p>{/if}
      <p>{t('deck.hints.help')}</p>
      <p>{t('deck.hints.home')}</p>
      <p>{t('deck.hints.close')}</p>
      <p>{t('deck.hints.controls')}</p>
    </div>
  </Modal>
{/if}

<style>
  .control-tabs {
    display: flex;
    gap: 6px;
  }
  .control-tabs button {
    flex: 1;
  }
  .control-tabs [aria-selected='true'] {
    background: var(--accent-surface);
  }
  .control-actions {
    display: grid;
    gap: 10px;
  }
  .control-actions button {
    display: flex;
    align-items: center;
    gap: 12px;
  }
</style>
