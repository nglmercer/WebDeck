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
  import { Editor, clone } from './editor.svelte';
  let deck = $state<DeckBoot | null>(null),
    editor = $state<Editor | null>(null),
    active = $state('home');
  let error = $state(''),
    notice = $state(''),
    loading = $state(true),
    pairing = $state(false),
    credential = $state('');
  let panel = $state<'deck' | 'settings' | 'usage'>('deck'),
    editing = $state(false),
    button = $state<Button | null>(null),
    buttonFolder = $state(''),
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
  const busy = $derived(Object.values(running).some(Boolean));
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
      active = deck.layout.folders[0]?.id ?? 'home';
      const tr = await request<TranslationsResponse>('translations', 'TranslationsResponse');
      translation = tr.translations;
      languages = tr.languages;
      catalog = await request<CatalogResponse>('commands', 'CatalogResponse');
      if (deck.can_edit) editor = new Editor(await config());
      await assets();
      connect();
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
    if (editing) {
      button = clone(b);
      buttonFolder = current?.id ?? '';
      return;
    }
    if (running[b.id]) return;
    const a = b.action;
    if (a.type === 'folder') {
      active = a.folder_id;
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
      panel = 'settings';
      await getDevices();
      return;
    }
    if (a.type === 'usage') {
      await showUsage();
      return;
    }
    running = { ...running, [b.id]: true };
    try {
      const event = await execute(a.command, transport);
      if (event.state === 'failed') throw new Error(event.message);
      notice = t('success', 'Completed');
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      running = { ...running, [b.id]: false };
    }
  }
  function change() {
    editor?.change();
  }
  function newButton() {
    buttonFolder = current?.id ?? 'home';
    button = {
      id: id(),
      label: 'New button',
      icon: '✦',
      color: '#6654e8',
      action: { type: 'command', command: { type: 'play_pause' } },
      extensions: {},
    };
  }
  function commitButton() {
    if (!editor || !button) return;
    try {
      contract('Button', button);
      const f = editor.draft.layout.folders.find((f) => f.id === buttonFolder);
      if (!f) return;
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
    const f = editor.draft.layout.folders.find((f) => f.id === buttonFolder);
    if (f) f.buttons = f.buttons.filter((b) => b.id !== button?.id);
    change();
    button = null;
  }
  function newFolder() {
    if (!editor) return;
    const f = { id: id(), label: 'New folder', buttons: [], extensions: {} };
    editor.draft.layout.folders.push(f);
    active = f.id;
    change();
  }
  function removeFolder() {
    if (!editor || !current) return;
    const id = current.id;
    if (editor.draft.layout.folders.length <= 1) {
      error = 'The deck needs at least one folder';
      return;
    }
    if (
      editor.draft.layout.folders.some((f) =>
        f.buttons.some((b) => b.action.type === 'folder' && b.action.folder_id === id),
      )
    ) {
      error = 'Remove links to this folder before deleting it';
      return;
    }
    editor.draft.layout.folders = editor.draft.layout.folders.filter((f) => f.id !== id);
    active = editor.draft.layout.folders[0]?.id ?? 'home';
    change();
  }
  async function persist() {
    if (!editor) return;
    contract('Config', editor.draft);
    const snapshot = await save(editor.revision, editor.draft);
    editor.saved(snapshot);
    deck = await boot();
    notice = t('saved', 'Saved');
    await assets();
  }
  async function showUsage() {
    panel = 'usage';
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
    const i = f.buttons.findIndex((b) => b.id === button?.id),
      j = i + delta;
    if (i < 0 || j < 0 || j >= f.buttons.length) return;
    const b = f.buttons.splice(i, 1)[0];
    if (b) f.buttons.splice(j, 0, b);
    change();
  }
  function canRun(b: Button) {
    if (editing) return true;
    if (b.action.type === 'settings') return deck?.can_edit ?? false;
    if (b.action.type === 'command') {
      const capability = deck?.button_capabilities[b.id];
      if (capability && !deck?.capabilities.includes(capability)) return false;
      const c = catalog?.commands.find(
        (c) => b.action.type === 'command' && c.id === b.action.command.type,
      );
      return !!c && !!deck?.capabilities.includes(c.capability);
    }
    return true;
  }
  const bytes = (n: number) => `${(n / 1024 ** 3).toFixed(1)} GB`;
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
    if (panel !== 'usage') return;
    let disposed = false;
    let inFlight = false;
    const timer = setInterval(async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const value = await request<UsageResponse>('usage', 'UsageResponse');
        if (!disposed) usage = value;
      } catch (e) {
        if (!disposed) error = e instanceof Error ? e.message : String(e);
      } finally {
        inFlight = false;
      }
    }, 1000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  });
  onMount(() => {
    void load();
    return () => {
      for (const u of [...themeUrls, ...backgroundUrls, ...Object.values(assetUrls)])
        URL.revokeObjectURL(u);
    };
  });
