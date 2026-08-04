<script lang="ts">
  /**
   * Root component: view switching (dialer, active call, history, pairing,
   * settings), connection status header, and the emergency-notice surface
   * required by ADR-0008 UX copy.
   */

  import StatusBadge from './components/StatusBadge.svelte';
  import ActiveCallView from './views/ActiveCallView.svelte';
  import DialerView from './views/DialerView.svelte';
  import HistoryView from './views/HistoryView.svelte';
  import PairingView from './views/PairingView.svelte';
  import SettingsView from './views/SettingsView.svelte';
  import { ipc } from '$lib/ipc';
  import { applyEvent, applyStatus, connection, primaryCall, revocation } from '$lib/state';

  type Tab = 'dialer' | 'history' | 'pairing' | 'settings';

  let tab = $state<Tab>('dialer');
  let startupError = $state<string | null>(null);

  $effect(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        applyStatus(await ipc.status());
        unlisten = await ipc.onEvent(applyEvent);
      } catch (error) {
        startupError = error instanceof Error ? error.message : 'Cannot reach the Tandem daemon';
      }
    })();

    return () => unlisten?.();
  });

  // A live call takes over the main pane regardless of the selected tab.
  const showingCall = $derived($primaryCall !== null);
</script>

<header>
  <h1>Tandem</h1>
  <StatusBadge status={$connection} />
</header>

{#if startupError}
  <p class="banner error" role="alert">{startupError}</p>
{/if}

{#if $revocation}
  <p class="banner error" role="alert">
    This computer was unpaired from the phone: {$revocation}
  </p>
{/if}

<nav>
  {#each [['dialer', 'Dialer'], ['history', 'History'], ['pairing', 'Pairing'], ['settings', 'Settings']] as [id, label] (id)}
    <button
      type="button"
      class:selected={tab === id && !showingCall}
      aria-current={tab === id && !showingCall}
      onclick={() => (tab = id as Tab)}
    >
      {label}
    </button>
  {/each}
</nav>

<main>
  {#if showingCall}
    <ActiveCallView />
  {:else if tab === 'dialer'}
    <DialerView />
  {:else if tab === 'history'}
    <HistoryView />
  {:else if tab === 'pairing'}
    <PairingView />
  {:else}
    <SettingsView />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: system-ui, sans-serif;
    color: #1c1b1f;
    background: #fbfbfd;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e5e5ea;
  }

  h1 {
    margin: 0;
    font-size: 1rem;
    letter-spacing: 0.02em;
  }

  nav {
    display: flex;
    gap: 0.25rem;
    padding: 0.5rem 1rem 0;
  }

  nav button {
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    padding: 0.375rem 0.5rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.875rem;
    opacity: 0.7;
  }

  nav button.selected {
    border-bottom-color: #1c1b1f;
    opacity: 1;
  }

  main {
    padding: 1rem;
  }

  .banner {
    margin: 0;
    padding: 0.625rem 1rem;
    font-size: 0.875rem;
  }

  .error {
    background: #fce8e6;
    color: #8c1d18;
  }
</style>
