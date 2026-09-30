<script lang="ts">
  import { setDeviceToken } from './session';
  let { retry }: { retry: () => Promise<void> } = $props();
  let token = $state('');
  let pending = $state(false);
  async function connect(event: SubmitEvent): Promise<void> {
    event.preventDefault(); if (pending) return;
    pending = true; setDeviceToken(token);
    try { await retry(); } finally { pending = false; }
  }
</script>
<main class="wd2-panel" style="color:white;margin:2rem;max-width:32rem">
  <h1>Connect this device</h1>
  <p>Ask the administrator to approve this device in WebDeck settings on the host computer. Enter the temporary token below.</p>
  <form onsubmit={connect}>
    <label for="device-token">Device token</label>
    <input id="device-token" type="password" autocomplete="off" bind:value={token} required minlength="64" maxlength="64" />
    <button type="submit" disabled={pending}>Connect</button>
  </form>
</main>
