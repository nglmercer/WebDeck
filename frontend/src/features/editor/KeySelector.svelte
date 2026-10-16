<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { useTranslations } from '../../lib/i18n';
  import { request } from '../../lib/api/http';
  import type { KeyboardCapabilities } from '../../lib/contracts';
  import KeyPickerPanel from './KeyPickerPanel.svelte';
  import {
    captureChord,
    hostStatusMessage,
    isModifier,
    keyLabel,
    keyUnavailable,
    normalizeKey,
    sameKeys,
    validatePickerChordForHost,
  } from './keys';

  const t = useTranslations();
  let {
    value,
    onchange,
    active = true,
    onMobileStep = undefined,
  }: {
    value: string[];
    onchange: (keys: string[]) => void;
    active?: boolean;
    onMobileStep?: ((open: boolean) => void) | undefined;
  } = $props();

  type RecorderState = 'idle' | 'listening' | 'captured' | 'error';
  type PickerMode = 'popover' | 'step';
  let capabilities = $state<KeyboardCapabilities | undefined>();
  let recorderState = $state<RecorderState>('idle');
  let recorderMessage = $state('');
  let recordingTimer: ReturnType<typeof setTimeout> | undefined;
  let pickerOpen = $state(false);
  let pickerMode = $state<PickerMode>('popover');
  let candidate = $state<string[]>([]);
  let openingKeys: string[] = [];
  let advanced = $state(false);
  let panel = $state<HTMLDivElement>();
  let recordButton: HTMLButtonElement;
  let chooseButton: HTMLButtonElement;
  let placementListening = false;
  let focusExitTimer: ReturnType<typeof setTimeout> | undefined;
  let outsidePointerDown = false;
  const panelId = `key-picker-${Math.random().toString(36).slice(2)}`;

  const summary = $derived(value.map(keyLabel).join(' + '));
  const canUseCandidate = $derived(
    !validatePickerChordForHost(candidate, advanced, capabilities) &&
      candidate.some((key) => !isModifier(key)),
  );

  onMount(() => {
    const controller = new AbortController();
    request<KeyboardCapabilities>('keyboard/capabilities', 'KeyboardCapabilities', {
      signal: controller.signal,
    })
      .then((result) => (capabilities = result))
      .catch(() => {});
    return () => controller.abort();
  });

  function clearRecordingTimer() {
    clearTimeout(recordingTimer);
    recordingTimer = undefined;
  }

  function stopRecording() {
    clearRecordingTimer();
    if (recorderState !== 'idle') {
      recorderState = 'idle';
      recorderMessage = '';
    }
  }

  function showRecorderError(messageKey: string) {
    clearRecordingTimer();
    recorderState = 'error';
    recorderMessage = t(messageKey);
  }

  function startRecording() {
    if (!active) return;
    if (pickerOpen) closePicker(true);
    clearRecordingTimer();
    recorderMessage = t('ui_keypicker_record_prompt');
    recorderState = 'listening';
    recordButton?.focus({ preventScroll: true });
    recordingTimer = setTimeout(() => {
      recordingTimer = undefined;
      showRecorderError('ui_keypicker_record_timeout');
    }, 10000);
  }

  function recorderKeydown(event: KeyboardEvent) {
    if (recorderState !== 'listening') return;
    event.preventDefault();
    event.stopPropagation();
    if (event.isComposing) return;
    if (event.key === 'Escape') {
      stopRecording();
      return;
    }
    if (event.repeat || ['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) return;
    if (event.key === 'AltGraph' || event.getModifierState?.('AltGraph')) {
      showRecorderError('ui_keypicker_altgraph_unavailable');
      return;
    }
    if (event.code.startsWith('Numpad') && !/^Numpad[0-9]$/.test(event.code)) {
      showRecorderError('ui_keypicker_capture_unavailable');
      return;
    }

    const chord = captureChord(event);
    if (!chord) {
      showRecorderError('ui_keypicker_capture_unavailable');
      return;
    }
    if (chord.some((key) => keyUnavailable(key, capabilities))) {
      showRecorderError('ui_keypicker_capture_unavailable');
      return;
    }
    const error = validatePickerChordForHost(chord, false, capabilities);
    if (error) {
      showRecorderError('ui_keypicker_capture_unavailable');
      return;
    }

    clearRecordingTimer();
    onchange(chord.map((key) => normalizeKey(key)!));
    recorderState = 'captured';
    recorderMessage = t('ui_keypicker_recorded');
  }

  function removePlacementListeners() {
    if (!placementListening) return;
    window.removeEventListener('resize', positionPopover);
    window.removeEventListener('scroll', positionPopover, true);
    document.removeEventListener('pointerdown', noteOutsidePointer, true);
    placementListening = false;
    outsidePointerDown = false;
  }

  function noteOutsidePointer(event: PointerEvent) {
    if (pickerOpen && panel && !panel.contains(event.target as Node)) outsidePointerDown = true;
  }

  function positionPopover() {
    if (!pickerOpen || pickerMode !== 'popover' || !panel || !chooseButton) return;
    const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
    const viewportHeight = document.documentElement.clientHeight || window.innerHeight;
    const margin = 12;
    const width = Math.min(420, viewportWidth - margin * 2);
    panel.style.width = `${width}px`;
    panel.style.maxHeight = `${Math.min(480, viewportHeight * 0.7)}px`;

    const anchor = chooseButton.getBoundingClientRect();
    const bounds = panel.getBoundingClientRect();
    const gap = 8;
    const below = viewportHeight - anchor.bottom - gap - margin;
    const above = anchor.top - gap - margin;
    const placeAbove = bounds.height > below && above > below;
    const top = placeAbove ? anchor.top - bounds.height - gap : anchor.bottom + gap;
    const left = Math.max(margin, Math.min(anchor.left, viewportWidth - width - margin));
    panel.style.left = `${left}px`;
    panel.style.top = `${Math.max(margin, Math.min(top, viewportHeight - bounds.height - margin))}px`;
  }

  function assertPopoverReachable(popover: HTMLDivElement) {
    const rect = popover.getBoundingClientRect();
    if (!rect.width || !rect.height) throw new Error('Popover has no visible bounds.');
    const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    if (!hit || !popover.contains(hit))
      throw new Error('Popover is not in the active visual layer.');
  }

  function closePicker(restoreFocus: boolean) {
    if (!pickerOpen) return;
    const wasStep = pickerMode === 'step';
    pickerOpen = false;
    clearTimeout(focusExitTimer);
    focusExitTimer = undefined;
    removePlacementListeners();
    if (panel?.matches(':popover-open')) panel.hidePopover();
    candidate = [];
    openingKeys = [];
    if (wasStep) onMobileStep?.(false);
    if (restoreFocus) {
      void tick().then(() => {
        if (chooseButton?.isConnected) chooseButton.focus({ preventScroll: true });
      });
    }
  }

  function popoverToggled(event: ToggleEvent) {
    if (event.newState === 'closed' && pickerOpen && pickerMode === 'popover') closePicker(true);
  }

  function pickerKeydown(event: KeyboardEvent) {
    if (pickerMode === 'step' && event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      closePicker(true);
    }
  }

  function queueFocusExit(event: FocusEvent) {
    if (pickerMode !== 'popover' || !pickerOpen) return;
    if (outsidePointerDown) return;
    if (event.relatedTarget instanceof Node && panel?.contains(event.relatedTarget)) return;
    clearTimeout(focusExitTimer);
    focusExitTimer = setTimeout(() => {
      focusExitTimer = undefined;
      if (pickerOpen && panel && !panel.contains(document.activeElement)) closePicker(false);
    }, 0);
  }

  async function openPicker() {
    if (!active || pickerOpen) return;
    stopRecording();
    openingKeys = value.slice();
    candidate = value.slice();
    const primaryCount = value.filter((key) => !isModifier(key)).length;
    advanced = primaryCount > 1;
    const narrow = window.matchMedia?.('(max-width: 640px)').matches ?? window.innerWidth <= 640;
    const popoverAvailable = typeof HTMLElement.prototype.showPopover === 'function';
    pickerMode = narrow || !popoverAvailable ? 'step' : 'popover';
    pickerOpen = true;
    if (pickerMode === 'step') onMobileStep?.(true);
    await tick();
    const pickerPanel = panel;
    if (!pickerPanel) {
      closePicker(true);
      return;
    }

    if (pickerMode === 'popover') {
      try {
        pickerPanel.showPopover();
        positionPopover();
        assertPopoverReachable(pickerPanel);
        window.addEventListener('resize', positionPopover);
        window.addEventListener('scroll', positionPopover, true);
        document.addEventListener('pointerdown', noteOutsidePointer, true);
        outsidePointerDown = false;
        placementListening = true;
      } catch {
        if (pickerPanel.matches(':popover-open')) pickerPanel.hidePopover();
        removePlacementListeners();
        pickerMode = 'step';
        onMobileStep?.(true);
        await tick();
      }
    }
    panel?.querySelector<HTMLInputElement>('input[type="search"]')?.focus({ preventScroll: true });
  }

  function useCandidate() {
    if (!canUseCandidate || !pickerOpen) return;
    if (!sameKeys(candidate, openingKeys)) onchange(candidate.map((key) => normalizeKey(key)!));
    closePicker(true);
  }

  function reconcilePicker() {
    if (pickerOpen && !sameKeys(value, openingKeys)) closePicker(true);
  }

  $effect(() => {
    const enabled = active;
    const currentValue = value;
    if (!enabled) {
      stopRecording();
      if (pickerOpen) closePicker(true);
    } else if (pickerOpen && !sameKeys(currentValue, openingKeys)) {
      reconcilePicker();
    }
  });

  onDestroy(() => {
    clearRecordingTimer();
    clearTimeout(focusExitTimer);
    removePlacementListeners();
    if (panel?.matches(':popover-open')) panel.hidePopover();
  });
</script>

<section class="key-selector" aria-label={t('ui_keypicker_shortcut')}>
  <div
    class="shortcut-preview"
    aria-label={`${t('ui_keypicker_shortcut')}: ${summary || t('ui_keypicker_no_shortcut')}`}
  >
    <span>{t('ui_keypicker_shortcut')}</span>
    <output>{summary || t('ui_keypicker_no_shortcut')}</output>
  </div>

  <div class="selector-controls">
    <button
      bind:this={recordButton}
      type="button"
      class="record"
      aria-pressed={recorderState === 'listening'}
      onclick={startRecording}
      onkeydown={recorderKeydown}
      onblur={() => {
        if (recorderState === 'listening') stopRecording();
      }}
      >{recorderState === 'listening'
        ? t('ui_keypicker_recording')
        : t('ui_keypicker_record_shortcut')}</button
    >
    <button
      bind:this={chooseButton}
      type="button"
      aria-haspopup={pickerOpen && pickerMode === 'popover' ? 'dialog' : undefined}
      aria-expanded={pickerOpen}
      aria-controls={panelId}
      onclick={openPicker}>{t('ui_keypicker_choose_keys')}</button
    >
  </div>

  <p class="host-status" role="status">{t(hostStatusMessage(capabilities))}</p>
  <p class="capture-help">{t('ui_keypicker_capture_limits')}</p>
  <p class="recorder-status" aria-live="polite" aria-atomic="true">{recorderMessage}</p>

  {#if pickerOpen}
    <div
      bind:this={panel}
      id={panelId}
      class:picker-step={pickerMode === 'step'}
      class="picker"
      popover={pickerMode === 'popover' ? 'auto' : undefined}
      role={pickerMode === 'popover' ? 'dialog' : 'region'}
      aria-label={t('ui_keypicker_choose_keys')}
      ontoggle={popoverToggled}
      onfocusout={queueFocusExit}
      onkeydown={pickerKeydown}
    >
      <div class="picker-content">
        <KeyPickerPanel
          {candidate}
          onCandidateChange={(next) => (candidate = next)}
          {advanced}
          onAdvancedChange={(next) => (advanced = next)}
          {capabilities}
        />
      </div>
      <div class="picker-actions">
        <button type="button" onclick={() => closePicker(true)}>{t('ui_keypicker_cancel')}</button>
        <button type="button" class="use" disabled={!canUseCandidate} onclick={useCandidate}
          >{t('ui_keypicker_use_shortcut')}</button
        >
      </div>
    </div>
  {/if}
</section>

<style>
  .key-selector {
    position: relative;
    display: grid;
    gap: 9px;
    max-width: none;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    min-width: 0;
  }
  .shortcut-preview {
    display: grid;
    gap: 5px;
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-control);
    background: var(--surface-raised, #ffffff08);
  }
  .shortcut-preview > span {
    color: var(--text-muted);
    font-size: 0.76rem;
  }
  .shortcut-preview output {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 1rem;
    font-weight: 600;
  }
  .selector-controls {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .selector-controls button {
    min-height: 44px;
  }
  .selector-controls .record {
    font-weight: 650;
    background: var(--accent-surface, #ffffff14);
  }
  .host-status,
  .capture-help,
  .recorder-status {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.78rem;
    line-height: 1.4;
  }
  .recorder-status:empty {
    display: none;
  }
  .picker {
    position: fixed;
    inset: auto;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: min(420px, calc(100vw - 24px));
    height: max-content;
    max-height: min(480px, 70dvh);
    margin: 0;
    padding: 14px;
    overflow: hidden;
    overscroll-behavior: contain;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-panel, 12px);
    background: var(--surface);
    color: inherit;
    box-shadow: var(--shadow-dialog);
  }
  .picker::backdrop {
    background: transparent;
  }
  .picker-step {
    position: static;
    z-index: auto;
    display: block;
    width: auto;
    height: auto;
    max-height: none;
    margin: 0;
    padding: 0;
    overflow: visible;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
  }
  .picker-content {
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    flex: 1 1 auto;
  }
  .picker-step .picker-content {
    overflow: visible;
  }
  .picker-actions {
    flex: 0 0 auto;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin: 12px -14px -14px;
    padding: 10px 14px 14px;
    border-top: 1px solid var(--border-subtle);
    background: var(--surface);
  }
  .picker-actions button {
    min-height: 44px;
  }
  .picker-actions .use {
    font-weight: 650;
  }
  .picker-actions .use:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  @media (max-width: 640px) {
    .selector-controls {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    }
    .selector-controls button {
      min-width: 0;
      padding: 8px;
    }
  }
</style>
