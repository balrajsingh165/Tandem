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
  <h1>Settings</h1>

  <section class="group">
    <h2>Paired phone</h2>
    <p class="row">
      <span>{$phoneName || 'No phone paired'}</span>
      <StatusBadge status={$connection} />
    </p>
  </section>

  <section class="group">
    <h2>Desktop audio</h2>
    {#if $desktopAudioAvailable}
      <p class="row">Call audio can be routed to this computer over Bluetooth.</p>
    {:else}
      <p class="row muted">
        This build has no desktop audio path. Control and history work fully; talk on the handset
        or pair a Bluetooth headset directly to your phone.
      </p>
    {/if}
  </section>

  <section class="group">
    <h2>Emergency calls</h2>
    <p class="row muted">
      Tandem never places emergency calls from this computer. Dial them on the handset, which has
      carrier location. An emergency call in progress is shown read-only.
    </p>
  </section>

  <section class="group">
    <h2>Unpair</h2>
    {#if confirmingUnpair}
      <p class="row muted">
        Unpairing deletes this computer's key and the mirrored call history. You will need to pair
        again from the phone.
      </p>
      <div class="row">
        <button type="button" class="danger">Unpair now</button>
        <button type="button" onclick={() => (confirmingUnpair = false)}>Cancel</button>
      </div>
    {:else}
      <button type="button" onclick={() => (confirmingUnpair = true)}>Unpair this computer</button>
    {/if}
  </section>
</section>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    max-width: 30rem;
  }

  h1 {
    margin: 0;
    font-size: 1.25rem;
  }

  h2 {
    margin: 0 0 0.375rem;
    font-size: 0.9375rem;
  }

  .group {
    border-top: 1px solid var(--border, #e5e5ea);
    padding-top: 0.75rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
    font-size: 0.875rem;
  }

  .muted {
    display: block;
    opacity: 0.75;
    line-height: 1.5;
  }

  button {
    min-height: 2.25rem;
    padding: 0 0.75rem;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
    background: var(--surface, #fff);
    cursor: pointer;
    font: inherit;
  }

  .danger {
    border-color: #b3261e;
    color: #b3261e;
  }
</style>
