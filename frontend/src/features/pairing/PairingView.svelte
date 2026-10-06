<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { useTranslations } from '../../lib/i18n';
  import { request } from '../../lib/api/client';
  import type { PairingChallenge, PairingResult } from '../../lib/contracts';
  const t = useTranslations();
  let { connect }: { connect: (token: string, remember: boolean) => Promise<boolean> } = $props();
  let credential = $state(''),
    name = $state(''),
    remember = $state(true),
    working = $state(false);
  let challenge = $state<PairingChallenge | null>(null),
    message = $state(''),
    error = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  let destroyed = false;
  onDestroy(() => {
    destroyed = true;
    clearTimeout(timer);
  });
  function clearRequest() {
    challenge = null;
    sessionStorage.removeItem('webdeck.pairing');
    clearTimeout(timer);
  }
  async function poll() {
    const current = challenge;
    if (!current || destroyed) return;
    if (Date.now() / 1000 >= current.expires_at) {
      clearRequest();
      message = t('ui_pair_expired');
      return;
    }
    try {
      const result = await request<PairingResult>('pairing/claim', 'PairingResult', {
        method: 'POST',
        body: JSON.stringify({ id: current.id, secret: current.secret }),
      });
      if (destroyed || challenge !== current) return;
      if (result.state === 'approved') {
        clearRequest();
        if (!(await connect(result.token, remember))) error = t('ui_pair_connect_failed');
        return;
      }
      if (result.state !== 'pending') {
        clearRequest();
        message = t(result.state === 'rejected' ? 'ui_pair_rejected' : 'ui_pair_expired');
        return;
      }
      error = '';
    } catch (e) {
      if (!destroyed) error = e instanceof Error ? e.message : String(e);
    }
    if (!destroyed && challenge === current)
      timer = setTimeout(() => {
        void poll();
      }, 2000);
  }
  async function ask() {
    if (working || !name.trim()) return;
    working = true;
    error = '';
    message = '';
    try {
      const value = await request<PairingChallenge>('pairing/request', 'PairingChallenge', {
        method: 'POST',
        body: JSON.stringify({ name: name.trim() }),
      });
      if (destroyed) return;
      challenge = value;
      sessionStorage.setItem('webdeck.pairing', JSON.stringify(value));
      void poll();
    } catch (e) {
      if (!destroyed) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (!destroyed) working = false;
    }
  }
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (working || !credential.trim()) return;
    working = true;
    error = '';
    try {
      if (await connect(credential, remember)) {
        credential = '';
        clearRequest();
      } else error = t('ui_pair_invalid_token');
    } finally {
      working = false;
    }
  }
  onMount(() => {
    try {
      const saved = sessionStorage.getItem('webdeck.pairing');
      if (saved) {
        const value = JSON.parse(saved) as PairingChallenge;
        if (value.expires_at > Date.now() / 1000) {
          challenge = value;
          void poll();
        } else clearRequest();
      }
    } catch {
      clearRequest();
    }
  });
</script>

<main class="pairing">
  <div class="pair-icon" aria-hidden="true">↗</div>
  <h1>{t('ui_connect_your_device')}</h1>
  <p>{t('ui_pair_intro')}</p>
  {#if error}<p class="pair-error" role="alert">{error}</p>{/if}
  {#if message}<p role="status">{message}</p>{/if}
  <label class="check"
    ><input type="checkbox" bind:checked={remember} />{t('ui_remember_device')}</label
  >
  <p class="muted">{t('ui_remember_device_help')}</p>
  {#if challenge}
    <div class="waiting" role="status">
      <h2>{t('ui_waiting_host')}</h2>
      <div class="pair-code">{challenge.code}</div>
      <p>{t('ui_pair_compare_code')}</p>
      <p>{t('ui_pair_wait_help')}</p>
    </div>
  {:else}
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void ask();
      }}
    >
      <label
        >{t('ui_device_name')}<input
          bind:value={name}
          placeholder={t('ui_phone_name_example')}
          maxlength="128"
          autocomplete="nickname"
          required
        /></label
      >
      <button class="primary" disabled={working || !name.trim()}
        >{working ? t('ui_connecting') : t('ui_request_access')}</button
      >
    </form>
  {/if}
  <details>
    <summary>{t('ui_use_pairing_token')}</summary>
    <p>{t('ui_manual_pair_help')}</p>
    <form onsubmit={submit}>
      <label
        >{t('ui_pairing_token')}<input
          type="password"
          bind:value={credential}
          autocomplete="off"
          required
        /></label
      >
      <button class="primary" disabled={working || !credential.trim()}
        >{working ? t('ui_connecting') : t('ui_connect')}</button
      >
    </form>
  </details>
</main>

<style>
  .pairing {
    max-width: 480px;
    margin: 6vh auto;
    padding: 28px;
    border: 1px solid var(--border);
    border-radius: 20px;
    background: var(--surface);
  }
  .pair-icon {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    background: var(--accent-surface);
    font-size: 28px;
  }
  p {
    color: var(--text-muted);
    line-height: 1.6;
  }
  h1 {
    margin-top: 20px;
  }
  .muted {
    font-size: 0.8rem;
  }
  form button {
    width: 100%;
    margin-top: 12px;
  }
  .waiting {
    text-align: center;
    background: var(--canvas);
    border-radius: 14px;
    padding: 20px;
    margin: 20px 0;
  }
  .pair-code {
    font-size: 36px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.15em;
    color: var(--text);
    font-weight: 700;
  }
  details {
    margin-top: 24px;
    border-top: 1px solid var(--border);
    padding-top: 20px;
  }
  summary {
    cursor: pointer;
  }
  .pair-error {
    color: var(--danger);
  }
</style>
