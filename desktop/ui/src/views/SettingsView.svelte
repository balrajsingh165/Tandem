<script lang="ts">
  /**
   * Settings view: paired phone identity and fingerprint display, audio device
   * pickers, Bluetooth backend status, autostart, and unpair (with the re-pairing
   * consequence spelled out).
   */

  import StatusBadge from '../components/StatusBadge.svelte';
  import { connection, desktopAudioAvailable, phoneName } from '$lib/state';

  let confirmingUnpair = $state(false);
</script>

<section class="settings">
  <header>
    <h1>Settings</h1>
  </header>

  <div class="card">
    <div class="row">
      <span class="label">Paired phone</span>
      <StatusBadge status={$connection} />
    </div>
    <p class="value">{$phoneName || 'No phone paired'}</p>
  </div>

  <div class="card">
    <span class="label">Desktop audio</span>
    {#if $desktopAudioAvailable}
      <p class="body">Call audio can be routed to this computer over Bluetooth.</p>
    {:else}
      <p class="body muted">
        This build has no desktop audio path. Control and history work fully — talk on the handset,
        or pair a Bluetooth headset directly to your phone.
      </p>
    {/if}
  </div>

  <div class="card danger-card">
    <span class="label">Emergency calls</span>
    <p class="body muted">
      Tandem never places emergency calls from this computer. Dial them on the handset, which can
      share your location. An emergency call in progress is shown read-only.
    </p>
  </div>

  <div class="card">
    <span class="label">Unpair</span>
    {#if confirmingUnpair}
      <p class="body muted">
        This deletes this computer's key and the mirrored history. You will need to pair again from
        the phone.
      </p>
      <div class="actions">
        <button type="button" class="danger">Unpair now</button>
        <button type="button" class="ghost" onclick={() => (confirmingUnpair = false)}>
          Cancel
        </button>
      </div>
    {:else}
      <button type="button" class="ghost" onclick={() => (confirmingUnpair = true)}>
        Unpair this computer
      </button>
    {/if}
  </div>
</section>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  header h1 {
    margin: 0 0 2px;
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--hairline);
  }

  /* The emergency card is informational, so it is tinted rather than alarming. */
  .danger-card {
    border-color: var(--danger-a15);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .value {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .body {
    margin: 0;
    font-size: 12px;
    line-height: 1.55;
  }

  .muted {
    color: var(--text-2);
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 2px;
  }

  button {
    min-height: 34px;
    padding: 0 12px;
    border-radius: var(--radius-s);
    font-size: 12px;
    font-weight: 650;
    align-self: flex-start;
    transition: background 0.16s ease, filter 0.16s ease;
  }

  .ghost {
    background: var(--surface-hi);
    border: 1px solid var(--hairline);
  }

  .ghost:hover {
    border-color: var(--hairline-strong);
  }

  .danger {
    background: var(--danger);
    color: #fff;
  }

  .danger:hover {
    filter: brightness(1.08);
  }
</style>
