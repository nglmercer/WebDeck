<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import type { Driver } from 'driver.js';
  import 'driver.js/dist/driver.css';
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();
  let {
    canEdit,
    prepare,
    restore,
  }: { canEdit: boolean; prepare: () => void; restore: () => void } = $props();
  export function open() {
    view = 'learn';
  }
  let view = $state<'learn' | 'demo' | null>(null);
  let scene = $state('ui_scene_focus');
  let microphone = $state(true);
  let recording = $state(false);
  let feedback = $state('');
  let tour: Driver | undefined;
  let disposed = false;

  function close() {
    view = null;
    void tick().then(() => restore());
  }
  function reset() {
    scene = 'ui_scene_focus';
    microphone = true;
    recording = false;
    feedback = '';
  }
  async function startTour() {
    view = null;
    prepare();
    await tick();
    tour?.destroy();
    const steps = [
      {
        element: '[data-tour="deck"]',
        title: 'ui_tour_workspace',
        description: 'ui_tour_workspace_help',
      },
      ...(document.querySelector('[data-tour="folders"]')
        ? [
            {
              element: '[data-tour="folders"]',
              title: 'ui_tour_folders',
              description: 'ui_tour_folders_help',
            },
          ]
        : []),
      ...(canEdit
        ? [
            { element: undefined, title: 'ui_tour_edit', description: 'ui_tour_edit_help' },
            { element: undefined, title: 'ui_tour_connect', description: 'ui_tour_connect_help' },
          ]
        : []),
      { element: undefined, title: 'ui_tour_learn', description: 'ui_tour_learn_help' },
    ];
    const { driver } = await import('driver.js');
    if (disposed) return;
    tour = driver({
      animate: !matchMedia('(prefers-reduced-motion: reduce)').matches,
      showProgress: true,
      disableActiveInteraction: true,
      popoverClass: 'webdeck-tour',
      stageRadius: 14,
      nextBtnText: t('ui_next_step'),
      prevBtnText: t('ui_previous_step'),
      doneBtnText: t('ui_finish_tour'),
      progressText: t('ui_tour_progress'),
      onPopoverRender: (popover) => {
        popover.closeButton.setAttribute('aria-label', t('ui_close_tour'));
      },
      onDestroyStarted: (_element, _step, { driver: activeTour }) => {
        activeTour.destroy();
        queueMicrotask(() => restore());
      },
      steps: steps.map(({ element, title, description }) => ({
        ...(element ? { element } : {}),
        popover: { title: t(title), description: t(description) },
      })),
    });
    tour.drive();
  }
  onDestroy(() => {
    disposed = true;
    tour?.destroy();
  });
  function choose(next: string) {
    scene = next;
    feedback = t('ui_demo_scene', { scene: t(next) });
  }
  function toggle(control: 'microphone' | 'recording') {
    if (control === 'microphone') microphone = !microphone;
    else recording = !recording;
    feedback = t('ui_demo_toggle', {
      control: t(control === 'microphone' ? 'ui_mic' : 'ui_record'),
      state: t((control === 'microphone' ? microphone : recording) ? 'ui_on' : 'ui_off'),
    });
  }
</script>

{#if view}
  <Modal label={t(view === 'demo' ? 'ui_practice' : 'ui_learn')} width="640px" {close}>
    <div class="heading" data-learning-dialog>
      <h2>{t(view === 'demo' ? 'ui_practice' : 'ui_learn')}</h2>
      <button onclick={close}>{t('ui_close')}</button>
    </div>
    {#if view === 'learn'}
      <p>{t('ui_learn_help')}</p>
      <button class="lesson" onclick={startTour}
        ><span class="lesson-icon"><Icon name="grid" /></span><span
          ><strong>{t('ui_tour')}</strong><small>{t('ui_tour_help')}</small></span
        ><span aria-hidden="true">→</span></button
      >
      <button
        class="lesson"
        onclick={() => {
          reset();
          view = 'demo';
        }}
        ><span class="lesson-icon"><Icon name="play" /></span><span
          ><strong>{t('ui_demo')}</strong><small>{t('ui_demo_help')}</small></span
        ><span aria-hidden="true">→</span></button
      >
    {:else}
      <span class="demo-safe">{t('ui_demo_safe')}</span>
      <p>{t('ui_demo_prompt')}</p>
      <div class="practice-grid">
        {#each ['ui_scene_focus', 'ui_scene_break'] as item}
          <button aria-pressed={scene === item} onclick={() => choose(item)}
            ><Icon name="folder" size={28} /><strong>{t(item)}</strong><small
              >{scene === item ? t('ui_on') : t('ui_off')}</small
            ></button
          >
        {/each}
        <button aria-pressed={microphone} onclick={() => toggle('microphone')}
          ><Icon name="microphone" size={28} /><strong>{t('ui_mic')}</strong><small
            >{t(microphone ? 'ui_on' : 'ui_off')}</small
          ></button
        >
        <button aria-pressed={recording} onclick={() => toggle('recording')}
          ><span class="record-dot" class:recording></span><strong>{t('ui_record')}</strong><small
            >{t(recording ? 'ui_on' : 'ui_off')}</small
          ></button
        >
      </div>
      <p class="practice-feedback" role="status">{feedback || t('ui_demo_safe')}</p>
      <div class="row">
        <button onclick={reset}>{t('ui_reset_demo')}</button><button
          onclick={() => (view = 'learn')}>{t('ui_back')}</button
        >
      </div>
    {/if}
  </Modal>
{/if}

<style>
  h2 {
    margin: 0;
    font-size: 1.3rem;
  }
  .heading {
    margin-bottom: 0;
  }
  .lesson {
    display: flex;
    align-items: center;
    gap: 16px;
    text-align: left;
    padding: 20px;
  }
  .lesson > span:nth-child(2) {
    flex: 1;
  }
  .lesson small {
    display: block;
    color: var(--text-muted);
    line-height: 1.6;
    margin-top: 6px;
  }
  .lesson-icon {
    color: var(--accent);
  }
  .demo-safe {
    color: var(--accent);
    font-size: 0.8rem;
  }
  .practice-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .practice-grid button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 24px 8px;
  }
  .practice-grid button[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .practice-grid small {
    color: var(--text-muted);
  }
  .record-dot {
    width: 28px;
    height: 28px;
    border: 2px solid var(--text-muted);
    border-radius: 50%;
  }
  .record-dot.recording {
    background: #ff7586;
    border-color: #ff7586;
  }
  .practice-feedback {
    min-height: 2.5em;
  }
</style>
