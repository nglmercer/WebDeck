<script lang="ts">
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

{#if interactions.controls}
  <Modal label={t('ui_deck_controls')} close={() => (interactions.controls = false)}>
    <button aria-label={t('ui_close_controls')} onclick={() => (interactions.controls = false)}
      >×</button
    >
    <button
      onclick={() => {
        back();
        interactions.controls = false;
      }}>{t('ui_back')}</button
    >
    <button onclick={home}>{t('ui_home')}</button>
    {#if canEdit}<button onclick={settings}>{t('ui_settings')}</button>{/if}
    <button
      onclick={() => {
        interactions.controls = false;
        reload();
      }}>{t('ui_reload')}</button
    >
    <p>
      {t(
        canEdit
          ? 'ui_right_click_or_hold_the_deck_for_controls_q_toggles_editing_f1_shows_all_shortcu'
          : 'ui_controls_hint_viewer',
      )}
    </p>
  </Modal>
{/if}
{#if interactions.help}
  <Modal label={t('ui_keyboard_shortcuts')} close={() => (interactions.help = false)}>
    <button onclick={() => (interactions.help = false)}>{t('ui_close')}</button>
    <h2>{t('ui_shortcuts')}</h2>
    {#if canEdit}<p>{t('ui_q_toggle_edit_mode_saves_changes_when_leaving')}</p>
      <p>{t('ui_ctrl_settings')}</p>{/if}
    <p>{t('ui_f1_show_or_hide_shortcuts')}</p>
    <p>{t('ui_alt_left_return_to_home_folder')}</p>
    <p>{t('ui_escape_close_a_dialog_or_settings')}</p>
    <p>{t('ui_right_click_or_touch_and_hold_deck_controls')}</p>
  </Modal>
{/if}
