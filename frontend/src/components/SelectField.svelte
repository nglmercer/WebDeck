<script lang="ts">
  import { tick, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { id } from '../lib/id';
  import { useTranslations } from '../lib/i18n';
  const t = useTranslations();
  type Option = { value: string; label: string; description?: string; icon?: string };
  let {
    label,
    value,
    options,
    onchange,
    searchable = false,
    allowCustom = false,
    placeholder = '',
    disabled = false,
    loading = false,
    onrefresh,
    required = false,
    maxLength = 4096,
  }: {
    label: string;
    value: string;
    options: readonly Option[];
    onchange: (value: string) => void;
    searchable?: boolean;
    allowCustom?: boolean;
    placeholder?: string;
    disabled?: boolean;
    loading?: boolean;
    onrefresh?: (() => void) | undefined;
    required?: boolean;
    maxLength?: number;
  } = $props();
  const uid = `select-${id()}`;
  let invalid = $state(false);
  $effect(() => {
    if (value) invalid = false;
  });
  let open = $state(false),
    query = $state(''),
    active = $state(0);
  let root: HTMLDivElement, trigger: HTMLButtonElement;
  let popup = $state<HTMLDivElement>(),
    search = $state<HTMLInputElement>();
  const selected = $derived(options.find((option) => option.value === value));
  const filtered = $derived(
    options.filter((option) =>
      `${option.label} ${option.value} ${option.description ?? ''}`
        .toLowerCase()
        .includes(query.toLowerCase()),
    ),
  );
  const custom = $derived(
    allowCustom && query.trim() && !options.some((option) => option.value === query.trim()),
  );
  const choices = $derived(
    custom
      ? [
          ...filtered.slice(0, 99),
          {
            value: query.trim(),
            label: t('forms.select.useCustom', { value: query.trim() }),
            icon: 'edit',
          },
        ]
      : filtered.slice(0, 100),
  );
  $effect(() => {
    choices;
    if (open) void tick().then(position);
  });
  function position() {
    if (!open || !popup || !trigger) return;
    const b = trigger.getBoundingClientRect();
    const width = Math.min(Math.max(b.width, 240), innerWidth - 24);
    const below = innerHeight - b.bottom - 12,
      above = b.top - 12;
    const height = Math.min(320, Math.max(below, above));
    popup.style.width = `${width}px`;
    popup.style.maxHeight = `${height}px`;
    popup.style.left = `${Math.max(12, Math.min(b.left, innerWidth - width - 12))}px`;
    popup.style.top = `${below >= Math.min(height, popup.scrollHeight) ? b.bottom + 6 : Math.max(12, b.top - Math.min(height, popup.scrollHeight) - 6)}px`;
  }
  function close(focus = false) {
    open = false;
    popup?.hidePopover?.();
    window.removeEventListener('resize', position);
    window.removeEventListener('scroll', position, true);
    if (focus) trigger?.focus();
  }
  async function show() {
    if (disabled) return;
    query = allowCustom && value && !options.some((option) => option.value === value) ? value : '';
    active = Math.max(
      0,
      choices.findIndex((option) => option.value === value),
    );
    open = true;
    await tick();
    popup?.showPopover?.();
    position();
    window.addEventListener('resize', position);
    window.addEventListener('scroll', position, true);
    if (searchable) search?.focus();
    else popup?.querySelector<HTMLElement>('[role=listbox]')?.focus();
  }
  function choose(index: number) {
    const option = choices[index];
    if (option) {
      onchange(option.value);
      close(true);
    }
  }
  function keyboard(event: KeyboardEvent) {
    if (event.target === search && query && ['Home', 'End'].includes(event.key)) return;
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      event.stopPropagation();
      close(true);
    } else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      if (!open) {
        void show();
        return;
      }
      active =
        event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? choices.length - 1
            : Math.max(
                0,
                Math.min(choices.length - 1, active + (event.key === 'ArrowUp' ? -1 : 1)),
              );
      document.getElementById(`${uid}-option-${active}`)?.scrollIntoView({ block: 'nearest' });
    } else if (event.key === 'Enter' && open) {
      event.preventDefault();
      choose(active);
    } else if (event.key === 'Tab') close(true);
  }
  function keyboardCapture(node: HTMLDivElement) {
    node.addEventListener('keydown', keyboard, true);
    return {
      destroy() {
        node.removeEventListener('keydown', keyboard, true);
      },
    };
  }
  onDestroy(() => close());
</script>

<svelte:window
  onpointerdown={(event) => {
    if (open && event.target instanceof Node && !root?.contains(event.target)) close();
  }}
/>
<div
  class="select-field"
  bind:this={root}
  use:keyboardCapture
  role="group"
  aria-labelledby={uid + '-label'}
