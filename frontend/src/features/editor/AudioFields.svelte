<script lang="ts">
  import { id } from '../../lib/id';
  import { onDestroy, onMount, untrack } from 'svelte';
  import Icon from '../../components/Icon.svelte';
  import FilePicker from '../../components/FilePicker.svelte';
  import RangeField from '../../components/RangeField.svelte';
  import { useTranslations } from '../../lib/i18n';
  import { request, asset, upload } from '../../lib/api/client';
  import { credentials } from '../../lib/api/http';
  import type { AudioDevices, Command } from '../../lib/contracts';
  import {
    decodeAudio,
    fileSize,
    formatTime,
    parseTime,
    sourceKey,
    waveform,
    type MediaMetadata,
  } from './audio';
  const t = useTranslations();
  const clipId = `wave-${id()}`;
  let {
    value,
    previewBuffer,
    onchange,
    active = true,
    metadata,
    onmetadata = () => {},
    onvalidity = () => {},
  }: {
    value: Extract<Command, { type: 'play_sound' }>;
    previewBuffer?: AudioBuffer | undefined;
    onchange: (value: Extract<Command, { type: 'play_sound' }>) => void;
    active?: boolean;
    metadata?: MediaMetadata | undefined;
    onmetadata?: ((metadata: MediaMetadata) => void) | undefined;
    onvalidity?: (valid: boolean) => void;
  } = $props();
  let outputs = $state<string[]>([]);
  let deviceError = $state(false);
  let loadError = $state('');
  let loading = $state(false);
  let busy = $state(false);
  let decoded = $state.raw<AudioBuffer | null>(null);
  let peaks = $state<number[]>([]);
  let size = $state(0);
  let playing = $state(false);
  let position = $state(0);
  let hoverTime = $state<number | null>(null);
  let focusedEdge = $state<'start' | 'end' | 'seek' | null>(null);
  let dragging = false;
  let overTrack = false;
  let fieldError = $state('');
  let startInput = $state<HTMLInputElement>();
  let endInput = $state<HTMLInputElement>();
  let destroyed = false;
  let player: AudioBufferSourceNode | undefined;
  let context: AudioContext | undefined;
  let gain: GainNode | undefined;
  let frame = 0;
  let track = $state<HTMLDivElement>();
  const key = $derived(sourceKey(value.source));
  const duration = $derived(decoded ? Math.floor(decoded.duration * 1000) : 0);
  const start = $derived(value.start_ms ?? 0);
  const end = $derived(value.end_ms ?? duration);
  const ready = $derived(!!decoded && !loading && start >= 0 && start < end && end <= duration);
  const filename = $derived(
    metadata?.source === key ? metadata.name : (key.split(/[\\/]/).pop() ?? key),
  );
  const fileMetadata = $derived(
    [
      filename.split('.').pop()?.toUpperCase(),
      duration
        ? `${(duration / 1000).toLocaleString(undefined, { maximumFractionDigits: 3 })} s`
        : '',
      size ? fileSize(size) : '',
    ]
      .filter(Boolean)
      .join(' · '),
  );
  const left = $derived(duration ? (100 * start) / duration : 0);
  const right = $derived(duration ? (100 * end) / duration : 100);
  const tooltipTime = $derived(
    hoverTime ??
      (focusedEdge === 'start'
        ? start
        : focusedEdge === 'end'
          ? end
          : focusedEdge === 'seek' || playing
            ? position
            : null),
  );
  onMount(() => {
    void request<AudioDevices>('audio/devices', 'AudioDevices')
      .then((result) => {
        if (!destroyed) outputs = [...new Set(result.outputs)];
      })
      .catch(() => {
        if (!destroyed) deviceError = true;
      });
  });
  $effect(() => {
    const currentKey = key;
    const local = previewBuffer;
    const source = untrack(() => value.source);
    stop();
    decoded = null;
    peaks = [];
    size = 0;
    position = 0;
    hoverTime = null;
    loadError = '';
    fieldError = '';
    if (local) {
      decoded = local;
      peaks = waveform(local);
      size = local.length * 2 + 44;
      position = untrack(() => start);
      loading = false;
      return;
    }
    if (!currentKey) {
      loading = false;
      return;
    }
    let stale = false;
    const controller = new AbortController();
    loading = true;
    void (async () => {
      let url: string | undefined;
      try {
        let bytes: ArrayBuffer;
        if (source.type === 'asset') {
          url = await asset(source.id);
          bytes = await (await fetch(url, { signal: controller.signal })).arrayBuffer();
        } else {
          const response = await fetch('/api/v2/audio/preview', {
            method: 'POST',
            headers: { ...credentials(), 'content-type': 'application/json' },
            body: JSON.stringify(source),
            signal: controller.signal,
          });
          if (!response.ok) throw new Error('ui_audio_unreadable');
          bytes = await response.arrayBuffer();
        }
        const byteLength = bytes.byteLength;
        const buffer = await decodeAudio(bytes);
        if (!stale) {
          decoded = buffer;
          peaks = waveform(buffer);
          size = byteLength;
          position = start;
        }
      } catch {
        if (!stale) loadError = 'ui_audio_unreadable';
      } finally {
        if (url) URL.revokeObjectURL(url);
        if (!stale) loading = false;
      }
    })();
    return () => {
      stale = true;
      controller.abort();
      stop();
    };
  });
  $effect(() => {
    onvalidity(!busy && !loading && !!key && !loadError && !fieldError && ready);
  });
  $effect(() => {
    if (!active) stop();
  });
  $effect(() => {
    const volume = value.volume;
    if (gain) gain.gain.value = volume / 100;
  });
  function stop() {
    cancelAnimationFrame(frame);
    if (player) {
      player.onended = null;
      player.stop();
      player.disconnect();
      player = undefined;
    }
    playing = false;
  }
  async function play() {
    if (playing) {
      stop();
      return;
    }
    if (!decoded || !ready) return;
    try {
      context ??= new AudioContext();
      await context.resume();
      if (destroyed || !active || !decoded || !ready) return;
      gain ??= context.createGain();
      gain.disconnect();
      gain.connect(context.destination);
      gain.gain.value = value.volume / 100;
      player = context.createBufferSource();
      player.buffer = decoded;
      player.connect(gain);
      const offset = position >= start && position < end ? position : start;
      const began = context.currentTime;
      player.onended = () => {
        stop();
        position = start;
      };
      player.start(0, offset / 1000, (end - offset) / 1000);
      playing = true;
      const advance = () => {
        if (!playing || !context) return;
        position = Math.min(end, offset + (context.currentTime - began) * 1000);
        frame = requestAnimationFrame(advance);
      };
      advance();
    } catch {
      stop();
      loadError = 'ui_audio_preview_failed';
    }
  }
  async function choose(file: File) {
    busy = true;
    try {
      try {
        await decodeAudio(await file.arrayBuffer());
      } catch {
        throw new Error(t('ui_audio_unreadable'));
      }
      const source = await upload(file);
      if (destroyed) return;
      onmetadata({ source: sourceKey(source), name: file.name, size: file.size });
      const next = { ...value, source };
      delete next.start_ms;
      delete next.end_ms;
      onchange(next);
    } finally {
      if (!destroyed) busy = false;
    }
  }
  function trim(which: 'start' | 'end', ms: number) {
    stop();
    fieldError = '';
    const next = { ...value };
    if (which === 'start') next.start_ms = Math.max(0, Math.min(Math.round(ms), end - 1));
    else next.end_ms = Math.min(duration, Math.max(Math.round(ms), start + 1));
    position = next.start_ms ?? 0;
    startInput?.setCustomValidity('');
    endInput?.setCustomValidity('');
    if (startInput) startInput.value = formatTime(next.start_ms ?? 0);
    if (endInput) endInput.value = formatTime(next.end_ms ?? duration);
    onchange(next);
  }
  function editTime(event: Event, which: 'start' | 'end') {
    const input = event.currentTarget as HTMLInputElement;
    const ms = parseTime(input.value);
    const valid =
      ms !== null && (which === 'start' ? ms >= 0 && ms < end : ms > start && ms <= duration);
    fieldError = valid ? '' : 'ui_audio_time_invalid';
    input.setCustomValidity(valid ? '' : t(fieldError));
    if (valid) trim(which, ms!);
  }
  function reset() {
    stop();
    fieldError = '';
    position = 0;
    const next = { ...value };
    delete next.start_ms;
    delete next.end_ms;
    startInput?.setCustomValidity('');
    endInput?.setCustomValidity('');
    if (startInput) startInput.value = formatTime(0);
    if (endInput) endInput.value = formatTime(duration);
    onchange(next);
  }
  function drag(event: PointerEvent, which: 'start' | 'end' | 'seek') {
    if (!ready || event.button !== 0) return;
    event.preventDefault();
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    dragging = true;
    stop();
    const update = (move: PointerEvent) => {
      if (!track) return;
      const bounds = track.getBoundingClientRect();
      const ms = Math.round(
        Math.max(0, Math.min(1, (move.clientX - bounds.left) / bounds.width)) * duration,
      );
      if (which === 'seek') position = Math.max(start, Math.min(end, ms));
      else trim(which, ms);
      hoverTime =
        which === 'seek'
          ? position
          : which === 'start'
            ? Math.min(ms, end - 1)
            : Math.max(ms, start + 1);
    };
    update(event);
    target.onpointermove = update;
    const release = () => {
      dragging = false;
      if (!overTrack || event.pointerType === 'touch') hoverTime = null;
      target.onpointermove = null;
      target.onpointerup = null;
      target.onlostpointercapture = null;
    };
    target.onpointerup = release;
    target.onlostpointercapture = release;
  }
  function inspect(event: PointerEvent) {
    if (!ready || !track || dragging || event.pointerType === 'touch') return;
    const edge = (event.target as Element).closest<HTMLElement>('[data-edge]')?.dataset.edge;
    const bounds = track.getBoundingClientRect();
    hoverTime =
      edge === 'start'
        ? start
        : edge === 'end'
          ? end
          : Math.round(
              Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width)) * duration,
            );
  }
  onDestroy(() => {
    destroyed = true;
    stop();
    void context?.close();
  });
