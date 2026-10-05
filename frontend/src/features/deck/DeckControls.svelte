<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
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
  }: {
    interactions: DeckInteractions;
    canEdit: boolean;
    back: () => void;
    home: () => void;
    settings: () => void;
    reload: () => void;
  } = $props();
</script>

{#if interactions.controls || interactions.help}
  <Modal
    label={t(interactions.help ? 'ui_keyboard_shortcuts' : 'ui_deck_controls')}
    close={() => {
      interactions.controls = false;
      interactions.help = false;
    }}
  >
    <div role="tablist" aria-label={t('ui_deck_controls')} class="control-tabs">
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
          }}>{t(tab === 'help' ? 'ui_shortcuts' : 'ui_deck_controls')}</button
        >{/each}
    </div>
    <button
      aria-label={t(interactions.help ? 'ui_close' : 'ui_close_controls')}
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
        onclick={() => {
          back();
          interactions.controls = false;
        }}><Icon name="back" />{t('ui_back')}</button
      >
      <button onclick={home}><Icon name="home" />{t('ui_home')}</button>
      {#if canEdit}<button onclick={settings}><Icon name="settings" />{t('ui_settings')}</button
        >{/if}
      <button
        onclick={() => {
          interactions.controls = false;
          reload();
        }}><Icon name="refresh" />{t('ui_reload')}</button
      >
      <p>
        {t(
          canEdit
            ? 'ui_right_click_or_hold_the_deck_for_controls_q_toggles_editing_f1_shows_all_shortcu'
            : 'ui_controls_hint_viewer',
        )}
      </p>
    </div>
    <div
      role="tabpanel"
      id="controls-panel-help"
      aria-labelledby="controls-tab-help"
      hidden={!interactions.help}
    >
      <h2>{t('ui_shortcuts')}</h2>
      {#if canEdit}<p>{t('ui_q_toggle_edit_mode_saves_changes_when_leaving')}</p>
        <p>{t('ui_ctrl_settings')}</p>{/if}
      <p>{t('ui_f1_show_or_hide_shortcuts')}</p>
      <p>{t('ui_alt_left_return_to_home_folder')}</p>
      <p>{t('ui_escape_close_a_dialog_or_settings')}</p>
      <p>{t('ui_right_click_or_touch_and_hold_deck_controls')}</p>
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
