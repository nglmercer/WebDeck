<script lang="ts">
  import { approveDevice, listDevices, revokeDevice } from '../../api/v2';
  import type { Capability, Device } from '../../contracts/v2';
  let devices = $state<Device[]>([]); let token = $state(''); let name = $state('');
  let capabilities = $state<Capability[]>(['read', 'input', 'audio']);
  let error = $state(''); let pending = $state(false); let loaded = $state(false);
  const choices: Capability[] = ['read','input','audio','window','power','script','network','plugin','settings'];
  async function refresh(): Promise<void> {
    try { devices = (await listDevices()).devices; loaded = true; error = ''; }
    catch { error = 'Device approval is available only on the host computer using its loopback address.'; }
  }
  async function approve(): Promise<void> {
    if (pending) return; pending = true;
    try { token = (await approveDevice({name, capabilities, ttl_seconds: 3600})).token; await refresh(); }
    catch (e) { error = e instanceof Error ? e.message : 'Approval failed'; }
    finally { pending = false; }
  }
  async function revoke(id: string): Promise<void> {
    if (pending) return; pending = true;
    try { await revokeDevice(id); token = ''; await refresh(); }
    catch (e) { error = e instanceof Error ? e.message : 'Revocation failed'; }
    finally { pending = false; }
  }
</script>
<section class="wd2-panel" aria-label="Device access">
  <h2>Device access</h2>
  <p>Remote access requires a paired device for HTTP and sockets. Approve devices on the host via a loopback address. Tokens expire in one hour; reconnects require a valid token.</p>
  <button type="button" onclick={refresh}>Manage devices</button>
  {#if loaded}
    <label for="device-name">Device name</label><input id="device-name" bind:value={name} maxlength="128" />
    <fieldset><legend>Permissions</legend>{#each choices as capability}<label><input type="checkbox" bind:group={capabilities} value={capability} />{capability}</label>{/each}</fieldset>
    <button type="button" disabled={pending || !name.trim()} onclick={approve}>Approve for one hour</button>
    {#if token}<label for="approved-token">Share this token privately with the approved device</label><input id="approved-token" type="password" readonly value={token} /><button type="button" onclick={() => token = ''}>Dismiss token</button>{/if}
    <ul>{#each devices as device}<li>{device.name} — {device.revoked ? 'revoked' : new Date(device.expires_at * 1000).toLocaleString()}<button type="button" disabled={pending || device.revoked} onclick={() => revoke(device.id)}>Revoke</button></li>{/each}</ul>
  {/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>