>
  <span id={uid + '-label'} class="field-label">{label}</span>
  <div class="trigger-row">
    <button
      bind:this={trigger}
      type="button"
      class="trigger"
      role="combobox"
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-controls={open ? uid + '-list' : undefined}
      aria-labelledby={uid + '-label'}
      aria-describedby={uid + '-value' + (invalid ? ' ' + uid + '-error' : '')}
      aria-invalid={invalid}
      aria-required={required}
      {disabled}
      onclick={() => (open ? close() : show())}
    >
      {#if selected?.icon}<span class="app-icon"><Icon name={selected.icon} size={20} /></span
        >{/if}<span id={uid + '-value'} class:placeholder={!value}
        >{selected?.label || value || placeholder || t('forms.select.choose')}</span
      ><Icon name="chevron-down" size={16} />
    </button>{#if onrefresh}<button
        type="button"
        class="refresh"
        aria-label={t('forms.select.refresh')}
        disabled={loading}
        onclick={onrefresh}><Icon name="refresh" size={18} /></button
      >{/if}
  </div>
  {#if required}<input
      class="validity-proxy"
      aria-hidden="true"
      tabindex="-1"
      required
      maxlength={maxLength}
      {value}
      oninvalid={(event) => {
        event.preventDefault();
        invalid = true;
        trigger.focus();
      }}
    />{/if}
  {#if invalid}<p id={uid + '-error'} class="select-error" role="alert">
      {t('forms.select.required')}
    </p>{/if}
  {#if open}<div
      bind:this={popup}
      class="select-popup"
      popover="manual"
      tabindex="-1"
      ontoggle={(event) => {
        if (event.newState === 'closed' && open) close();
      }}
    >
      {#if searchable}<div class="search">
          <Icon name="search" size={18} /><input
            bind:this={search}
            type="search"
            maxlength={maxLength}
            aria-label={t('forms.select.search')}
            bind:value={query}
            oninput={() => (active = 0)}
            aria-controls={uid + '-list'}
            aria-activedescendant={choices[active] ? `${uid}-option-${active}` : undefined}
          />
        </div>{/if}
      <div
        tabindex="-1"
        role="listbox"
        id={uid + '-list'}
        aria-label={label}
        aria-activedescendant={choices[active] ? `${uid}-option-${active}` : undefined}
      >
        {#each choices.slice(0, 100) as option, index}<button
            type="button"
            role="option"
            id={`${uid}-option-${index}`}
            aria-selected={value === option.value}
            class:active={active === index}
            onpointermove={() => (active = index)}
            onclick={() => choose(index)}
          >
            <Icon name={option.icon ?? 'app-window'} size={21} /><span
              ><strong>{option.label}</strong>{#if option.description}<small
                  >{option.description}</small
                >{/if}</span
            >{#if value === option.value}<Icon name="check" size={17} />{/if}
          </button>{/each}
        {#if !choices.length}<p role="status">
            {t(loading ? 'forms.select.loading' : 'forms.select.empty')}
          </p>{/if}
      </div>
    </div>{/if}
</div>

<style>
  .select-error {
    color: var(--error-text);
    font-size: 0.75rem;
    margin: 6px 0;
  }
  .select-field {
    min-width: 0;
    position: relative;
  }
  .field-label {
    display: block;
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-bottom: 6px;
  }
  .trigger-row {
    display: flex;
    gap: 6px;
  }
  .trigger {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    min-height: 44px;
    background: var(--canvas);
    text-align: left;
    padding: 8px 10px;
    font-size: 0.85rem;
  }
  .trigger > span:last-of-type {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .placeholder {
    color: var(--text-muted);
  }
  .app-icon {
    display: flex;
  }
  .refresh {
    flex: 0 0 44px;
    padding: 8px;
    display: grid;
    place-items: center;
  }
  .select-popup {
    position: fixed;
    inset: auto;
    margin: 0;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow-dialog);
    background: var(--surface);
    color: var(--text);
    overflow: auto;
    z-index: 100;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px 10px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 4px;
  }
  .search input {
    border: 0;
    background: var(--canvas);
    min-width: 0;
    width: 100%;
    min-height: 40px;
  }
  .select-popup button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    min-height: 44px;
    text-align: left;
    border: 0;
    background: transparent;
    border-radius: 6px;
  }
  .select-popup button.active {
    background: var(--surface-hover);
  }
  .select-popup button[aria-selected='true'] {
    background: var(--accent-surface);
  }
  .select-popup button > span {
    flex: 1;
    min-width: 0;
  }
  .select-popup strong {
    font-size: 0.85rem;
    display: block;
    overflow-wrap: anywhere;
    font-weight: 500;
  }
  .select-popup small {
    display: block;
    font-size: 0.72rem;
    color: var(--text-muted);
    overflow-wrap: anywhere;
    margin-top: 2px;
  }
  .select-popup p {
    padding: 8px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .validity-proxy {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
    padding: 0;
    border: 0;
  }
</style>