</script>

<div class="audio-fields">
  <FilePicker
    label={t('ui_audio_file')}
    accept=".mp3,.wav,.ogg,.flac"
    {filename}
    metadata={fileMetadata}
    icon="music"
    busy={busy || loading}
    onselect={choose}
  />
  {#if loadError}<p class="error" role="alert">{t(loadError)}</p>{/if}
  {#if key}
    <div class="fragment">
      <div class="section-heading">
        <span>{t('ui_audio_fragment')}</span><button
          type="button"
          class="text-button"
          disabled={!decoded}
          onclick={reset}>{t('ui_audio_reset')}</button
        >
      </div>
      <div class="waveform" class:loading aria-busy={loading}>
        <div
          class="track"
          role="group"
          aria-label={t('ui_audio_fragment')}
          bind:this={track}
          onpointerenter={(event) => {
            overTrack = true;
            inspect(event);
          }}
          onpointermove={inspect}
          onpointerleave={() => {
            overTrack = false;
            if (!dragging) hoverTime = null;
          }}
        >
          <svg viewBox="0 0 600 84" preserveAspectRatio="none" aria-hidden="true"
            ><defs
              ><clipPath id={clipId}
                ><rect x={left * 6} y="0" width={(right - left) * 6} height="84" /></clipPath
              ></defs
            >
            <g stroke="var(--border)" stroke-width="2.5" stroke-linecap="round"
              >{#each peaks as peak, i}<path
                  d={`M${i * 5 + 2.5},${42 - peak * 34}v${peak * 68}`}
                />{/each}</g
            >
            <g
              stroke="var(--accent)"
              stroke-width="2.5"
              stroke-linecap="round"
              clip-path={`url(#${clipId})`}
              >{#each peaks as peak, i}<path
                  d={`M${i * 5 + 2.5},${42 - peak * 34}v${peak * 68}`}
                />{/each}</g
            >
          </svg>
          <div
            class="seek"
            role="slider"
            tabindex={ready ? 0 : -1}
            aria-label={t('ui_audio_position')}
            aria-valuemin={start}
            aria-valuemax={end}
            aria-valuenow={Math.round(position)}
            aria-valuetext={formatTime(position)}
            aria-disabled={!ready}
            onfocus={() => {
              hoverTime = null;
              focusedEdge = 'seek';
            }}
            onblur={() => (focusedEdge = null)}
            onpointerdown={(event) => drag(event, 'seek')}
            onkeydown={(event) => {
              if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
                event.preventDefault();
                stop();
                position = Math.max(
                  start,
                  Math.min(end, position + (event.key === 'ArrowRight' ? 100 : -100)),
                );
              }
            }}
          ></div>
          {#if ready}<span class="playhead" style:left={`${(100 * position) / duration}%`}
            ></span>{/if}
          {#each ['start', 'end'] as edge}<div
              class="trim-handle"
              data-edge={edge}
              role="slider"
              tabindex={ready ? 0 : -1}
              aria-label={t(edge === 'start' ? 'ui_audio_start' : 'ui_audio_end')}
              aria-disabled={!ready}
              aria-valuemin={edge === 'start' ? 0 : start + 1}
              aria-valuemax={edge === 'start' ? end - 1 : duration}
              aria-valuenow={edge === 'start' ? start : end}
              aria-valuetext={formatTime(edge === 'start' ? start : end)}
              onfocus={() => {
                hoverTime = null;
                focusedEdge = edge as 'start' | 'end';
              }}
              onblur={() => (focusedEdge = null)}
              style:left={`${edge === 'start' ? left : right}%`}
              onpointerdown={(event) => drag(event, edge as 'start' | 'end')}
              onkeydown={(event) => {
                if (!ready) return;
                const current = edge === 'start' ? start : end;
                const next =
                  event.key === 'ArrowRight'
                    ? current + (event.shiftKey ? 1000 : 100)
                    : event.key === 'ArrowLeft'
                      ? current - (event.shiftKey ? 1000 : 100)
                      : event.key === 'Home'
                        ? 0
                        : event.key === 'End'
                          ? duration
                          : null;
                if (next !== null) {
                  event.preventDefault();
                  trim(edge as 'start' | 'end', next);
                }
              }}
            >
              <span></span>
            </div>{/each}
          {#if ready && tooltipTime !== null}
            {@const percent = (100 * tooltipTime) / duration}
            <span class="cursor-guide" style:left={`${percent}%`} aria-hidden="true"></span>
            <output
              class="time-tooltip"
              aria-hidden="true"
              style:left={`clamp(46px, ${percent}%, calc(100% - 46px))`}
              >{formatTime(tooltipTime)}</output
            >
          {/if}
        </div>
        <div class="ruler" aria-hidden="true"></div>
      </div>
      <div class="transport">
        <button
          type="button"
          class="play"
          disabled={!ready}
          aria-label={t(playing ? 'ui_audio_stop_preview' : 'ui_audio_play_preview')}
          onclick={play}><Icon name={playing ? 'pause' : 'play'} size={24} /></button
        >
      </div>
      <div class="time-fields">
        <label
          >{t('ui_audio_start')}<input
            bind:this={startInput}
            aria-label={t('ui_audio_start')}
            value={formatTime(start)}
            disabled={!decoded}
            required
            pattern={'[0-9]+:[0-5][0-9]([.,][0-9]{1,3})?'}
            onchange={(event) => editTime(event, 'start')}
          /></label
        >
        <label
          >{t('ui_audio_end')}<input
            bind:this={endInput}
            aria-label={t('ui_audio_end')}
            value={formatTime(end)}
            disabled={!decoded}
            required
            pattern={'[0-9]+:[0-5][0-9]([.,][0-9]{1,3})?'}
            onchange={(event) => editTime(event, 'end')}
          /></label
        >
        <label
          >{t('ui_audio_duration')}<output
            >{(Math.max(0, end - start) / 1000).toLocaleString(undefined, {
              minimumFractionDigits: 3,
              maximumFractionDigits: 3,
            })}
            {t('ui_audio_seconds')}</output
          ></label
        >
      </div>
      {#if fieldError || (decoded && !ready)}<p class="error" role="alert">
          {t(fieldError || 'ui_audio_time_invalid')}
        </p>{/if}
    </div>
  {/if}
  <RangeField
    label={t('ui_volume_2')}
    value={value.volume}
    unit="%"
    onchange={(volume) => onchange({ ...value, volume })}
  />
  <label class="output-device"
    >{t('ui_audio_output')}<select
      value={value.output_device}
      onchange={(event) => onchange({ ...value, output_device: event.currentTarget.value })}
    >
      <option value="">{t('ui_audio_default_output')}</option>
      {#if value.output_device && !outputs.includes(value.output_device)}<option
          value={value.output_device}>{value.output_device}</option
        >{/if}
      {#each outputs as output}<option value={output}>{output}</option>{/each}
    </select></label
  >
  {#if deviceError}<p class="error" role="status">{t('ui_audio_devices_unavailable')}</p>{/if}
</div>

<style>
  .audio-fields {
    display: grid;
    gap: 14px;
    min-width: 0;
  }
  .section-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
    font-size: 0.85rem;
    font-weight: 550;
  }
  .text-button {
    padding: 4px 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 0.75rem;
  }
  .waveform {
    padding: 32px 14px 10px;
    background: var(--canvas);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
  }
  .track {
    height: 72px;
    position: relative;
    margin: 0 4px;
  }
  svg {
    width: 100%;
    height: 72px;
    display: block;
  }
  .seek {
    position: absolute;
    inset: 0;
    cursor: crosshair;
    touch-action: none;
  }
  .playhead {
    position: absolute;
    top: 4px;
    bottom: 4px;
    width: 1px;
    background: var(--text);
    pointer-events: none;
  }
  .trim-handle {
    position: absolute;
    top: 4px;
    bottom: 4px;
    transform: translateX(-50%);
    width: 24px;
    display: flex;
    justify-content: center;
    cursor: ew-resize;
    touch-action: none;
  }
  .trim-handle > span {
    width: 7px;
    background: var(--accent);
    border-radius: 5px;
    box-shadow: 0 0 0 1px var(--surface);
  }
  .trim-handle > span::after {
    content: '';
    display: block;
    width: 2px;
    height: 10px;
    margin: 8px auto;
    border-radius: 2px;
    background: var(--canvas);
  }
  .trim-handle[aria-disabled='true'] {
    display: none;
  }
  .ruler {
    height: 6px;
    border-top: 1px solid var(--border);
    background: repeating-linear-gradient(to right, var(--border) 0 1px, transparent 1px 10%);
    margin: 0 4px;
  }
  .cursor-guide {
    position: absolute;
    inset-block: 0;
    width: 1px;
    background: var(--text-muted);
    opacity: 0.6;
    pointer-events: none;
  }
  .time-tooltip {
    position: absolute;
    top: -26px;
    transform: translateX(-50%);
    padding: 3px 7px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    pointer-events: none;
  }
  .transport {
    display: flex;
    justify-content: center;
    align-items: center;
    margin: 12px 0;
  }
  .play {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    min-width: 48px;
    padding: 10px;
    border-radius: 50%;
    border-color: var(--accent);
    background: var(--accent-surface);
  }
  .time-fields {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  label {
    margin: 0;
  }
  .time-fields input,
  .time-fields output {
    display: block;
    width: 100%;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    padding: 8px;
    border-radius: 6px;
  }
  .time-fields output {
    background: var(--surface-raised);
    border: 1px solid var(--border-subtle);
  }
  .output-device {
    font-size: 0.85rem;
    color: var(--text);
    font-weight: 550;
  }
  select {
    font-weight: 400;
  }
  .error {
    margin: 0;
    color: var(--error-text);
    font-size: 0.8rem;
  }
  .loading {
    opacity: 0.5;
  }
  @media (max-width: 420px) {
    .time-fields {
      gap: 8px;
    }
    .time-fields input,
    .time-fields output {
      padding: 8px 4px;
      font-size: 0.72rem;
    }
  }
</style>
