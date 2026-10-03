<script lang="ts">
  import { useTranslations } from '../../lib/i18n';
  const t = useTranslations();

  let { connect }: { connect: (token: string) => Promise<boolean> } = $props();
  let credential = $state(''),
    working = $state(false);
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (working || !credential.trim()) return;
    working = true;
    try {
      if (await connect(credential)) credential = '';
    } finally {
      working = false;
    }
  }
</script>

<main class="pairing">
  <h1>{t('ui_connect_your_device')}</h1>
  <p id="pairing-help">
    {t('ui_on_the_host_computer_open_settings_devices_and_approve_this_device_enter_the_one')}
  </p>
  <form onsubmit={submit}>
    <label
      >{t('ui_pairing_token')}<input
        aria-describedby="pairing-help"
        type="password"
        bind:value={credential}
        autocomplete="off"
        required
      /></label
    >
    <button aria-describedby="pairing-help" class="primary" disabled={working || !credential.trim()}
      >{working ? t('ui_connecting') : t('ui_connect')}</button
    >
  </form>
</main>

<style>
  .pairing {
    max-width: 520px;
    margin: 40px auto;
    padding: var(--space-5);
  }
  p {
    color: var(--text-muted);
    line-height: 1.6;
  }
</style>
