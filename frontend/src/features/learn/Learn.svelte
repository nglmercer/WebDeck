<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import 'driver.js/dist/driver.css';
  import type { GuideRunner } from './runner';
  import { availableGuides, guideSteps, type GuideId, type GuideStep } from './guides';
  import Modal from '../../components/Modal.svelte';
  import Icon from '../../components/Icon.svelte';
  import FilterSelect from '../../components/FilterSelect.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { tutorialActive } from './tour-state';
  const t = useTranslations();
  let {
    canEdit,
    pairing = false,
    hasFolders = true,
    prepare,
    restore,
    focus,
  }: {
    canEdit: boolean;
    pairing?: boolean;
    hasFolders?: boolean;
    prepare: (step: GuideStep, signal: AbortSignal) => Promise<void>;
    restore: () => Promise<void>;
    focus: () => void;
  } = $props();
  let view = $state<'learn' | 'demo' | null>(null);
  let scene = $state('ui_scene_focus');
  let microphone = $state(true);
  let recording = $state(false);
  let feedback = $state('');
  let search = $state('');
  let category = $state('all');
  let error = $state('');
  let running = $state<GuideId | null>(null);
  let completed = $state<string[]>([]);
  let runner: GuideRunner | undefined;
  let disposed = false;
  let request = 0;
  let origin: HTMLElement | null = null;
  const available = $derived(availableGuides(canEdit, pairing));
  const visible = $derived(
    available.filter(
      (guide) =>
        (category === 'all' || category === guide.group) &&
        `${t(guide.title)} ${t(guide.description)}`
          .toLowerCase()
          .includes(search.trim().toLowerCase()),
    ),
  );
  const groups = $derived([...new Set(available.map((guide) => guide.group))]);
  const done = $derived(available.filter((guide) => completed.includes(guide.id)).length);
  try {
    const saved: unknown = JSON.parse(localStorage.getItem('webdeck.guides.v1') ?? '[]');
    if (Array.isArray(saved))
      completed = saved.filter((item): item is string => typeof item === 'string');
  } catch {
    /* Progress is optional when browser storage is unavailable. */
  }

  export function open(id?: GuideId, returnTo?: HTMLElement | null) {
    origin =
      returnTo ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    error = '';
    search = '';
    category = 'all';
    if (id) void startGuide(id);
    else view = 'learn';
  }
  function close() {
    view = null;
    void tick().then(() => {
      if (origin?.isConnected) origin.focus();
      else focus();
    });
  }
  function reset() {
    scene = 'ui_scene_focus';
    microphone = true;
    recording = false;
    feedback = '';
  }
  async function startGuide(id: GuideId) {
    const guide = available.find((item) => item.id === id);
    if (!guide) {
      view = 'learn';
      return;
    }
    const current = ++request;
    await runner?.stop();
    if (disposed || current !== request) return;
    if (id === 'practice') {
      reset();
      view = 'demo';
    } else view = null;
    error = '';
    running = id;
    await tick();
    const { GuideRunner } = await import('./runner');
    if (disposed || current !== request) return;
    runner ??= new GuideRunner({
      translate: t,
      prepare: async (step, signal) => {
        if (step.frame.surface === 'practice') view = 'demo';
        await tick();
        await prepare(step, signal);
      },
      restore: async () => {
        await restore();
        running = null;
      },
      focus: () => {
        if (view === 'demo')
          document.querySelector<HTMLElement>('[data-guide="practice-start"]')?.focus();
        else focus();
      },
      complete: (finished) => {
        completed = [...new Set([...completed, finished])];
        try {
          localStorage.setItem('webdeck.guides.v1', JSON.stringify(completed));
        } catch {
          /* Optional local progress. */
        }
      },
      error: (message) => {
        error = t(message);
        view = 'learn';
      },
    });
    await runner.start(guide, guideSteps(guide, canEdit, hasFolders), origin);
  }
  $effect(() => {
    if (running && !available.some((guide) => guide.id === running)) void runner?.stop();
  });
  onDestroy(() => {
    disposed = true;
    request++;
    void runner?.stop();
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
  <Modal label={t(view === 'demo' ? 'ui_practice' : 'ui_learn')} width="760px" {close}>
    <div class="heading" data-learning-dialog>
      <h2>{t(view === 'demo' ? 'ui_practice' : 'ui_learn')}</h2>
      <button type="button" onclick={close}>{t('ui_close')}</button>
    </div>
    {#if view === 'learn'}
      <p>{t('ui_learn_help')}</p>
      {#if error}<p role="alert">{error}</p>{/if}
      <div class="guide-filters">
        <label
          ><span class="sr-only">{t('ui_guide_search')}</span><input
            type="search"
            bind:value={search}
            placeholder={t('ui_guide_search')}
          /></label
        >
        <FilterSelect
          label={t('ui_guide_category')}
          hideLabel
          value={category}
          options={[
            { value: 'all', label: t('ui_guide_all') },
            ...groups.map((value) => ({ value, label: t(`ui_guide_${value}`) })),
          ]}
          onchange={(value) => (category = value)}
        />
      </div>
      <p class="guide-progress" role="status">
        {t('ui_guide_progress', { done, total: available.length })}
      </p>
      <div class="lessons">
        {#each visible as guide}<button
            type="button"
            class="lesson"
            data-guide-id={guide.id}
            onclick={() => void startGuide(guide.id)}
          >
            <span class="lesson-icon"><Icon name={guide.icon} /></span>
            <span class="lesson-copy"
              ><strong>{t(guide.title)}</strong><small>{t(guide.description)}</small><span
                class="lesson-meta"
                >{t('ui_guide_steps', {
                  count: guideSteps(guide, canEdit, hasFolders).length,
                })}{#if completed.includes(guide.id)}<span class="completed"
                    >{t('ui_guide_completed')}</span
                  >{/if}</span
              ></span
            ><span aria-hidden="true">→</span>
          </button>{/each}
        {#if !visible.length}<p role="status">{t('ui_guide_no_results')}</p>{/if}
      </div>
      <button
        type="button"
        class="lesson demo-entry"
        onclick={() => {
          reset();
          view = 'demo';
        }}
      >
        <span class="lesson-icon"><Icon name="play" /></span><span class="lesson-copy"
          ><strong>{t('ui_demo')}</strong><small>{t('ui_demo_help')}</small></span
        ><span aria-hidden="true">→</span>
      </button>
    {:else}
      <div class="demo-heading">
        <span class="demo-safe">{t('ui_demo_safe')}</span><button
          type="button"
          data-guide="practice-start"
          disabled={$tutorialActive}
          onclick={() => void startGuide('practice')}>{t('ui_guide_practice_title')}</button
        >
      </div>
      <p>{t('ui_demo_prompt')}</p>
      <div class="practice-grid" data-guide="practice-grid">
        {#each ['ui_scene_focus', 'ui_scene_break'] as item}<button
            type="button"
            data-guide={item === 'ui_scene_break' ? 'practice-scene' : undefined}
            aria-pressed={scene === item}
            onclick={() => choose(item)}
            ><Icon name="folder" size={28} /><strong>{t(item)}</strong><small
              >{scene === item ? t('ui_on') : t('ui_off')}</small
            ></button
          >{/each}
        <button
          type="button"
          data-guide="practice-microphone"
          aria-pressed={microphone}
          onclick={() => toggle('microphone')}
          ><Icon name="microphone" size={28} /><strong>{t('ui_mic')}</strong><small
            >{t(microphone ? 'ui_on' : 'ui_off')}</small
          ></button
        >
        <button
          type="button"
          data-guide="practice-recording"
          aria-pressed={recording}
          onclick={() => toggle('recording')}
          ><span class="record-dot" class:recording></span><strong>{t('ui_record')}</strong><small
            >{t(recording ? 'ui_on' : 'ui_off')}</small
          ></button
        >
      </div>
      <p class="practice-feedback" role="status">{feedback || t('ui_demo_safe')}</p>
      <div class="row">
        <button type="button" data-guide="practice-reset" onclick={reset}
          >{t('ui_reset_demo')}</button
        ><button type="button" onclick={() => (view = 'learn')}>{t('ui_back')}</button>
      </div>
    {/if}
  </Modal>
{/if}

<style>
  h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  .heading {
    margin-bottom: 0;
  }
  .guide-filters {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 190px;
    gap: 10px;
  }
  .guide-filters label {
    margin: 0;
  }
  .guide-filters input {
    min-height: 44px;
    border-radius: var(--radius-control);
    font-size: 0.85rem;
  }
  .guide-progress {
    margin: 0;
  }
  .lessons {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .lesson {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 16px;
    min-width: 0;
  }
  .lesson-copy {
    flex: 1;
    min-width: 0;
  }
  .lesson strong {
    font-size: 0.9rem;
    font-weight: 550;
  }
  .lesson small {
    display: block;
    color: var(--text-muted);
    font-size: 0.78rem;
    line-height: 1.5;
    margin-top: 6px;
  }
  .lesson-icon {
    color: var(--accent);
    flex-shrink: 0;
  }
  .lesson-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    margin-top: 10px;
    color: var(--text-muted);
    font-size: 0.7rem;
  }
  .completed {
    color: var(--success-text);
  }
  .demo-entry {
    margin-top: 6px;
  }
  .demo-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .demo-heading button {
    font-size: 0.8rem;
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
  @media (max-width: 600px) {
    .guide-filters,
    .lessons {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