</script>

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
  <header>
    <a class="brand" href="/">W<span>WebDeck</span><small>v2</small></a>
    <nav aria-label="Main navigation">
      <button class:active={panel === 'deck'} onclick={() => (panel = 'deck')}
        >{t('buttons', 'Deck')}</button
      ><button onclick={() => attempt(showUsage)}>{t('usage', 'Usage')}</button
      >{#if deck?.can_edit}<button
          onclick={() => {
            panel = 'settings';
            void getDevices();
          }}>{t('settings', 'Settings')}</button
        >{/if}
    </nav>
    <span class="status">{busy ? 'Running' : 'Ready'}</span>
  </header>
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
    <aside>
      <h2>{t('folders', 'Folders')}</h2>
      {#each layout?.folders ?? [] as f}<button
          class:active={f.id === current?.id}
          onclick={() => (active = f.id)}
          >▦ <span>{f.label}</span><small>{f.buttons.length}</small></button
        >{/each}{#if editing}<button onclick={newFolder}>+ Add folder</button>{/if}<label
        >Connection<select bind:value={transport}
          ><option value="http">HTTP</option><option value="socket">Realtime</option></select
        ></label
      >
    </aside>
    <main>
      {#if panel === 'deck'}
        <div class="heading">
          <div>
            <p class="eyebrow">YOUR CONTROL SPACE</p>
            <h1>{current?.label ?? 'Deck'}</h1>
          </div>
          {#if editor}<div class="row">
              <button onclick={() => (editing = !editing)}
                >{editing ? t('done', 'Done') : t('edit', 'Edit deck')}</button
              >{#if editor.dirty}<button class="primary" onclick={() => attempt(persist)}
                  >{t('save', 'Save changes')}</button
                >{/if}
            </div>{/if}
        </div>
        {#if editing && current && editor}<div class="toolbar">
            <label>Folder name<input bind:value={current.label} oninput={change} /></label><button
              onclick={newButton}>+ Add button</button
            ><button class="danger" onclick={removeFolder}>Delete folder</button>
          </div>{/if}
        <div
          class="deck-grid all-buttons"
          style:grid-template-columns={`repeat(${layout?.columns ?? 4}, minmax(100px, 1fr))`}
        >
          {#each current?.buttons ?? [] as b (b.id)}<button
              class="deck-button button"
              style:--button-color={b.color}
              disabled={running[b.id] || !canRun(b)}
              aria-label={editing ? `Edit ${b.label}` : b.label}
              onclick={() => attempt(() => invoke(b))}
              >{#if b.icon.startsWith('asset:') && assetUrls[b.icon.slice(6)]}<img
                  src={assetUrls[b.icon.slice(6)]}
                  alt=""
                />{:else}<span class="button-icon" aria-hidden="true">{b.icon}</span>{/if}<span
                >{b.label}</span
              ><small
                >{running[b.id]
                  ? 'Running…'
                  : editing
                    ? 'Edit'
                    : b.action.type === 'folder'
                      ? 'Open folder'
                      : ' '}</small
              ></button
            >{/each}
          {#each Array.from( { length: Math.max(0, (layout?.columns ?? 4) * (layout?.rows ?? 3) - (current?.buttons.length ?? 0)) } ) as _, index}{#if editing}<button
                class="deck-button add"
                onclick={newButton}
                aria-label={`Add button in slot ${index + 1}`}>+<span>Add button</span></button
              >{:else}<div class="deck-slot" aria-hidden="true"></div>{/if}{/each}
        </div>
        {#if !current?.buttons.length}<p class="empty">
            This folder is ready for your controls.
          </p>{/if}
      {:else if panel === 'settings' && editor}
        <div class="heading">
          <h1>{t('settings', 'Settings')}</h1>
          <button class="primary" onclick={() => attempt(persist)}>Save changes</button>
        </div>
        <section>
          <h2>Appearance</h2>
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
      {:else if panel === 'usage'}<div class="heading">
          <h1>System usage</h1>
          <button onclick={() => attempt(showUsage)}>Refresh</button>
        </div>
        {#if usage}<section>
            <h2>CPU</h2>
            <progress max="100" value={usage.usage.cpu_percent}></progress>
            <p>{usage.usage.cpu_percent.toFixed(1)}%</p>
            <h2>Memory</h2>
            <progress max={usage.usage.memory_total} value={usage.usage.memory_used}></progress>
            <p>{bytes(usage.usage.memory_used)} / {bytes(usage.usage.memory_total)}</p>
          </section>
          <section>
            <h2>Disks</h2>
            {#each usage.usage.disks as d}<h3>{d.name}</h3>
              <progress max={d.total} value={d.total - d.available}></progress>
              <p>{bytes(d.available)} available / {bytes(d.total)}</p>{/each}
          </section>
          {#each usage.usage.gpus as g}<section>
              <h2>{g.name}</h2>
              <progress max="100" value={g.usage}></progress>
              <p>{g.usage.toFixed(1)}% · {bytes(g.memory_used)} / {bytes(g.memory_total)}</p>
            </section>{/each}{/if}{/if}
    </main>
  {/if}
  <footer>WebDeck · 2.0.0-alpha.1</footer>
</div>
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
            else if (type === 'folder')
              button.action = { type, folder_id: layout?.folders[0]?.id ?? 'home' };
            else button.action = { type } as ButtonAction;
          }}
          >{#each ['command', 'folder', 'reload', 'fullscreen', 'settings', 'usage'] as type}<option
              value={type}>{type}</option
            >{/each}</select
        ></label
      >
      {#if button.action.type === 'folder'}<label
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
