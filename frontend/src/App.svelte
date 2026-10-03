<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    Button,
    ButtonAction,
    Command,
    DeckBoot,
    Device,
    DeviceApproval,
    Capability,
    TranslationsResponse,
    UsageResponse,
    CatalogResponse,
    PluginManifest,
    SpotifyConnect,
  } from './contracts';
  import {
    ApiError,
    approve,
    asset,
    boot,
    commandSchema,
    config,
    connect,
    contract,
    defaultValue,
    devices as getDevicesRequest,
    execute,
    id,
    request,
    revoke,
    save,
    setToken,
    upload,
    type Schema,
  } from './api';
  import Fields from './Fields.svelte';
  import { metricReading, appearanceNumber, gridCells, preserveCells } from './deck';
  import { Editor, clone } from './editor.svelte';
  let deck = $state<DeckBoot | null>(null),
    editor = $state<Editor | null>(null),
    active = $state('home');
  let error = $state(''),
    notice = $state(''),
    loading = $state(true),
    pairing = $state(false),
    credential = $state('');
  let panel = $state<'deck' | 'settings'>('deck'),
    editing = $state(false),
    button = $state<Button | null>(null),
    buttonFolder = $state(''),
    buttonOrigin = $state(''),
    transport = $state<'http' | 'socket'>('http');
  let running = $state<Record<string, boolean>>({}),
    devices = $state<Device[]>([]),
    grant = $state<DeviceApproval | null>(null),
    deviceName = $state(''),
    usage = $state<UsageResponse | null>(null);
  let translation = $state<Record<string, string>>({}),
    languages = $state<string[]>([]),
    catalog = $state<CatalogResponse | null>(null),
    themeUrls = $state<string[]>([]),
    backgroundUrls = $state<string[]>([]),
    assetUrls = $state<Record<string, string>>({});
  let dialog = $state<HTMLDialogElement>();
  $effect(() => {
    if (dialog && button && !dialog.open) dialog.showModal();
  });
  const layout = $derived(editor?.draft.layout ?? deck?.layout);
  const current = $derived(layout?.folders.find((f) => f.id === active) ?? layout?.folders[0]);
  const t = (key: string, fallback: string) => translation[key] ?? fallback;
  const defaultCommand = () => defaultValue(commandSchema) as Command;
  let grantCapabilities = $state<Capability[]>(['read', 'input']),
    grantTtl = $state(86400);
  const commandFormSchema = $derived({
    ...commandSchema,
    oneOf: (commandSchema.oneOf ?? []).map((o) =>
      o.properties?.type?.const === 'plugin'
        ? { ...o, properties: { type: o.properties.type } }
        : o,
    ),
  });
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
  let help = $state(false);
  const cells = $derived(
    gridCells(current?.buttons ?? [], layout?.columns ?? 4, layout?.rows ?? 3),
  );
  function freezeCells() {
    for (const f of editor?.draft.layout.folders ?? [])
      preserveCells(f.buttons, layout?.columns ?? 4, layout?.rows ?? 3);
  }
  function deleteCell(b: Button) {
    freezeCells();
    if (current) current.buttons = current.buttons.filter((v) => v.id !== b.id);
    change();
  }
  let controls = $state(false),
    now = $state(Date.now());
  let hold: ReturnType<typeof setTimeout> | undefined;
  let holdPoint: { x: number; y: number } | undefined;
  const appearance = $derived((layout?.extensions.appearance ?? {}) as Record<string, unknown>);
  const rootFolder = $derived(layout?.folders[0]?.id ?? 'home');
  const usageButtons = $derived(
    (current?.buttons ?? []).filter((b) => b.action.type === 'usage' || b.action.type === 'metric'),
  );
  function navigate(folder: string, replace = false) {
    if (!layout?.folders.some((f) => f.id === folder)) return;
    active = folder;
    panel = 'deck';
    controls = false;
    const url = `#folder=${encodeURIComponent(folder)}`;
    if (replace) history.replaceState(null, '', url);
    else history.pushState(null, '', url);
  }
  function route() {
    if (location.hash === '#settings' && editor) {
      panel = 'settings';
      void getDevices();
    } else {
      panel = 'deck';
      active = decodeURIComponent(location.hash.replace(/^#folder=/, '')) || rootFolder;
    }
  }
  function back() {
    if (active !== rootFolder) navigate(rootFolder);
    else panel = 'deck';
  }
  function openSettings() {
    if (!editor) return;
    controls = false;
    panel = 'settings';
    history.pushState(null, '', '#settings');
    void getDevices();
  }
  async function toggleEdit() {
    if (!editor) return;
    if (editing && (editor.dirty || savingPromise)) await persist();
    controls = false;
    panel = 'deck';
    editing = !editing;
  }
  function keyboard(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      controls = false;
      help = false;
      button = null;
      if (panel === 'settings') navigate(active);
      return;
    }
    if (
      e.target instanceof HTMLElement &&
      e.target.closest('input,textarea,select,[contenteditable]')
    )
      return;
    if (e.key === 'F1') {
      e.preventDefault();
      help = !help;
      return;
    }
    if (button || help || e.repeat) return;
    if (!e.ctrlKey && !e.metaKey && !e.altKey && e.key.toLowerCase() === 'q') {
      e.preventDefault();
      void attempt(toggleEdit);
    }
    if ((e.ctrlKey || e.metaKey) && e.key === ',') {
      e.preventDefault();
      openSettings();
    }
    if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault();
      back();
    }
  }
  function context(e: MouseEvent) {
    if (panel !== 'deck' || button) return;
    e.preventDefault();
    controls = true;
  }
  function startHold(e: PointerEvent) {
    if (e.pointerType !== 'touch' || panel !== 'deck' || button) return;
    holdPoint = { x: e.clientX, y: e.clientY };
    hold = setTimeout(() => {
      controls = true;
    }, 600);
  }
  function stopHold() {
    if (hold) clearTimeout(hold);
    holdPoint = undefined;
  }
  function moveHold(e: PointerEvent) {
    if (holdPoint && Math.hypot(e.clientX - holdPoint.x, e.clientY - holdPoint.y) > 8) stopHold();
  }
  function buttonStyle(b: Button) {
    const a = (b.extensions.appearance ?? {}) as Record<string, unknown>;
    return `--button-color:${b.color};--icon-size:${appearanceNumber(a.icon_size, appearanceNumber(appearance.icon_size, 42, 0, 200), 0, 200)}px;grid-column:span ${appearanceNumber(a.columns, 1, 1, layout?.columns ?? 4)};grid-row:span ${appearanceNumber(a.rows, 1, 1, 128)};`;
  }
  function setAppearance(key: string, value: unknown, target = false) {
    const owner = target ? button : editor?.draft.layout;
    if (!owner) return;
    const a = (owner.extensions.appearance ?? {}) as Record<string, unknown>;
    owner.extensions.appearance = { ...a, [key]: value };
    if (!target) change();
  }
  async function attempt(work: () => Promise<void>) {
    error = '';
    try {
      await work();
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) pairing = true;
      error = e instanceof Error ? e.message : String(e);
    }
  }
  async function load() {
    loading = true;
    try {
      deck = await boot();
      pairing = false;
      active = deck.layout.folders.some((f) => f.id === active)
        ? active
        : (deck.layout.folders[0]?.id ?? 'home');
      const tr = await request<TranslationsResponse>('translations', 'TranslationsResponse');
      translation = tr.translations;
      languages = tr.languages;
      catalog = await request<CatalogResponse>('commands', 'CatalogResponse');
      if (deck.can_edit) editor = new Editor(await config());
      await assets();
      connect();
      route();
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) pairing = true;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }
  async function assets() {
    for (const url of [...themeUrls, ...backgroundUrls, ...Object.values(assetUrls)])
      URL.revokeObjectURL(url);
    themeUrls = [];
    backgroundUrls = [];
    assetUrls = {};
    if (!layout) return;
    for (const id of layout.themes) {
      try {
        themeUrls.push(await asset(id));
      } catch {
        notice = 'A theme is unavailable';
      }
    }
    for (const id of layout.backgrounds) {
      try {
        backgroundUrls.push(await asset(id));
      } catch {
        notice = 'A background is unavailable';
      }
    }
    for (const f of layout.folders) {
      for (const b of f.buttons) {
        if (b.icon.startsWith('asset:')) {
          const id = b.icon.slice(6);
          try {
            assetUrls[id] = await asset(id);
          } catch {
            notice = 'A button image is unavailable';
          }
        }
      }
    }
  }
  async function invoke(b: Button) {
    if (controls) return;
    if (editing) {
      freezeCells();
      button = clone(b);
      buttonFolder = current?.id ?? '';
      buttonOrigin = buttonFolder;
      return;
    }
    if (running[b.id]) return;
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
      back();
      return;
    }
    if (a.type === 'edit') {
      return;
    }
    if (a.type === 'none') return;
    if (a.type === 'usage' || a.type === 'metric') {
      await refreshUsage();
      return;
    }
    running = { ...running, [b.id]: true };
    try {
      const event = await execute({ type: 'button', button_id: b.id }, transport);
      if (event.state === 'failed') throw new Error(event.message);
      notice = t('success', 'Completed');
      setTimeout(() => {
        if (notice === t('success', 'Completed')) notice = '';
      }, 1800);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      running = { ...running, [b.id]: false };
    }
  }
  function change() {
    editor?.change();
  }
  function newButton(cell: number) {
    freezeCells();
    buttonFolder = current?.id ?? 'home';
    buttonOrigin = buttonFolder;
    button = {
      id: id(),
      label: 'New button',
      icon: '✦',
      color: '#6654e8',
      action: { type: 'command', command: { type: 'play_pause' } },
      extensions: { appearance: { cell } },
    };
  }
  function commitButton() {
    if (!editor || !button) return;
    try {
      contract('Button', button);
      freezeCells();
      const f = editor.draft.layout.folders.find((f) => f.id === buttonFolder);
      if (!f) return;
      if (buttonOrigin !== buttonFolder) {
        const cell =
          gridCells(f.buttons, layout?.columns ?? 4, layout?.rows ?? 3).find(
            (c) => !c.button && !c.covered,
          )?.cell ?? f.buttons.length;
        button.extensions.appearance = {
          ...((button.extensions.appearance as Record<string, unknown>) ?? {}),
          cell,
        };
        const origin = editor.draft.layout.folders.find((f) => f.id === buttonOrigin);
        if (origin) origin.buttons = origin.buttons.filter((b) => b.id !== button?.id);
      }
      const index = f.buttons.findIndex((b) => b.id === button?.id);
      if (index < 0) f.buttons.push(clone(button));
      else f.buttons[index] = clone(button);
      change();
      button = null;
    } catch (e) {
      error = String(e);
    }
  }
  function removeButton() {
    if (!editor || !button) return;
    freezeCells();
    for (const f of editor.draft.layout.folders)
      f.buttons = f.buttons.filter((b) => b.id !== button?.id);
    change();
    button = null;
  }
  function newFolder() {
    if (!editor) return;
    freezeCells();
    const parent = current;
    const cell = cells.find((c) => !c.button && !c.covered)?.cell ?? cells.length;
    const f = {
      id: id(),
      label: 'New folder',
      buttons: [
        {
          id: id(),
          label: 'Back',
          icon: '↩',
          color: '#273549',
          action: { type: 'folder' as const, folder_id: parent?.id ?? rootFolder },
          extensions: {},
        },
      ],
      extensions: {},
    };
    editor.draft.layout.folders.push(f);
    parent?.buttons.push({
      id: id(),
      label: f.label,
      icon: '▦',
      color: '#273549',
      action: { type: 'folder', folder_id: f.id },
      extensions: { appearance: { cell } },
    });
    navigate(f.id);
    change();
  }
  function renameFolder(label: string) {
    if (!current || !editor) return;
    const previous = current.label;
    current.label = label;
    for (const f of editor.draft.layout.folders)
      for (const b of f.buttons)
        if (b.action.type === 'folder' && b.action.folder_id === current.id && b.label === previous)
          b.label = label;
    change();
  }
  function removeFolder() {
    if (!editor || !current) return;
    const id = current.id;
    if (editor.draft.layout.folders.length <= 1) {
      error = 'The deck needs at least one folder';
      return;
    }
    freezeCells();
    for (const f of editor.draft.layout.folders)
      f.buttons = f.buttons.filter(
        (b) => !(b.action.type === 'folder' && b.action.folder_id === id),
      );
    editor.draft.layout.folders = editor.draft.layout.folders.filter((f) => f.id !== id);
    active = editor.draft.layout.folders[0]?.id ?? 'home';
    change();
  }
  let savingPromise: Promise<void> | null = null;
  async function persist() {
    if (savingPromise) return savingPromise;
    savingPromise = persistOnce();
    try {
      await savingPromise;
    } finally {
      savingPromise = null;
    }
  }
  async function persistOnce() {
    if (!editor) return;
    contract('Config', editor.draft);
    const snapshot = await save(editor.revision, editor.draft);
    editor.saved(snapshot);
    deck = await boot();
    notice = t('saved', 'Saved');
    await assets();
  }
  async function refreshUsage() {
    usage = await request<UsageResponse>('usage', 'UsageResponse');
  }
  async function getDevices() {
    try {
      devices = (await getDevicesRequest()).devices;
    } catch {
      devices = [];
    }
  }
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
  function chooseCommand(c: Command) {
    if (!button) return;
    button.action = { type: 'command', command: c };
    if (c.type === 'plugin') {
      const m = catalog?.plugins[0];
      if (m) pluginCommand(m);
    }
  }
  function moveButton(delta: number) {
    if (!editor || !button) return;
    const f = editor.draft.layout.folders.find((f) => f.id === buttonFolder);
    if (!f) return;
    freezeCells();
    const oldCell = Number((button.extensions.appearance as Record<string, unknown>)?.cell ?? 0);
    const newCell = Math.max(0, oldCell + delta);
    const other = f.buttons.find(
      (b) =>
        b.id !== button?.id &&
        (b.extensions.appearance as Record<string, unknown>)?.cell === newCell,
    );
    if (other)
      other.extensions.appearance = {
        ...(other.extensions.appearance as Record<string, unknown>),
        cell: oldCell,
      };
    button.extensions.appearance = {
      ...((button.extensions.appearance as Record<string, unknown>) ?? {}),
      cell: newCell,
    };
    change();
  }
  function canRun(b: Button) {
    if (editing) return true;
    if (b.action.type === 'settings' || b.action.type === 'edit') return deck?.can_edit ?? false;
    if (b.action.type === 'command') {
      const capability = deck?.button_capabilities[b.id];
      return !capability || !!deck?.capabilities.includes(capability);
    }
    return true;
  }
  async function addDevice() {
    grant = await approve({
      name: deviceName,
      capabilities: grantCapabilities,
      ttl_seconds: grantTtl,
    });
    await getDevices();
  }
  async function importConfig(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file || !editor) return;
    const value: unknown = JSON.parse(await file.text());
    editor.draft = contract('Config', value);
    editor.change();
    notice = 'Backup loaded into your draft. Save to apply.';
  }
  async function uploadBackground(e: Event, theme = false) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f || !editor) return;
    const source = await upload(f);
    if (source.type === 'asset') {
      (theme ? editor.draft.layout.themes : editor.draft.layout.backgrounds).push(source.id);
      change();
      await assets();
    }
  }
  async function image(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f && button) {
      const s = await upload(f);
      if (s.type === 'asset') button.icon = `asset:${s.id}`;
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
  $effect(() => {
    if (panel !== 'deck' || !usageButtons.length) return;
    const interval = Math.min(
      ...usageButtons.map((b) => (b.action.type === 'metric' ? b.action.interval_ms : 1000)),
    );
    const needsUsage = usageButtons.some(
      (b) => b.action.type !== 'metric' || b.action.metric !== 'clock',
    );
    let disposed = false,
      inFlight = false;
    async function update() {
      now = Date.now();
      if (inFlight || !needsUsage || document.hidden) return;
      inFlight = true;
      try {
        const value = await request<UsageResponse>('usage', 'UsageResponse', {
          signal: AbortSignal.timeout(5000),
        });
        if (!disposed) usage = value;
      } catch (e) {
        if (!disposed) console.warn('Usage refresh:', e);
      } finally {
        inFlight = false;
      }
    }
    void update();
    const timer = setInterval(() => void update(), interval);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  });
  onMount(() => {
    void load();
    return () => {
      stopHold();
      for (const u of [...themeUrls, ...backgroundUrls, ...Object.values(assetUrls)])
        URL.revokeObjectURL(u);
    };
  });
</script>

<svelte:window
  onkeydown={keyboard}
  onpopstate={route}
  onhashchange={route}
  oncontextmenu={context}
  onpointerdown={startHold}
  onpointerup={stopHold}
  onpointermove={moveHold}
/>

<svelte:head
  ><title>WebDeck · v2</title>{#each themeUrls as href}<link
      rel="stylesheet"
      {href}
    />{/each}</svelte:head
>
<div
  class="application"
  style:background-image={backgroundUrls[0]
    ? `linear-gradient(#10121acc,#10121acc), url("${backgroundUrls[0]}")`
    : undefined}
>
  {#if error}<div class="banner error" role="alert">
      {error}<button aria-label="Dismiss error" onclick={() => (error = '')}>×</button>
    </div>{/if}
  {#if notice}<div class="banner" role="status">
      {notice}<button aria-label="Dismiss notice" onclick={() => (notice = '')}>×</button>
    </div>{/if}
  {#if loading}<main aria-busy="true"><p>Loading WebDeck…</p></main>
  {:else if pairing}<main class="pairing">
      <h1>Connect your device</h1>
      <p>Ask the local administrator to approve this device, then enter its pairing token.</p>
      <label
        >Pairing token<input type="password" bind:value={credential} autocomplete="off" /></label
      ><button
        class="primary"
        onclick={() =>
          attempt(async () => {
            setToken(credential);
            credential = '';
            await load();
          })}>Connect</button
      >
    </main>
  {:else if deck}
    <main>
      {#if panel === 'deck'}
        <h1 class="sr-only">{current?.label ?? 'Deck'}</h1>
        {#if editing && current && editor}<details class="edit-options" open>
            <summary>Folder options</summary>
            <div class="toolbar">
              {#if editor.dirty}<button class="primary" onclick={() => attempt(persist)}
                  >Save changes</button
                >{/if}
              <label
                >Folder<select value={active} onchange={(e) => navigate(e.currentTarget.value)}
                  >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option
                    >{/each}</select
                ></label
              >
              <label
                >Folder name<input
                  value={current.label}
                  oninput={(e) => renameFolder(e.currentTarget.value)}
                /></label
              >

              <button onclick={newFolder}>+ Add folder</button>
              <button onclick={openSettings}>Settings</button>
              <button class="danger" onclick={removeFolder}>Delete folder</button>
            </div>
          </details>{/if}
        <div
          class="deck-grid all-buttons"
          style:grid-template-columns={`repeat(${layout?.columns ?? 4}, minmax(0, 1fr))`}
          style:gap={`${appearanceNumber(appearance.gap, 10, 0, 100)}px`}
          style:--button-height={`${appearanceNumber(appearance.button_height, 140, 24, 800)}px`}
          style:--button-radius={`${appearanceNumber(appearance.radius, 8, 0, 100)}px`}
        >
          {#each cells.filter((c) => !c.covered) as slot (slot.cell)}<div
              class="deck-cell"
              data-cell={slot.cell}
              style:grid-column={`${(slot.cell % (layout?.columns ?? 4)) + 1} / span ${slot.columns}`}
              style:grid-row={`${Math.floor(slot.cell / (layout?.columns ?? 4)) + 1} / span ${slot.rows}`}
            >
              {#if slot.button}{@const b = slot.button}<button
                  class="deck-button button"
                  style={buttonStyle(b)}
                  class:blank={b.action.type === 'none'}
                  disabled={running[b.id] || !canRun(b)}
                  aria-label={b.label}
                  onclick={() => {
                    if (!editing) void attempt(() => invoke(b));
                  }}
                  >{#if b.icon.startsWith('asset:') && assetUrls[b.icon.slice(6)]}<img
                      src={assetUrls[b.icon.slice(6)]}
                      alt=""
                    />{:else}<span class="button-icon" aria-hidden="true">{b.icon}</span>{/if}<span
                    class:hidden-label={appearance.show_labels === false ||
                      (b.extensions.appearance as Record<string, unknown> | undefined)
                        ?.show_label === false}>{b.label}</span
                  >{#if b.action.type === 'metric' || b.action.type === 'usage'}{@const reading =
                      metricReading(b, usage?.usage, now)}<strong class="metric-value"
                      >{reading.text}</strong
                    >{#if reading.percent !== undefined}<progress
                        max="100"
                        value={reading.percent}
                        aria-label={`${b.label} usage`}
                      ></progress>{/if}{/if}<small>{running[b.id] ? 'Running…' : ''}</small></button
                >
                {#if editing}<div class="cell-actions">
                    <button aria-label={`Edit ${b.label}`} onclick={() => attempt(() => invoke(b))}
                      >Edit</button
                    ><button aria-label={`Remove ${b.label}`} onclick={() => deleteCell(b)}
                      >Remove</button
                    >
                  </div>{/if}
              {:else if editing}<button
                  class="deck-button add"
                  onclick={() => newButton(slot.cell)}
                  aria-label={`Add button in cell ${slot.cell + 1}`}>+</button
                >{:else}<div class="empty-cell" aria-hidden="true"></div>{/if}
            </div>{/each}
        </div>
      {:else if panel === 'settings' && editor}
        <div class="heading">
          <h1>{t('settings', 'Settings')}</h1>
          <button onclick={() => navigate(active)}>Back to deck</button>
          <button class="primary" onclick={() => attempt(persist)}>Save changes</button>
        </div>
        <section>
          <h2>Appearance</h2>
          <label
            >Connection<select bind:value={transport}
              ><option value="http">HTTP</option><option value="socket">Realtime</option></select
            ></label
          >
          <div class="row">
            <label
              >Button height<input
                type="number"
                value={appearance.button_height ?? 140}
                oninput={(e) => setAppearance('button_height', +e.currentTarget.value)}
              /></label
            >
            <label
              >Gap<input
                type="number"
                value={appearance.gap ?? 10}
                oninput={(e) => setAppearance('gap', +e.currentTarget.value)}
              /></label
            >
            <label
              >Corner radius<input
                type="number"
                value={appearance.radius ?? 8}
                oninput={(e) => setAppearance('radius', +e.currentTarget.value)}
              /></label
            >
            <label
              >Icon size<input
                type="number"
                value={appearance.icon_size ?? 42}
                oninput={(e) => setAppearance('icon_size', +e.currentTarget.value)}
              /></label
            >
            <label class="check"
              ><input
                type="checkbox"
                checked={appearance.show_labels !== false}
                onchange={(e) => setAppearance('show_labels', e.currentTarget.checked)}
              />Show button labels</label
            >
          </div>
          <div class="row">
            <label
              >Columns<input
                type="number"
                min="1"
                max="128"
                bind:value={editor.draft.layout.columns}
                oninput={change}
              /></label
            ><label
              >Rows<input
                type="number"
                min="1"
                max="128"
                bind:value={editor.draft.layout.rows}
                oninput={change}
              /></label
            ><label
              >Language<select bind:value={editor.draft.settings.language} onchange={change}
                >{#each languages as l}<option value={l}>{l}</option>{/each}</select
              ></label
            >
          </div>
          <label
            >Upload a background<input
              type="file"
              accept="image/*"
              onchange={(e) => attempt(() => uploadBackground(e))}
            /></label
          >{#each editor.draft.layout.backgrounds as b}<div class="row">
              <span>{b}</span><button
                onclick={() => {
                  if (editor)
                    editor.draft.layout.backgrounds = editor.draft.layout.backgrounds.filter(
                      (v) => v !== b,
                    );
                  change();
                  void assets();
                }}>Remove</button
              >
            </div>{/each}<label
            >Upload a theme<input
              type="file"
              accept=".css"
              onchange={(e) => attempt(() => uploadBackground(e, true))}
            /></label
          >{#each editor.draft.layout.themes as theme}<div class="row">
              <span>{theme}</span><button
                onclick={() => {
                  if (editor)
                    editor.draft.layout.themes = editor.draft.layout.themes.filter(
                      (v) => v !== theme,
                    );
                  change();
                  void assets();
                }}>Remove</button
              >
            </div>{/each}
        </section>
        <section>
          <h2>Integrations</h2>
          <h3>OBS</h3>
          <label>Host<input bind:value={editor.draft.settings.obs.host} oninput={change} /></label
          ><label
            >Port<input
              type="number"
              bind:value={editor.draft.settings.obs.port}
              oninput={change}
            /></label
          ><label
            >Password<input
              type="password"
              bind:value={editor.draft.settings.obs.password}
              oninput={change}
            /></label
          >
          <h3>Spotify</h3>
          <label
            >Client ID<input
              bind:value={editor.draft.settings.spotify.client_id}
              oninput={change}
            /></label
          ><label
            >Client secret<input
              type="password"
              bind:value={editor.draft.settings.spotify.client_secret}
              oninput={change}
            /></label
          ><label
            >Redirect URI<input
              bind:value={editor.draft.settings.spotify.redirect_uri}
              oninput={change}
            /></label
          ><button
            onclick={() =>
              attempt(async () => {
                await persist();
                const r = await request<SpotifyConnect>('spotify/connect', 'SpotifyConnect', {
                  method: 'POST',
                });
                window.open(r.url, '_blank', 'noopener');
              })}>Connect Spotify</button
          >
          <p>Automatic updates are disabled for this prerelease.</p>
        </section>
        <section>
          <h2>Paired devices</h2>
          <label>Device name<input bind:value={deviceName} /></label>
          <fieldset>
            <legend>Permissions</legend
            >{#each ['read', 'input', 'audio', 'window', 'power', 'script', 'network', 'plugin', 'settings'] as c}<label
                class="check"
                ><input type="checkbox" value={c} bind:group={grantCapabilities} />{c}</label
              >{/each}
          </fieldset>
          <label
            >Expires after (seconds)<input
              type="number"
              min="1"
              max="86400"
              bind:value={grantTtl}
            /></label
          ><button
            disabled={!deviceName.trim() || !grantCapabilities.length}
            onclick={() => attempt(addDevice)}>Approve device</button
          >{#if grant}<p>Copy this token now. It will not be shown again.</p>
            <code class="token">{grant.token}</code><button
              onclick={() => {
                grant = null;
              }}>Hide token</button
            >{/if}{#each devices as device}<div class="row">
              <span>{device.name} · {device.revoked ? 'Revoked' : 'Approved'}</span><button
                disabled={device.revoked}
                onclick={() =>
                  attempt(async () => {
                    await revoke(device.id);
                    await getDevices();
                  })}>Revoke</button
              >
            </div>{/each}
        </section>
        <section>
          <h2>Configuration</h2>
          <label
            >Restore a v2 backup<input
              type="file"
              accept=".json"
              onchange={(e) => attempt(() => importConfig(e))}
            /></label
          >
          <p>Your unsaved draft remains available after a save conflict.</p>
          <button
            onclick={() => {
              const snapshot = {
                api_version: 2 as const,
                revision: editor!.revision,
                config: editor!.draft,
              };
              const url = URL.createObjectURL(
                new Blob([JSON.stringify(snapshot.config, null, 2)], { type: 'application/json' }),
              );
              const a = document.createElement('a');
              a.href = url;
              a.download = 'webdeck-v2-config.json';
              a.click();
              URL.revokeObjectURL(url);
            }}>Download backup</button
          ><button
            onclick={() =>
              attempt(async () => {
                if (editor?.dirty && !window.confirm('Discard your unsaved draft and reload?'))
                  return;
                editor = new Editor(await config());
                await load();
              })}>Reload configuration</button
          >
        </section>
      {/if}
    </main>
  {/if}
</div>
{#if controls}
  <div class="overlay">
    <dialog open class="deck-menu" aria-label="Deck controls">
      <button aria-label="Close controls" onclick={() => (controls = false)}>×</button>
      <button
        onclick={() => {
          back();
          controls = false;
        }}>Back</button
      >
      <button onclick={() => navigate(rootFolder)}>Home</button>
      {#if editor}<button onclick={openSettings}>Settings</button>{/if}
      <button
        onclick={() => {
          controls = false;
          void attempt(load);
        }}>Reload</button
      >
      <p>Right-click or hold the deck for controls. Q toggles editing; F1 shows all shortcuts.</p>
    </dialog>
  </div>
{/if}
{#if help}<div class="overlay">
    <dialog open class="deck-menu" aria-label="Keyboard shortcuts">
      <button onclick={() => (help = false)}>Close</button>
      <h2>Shortcuts</h2>
      <p>Q — toggle edit mode (saves changes when leaving)</p>
      <p>F1 — show or hide shortcuts</p>
      <p>Ctrl+, — settings</p>
      <p>Alt+Left — return to home folder</p>
      <p>Escape — close a dialog or settings</p>
      <p>Right-click or touch and hold — deck controls</p>
    </dialog>
  </div>{/if}
{#if button}
  <div class="overlay">
    <dialog
      bind:this={dialog}
      oncancel={() => (button = null)}
      aria-modal="true"
      aria-labelledby="button-title"
      class="editor"
    >
      <div class="heading">
        <h2 id="button-title">Edit button</h2>
        <button aria-label="Close editor" onclick={() => (button = null)}>×</button>
      </div>
      <label
        >Button folder<select bind:value={buttonFolder}
          >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option>{/each}</select
        ></label
      >
      <label>Label<input bind:value={button.label} /></label><label
        >Icon<input bind:value={button.icon} /></label
      ><label
        >Upload image<input
          type="file"
          accept="image/*"
          onchange={(e) => attempt(() => image(e))}
        /></label
      ><label>Color<input type="color" bind:value={button.color} /></label><label
        >Action<select
          value={button.action.type}
          onchange={(e) => {
            if (!button) return;
            const type = e.currentTarget.value;
            if (type === 'command') button.action = { type, command: defaultCommand() };
            else if (type === 'metric')
              button.action = { type, metric: 'cpu', target: '', interval_ms: 1000 };
            else if (type === 'folder')
              button.action = { type, folder_id: layout?.folders[0]?.id ?? 'home' };
            else button.action = { type } as ButtonAction;
          }}
          >{#each ['command', 'folder', 'back', 'reload', 'fullscreen', 'settings', 'metric', 'none'] as type}<option
              value={type}>{type}</option
            >{/each}</select
        ></label
      >
      <div class="row">
        <label
          >Column span<input
            type="number"
            value={(button.extensions.appearance as Record<string, unknown> | undefined)?.columns ??
              1}
            oninput={(e) => setAppearance('columns', +e.currentTarget.value, true)}
          /></label
        >
        <label
          >Row span<input
            type="number"
            value={(button.extensions.appearance as Record<string, unknown> | undefined)?.rows ?? 1}
            oninput={(e) => setAppearance('rows', +e.currentTarget.value, true)}
          /></label
        >
        <label
          >Icon size<input
            type="number"
            value={(button.extensions.appearance as Record<string, unknown> | undefined)
              ?.icon_size ?? 42}
            oninput={(e) => setAppearance('icon_size', +e.currentTarget.value, true)}
          /></label
        >
        <label class="check"
          ><input
            type="checkbox"
            checked={(button.extensions.appearance as Record<string, unknown> | undefined)
              ?.show_label !== false}
            onchange={(e) => setAppearance('show_label', e.currentTarget.checked, true)}
          />Show label</label
        >
      </div>
      {#if button.action.type === 'metric'}
        <label
          >Metric<select bind:value={button.action.metric}
            >{#each ['cpu', 'memory', 'gpu', 'gpu_memory', 'disk', 'clock'] as metric}<option
                value={metric}>{metric}</option
              >{/each}</select
          ></label
        >
        <label
          >Device name or index (empty uses default)<input
            bind:value={button.action.target}
          /></label
        >
        <label
          >Update interval (ms)<input
            type="number"
            min="250"
            bind:value={button.action.interval_ms}
          /></label
        >
      {:else if button.action.type === 'folder'}<label
          >Destination<select bind:value={button.action.folder_id}
            >{#each layout?.folders ?? [] as f}<option value={f.id}>{f.label}</option
              >{/each}</select
          ></label
        >{:else if button.action.type === 'command'}<Fields
          schema={commandFormSchema}
          value={button.action.command}
          onchange={(c) => chooseCommand(c as Command)}
          label="Command"
        />{#if button.action.command.type === 'plugin'}<label
            >Plugin<select
              value={button.action.command.plugin_id}
              onchange={(e) => {
                const m = catalog?.plugins.find((m) => m.id === e.currentTarget.value);
                if (m) pluginCommand(m);
              }}
              >{#each catalog?.plugins ?? [] as m}<option value={m.id}>{m.id} · {m.version}</option
                >{/each}</select
            ></label
          >{#if selectedPlugin}<label
              >Plugin action<select
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
                if (button?.action.type === 'command' && button.action.command.type === 'plugin')
                  button.action.command.args = args as Record<string, unknown>;
              }}
              label="Plugin arguments"
            />{:else}<p>No v2 plugins are installed.</p>{/if}{/if}{/if}
      <div class="row">
        <button onclick={() => moveButton(-1)}>Move left</button><button
          onclick={() => moveButton(1)}>Move right</button
        ><button class="danger" onclick={removeButton}>Delete</button><button
          onclick={() => (button = null)}>Cancel</button
        ><button class="primary" onclick={commitButton}>Apply to draft</button>
      </div>
    </dialog>
  </div>
{/if}
