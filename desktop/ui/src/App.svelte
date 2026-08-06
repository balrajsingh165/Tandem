<script lang="ts">
  /**
   * Root component: view switching (dialer, active call, history, pairing,
   * settings), connection status header, and the emergency-notice surface
   * required by ADR-0008 UX copy.
   */

  import PhoneSwitcher from './components/PhoneSwitcher.svelte';
  import StatusBadge from './components/StatusBadge.svelte';
  import ActiveCallView from './views/ActiveCallView.svelte';
  import ContactsView from './views/ContactsView.svelte';
  import DialerView from './views/DialerView.svelte';
  import HistoryView from './views/HistoryView.svelte';
  import PairingView from './views/PairingView.svelte';
  import SettingsView from './views/SettingsView.svelte';
  import { ipc } from '$lib/ipc';
  import {
    applyEvent,
    applyStatus,
    connection,
    loadContacts,
    loadHistory,
    primaryCall,
    revocation,
  } from '$lib/state';

  type Tab = 'dialer' | 'contacts' | 'history' | 'pairing' | 'settings';

  const tabs: Array<[Tab, string, string]> = [
    ['dialer', 'Dialer', '⌨'],
    ['contacts', 'Contacts', '☰'],
    ['history', 'Recents', '↺'],
    ['pairing', 'Pair', '⇋'],
    ['settings', 'Settings', '⚙'],
  ];

  let tab = $state<Tab>('dialer');
  let startupError = $state<string | null>(null);

  $effect(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        applyStatus(await ipc.status());
        void loadHistory(ipc.history).catch(() => {});
        void loadContacts(ipc.contacts);

        unlisten = await ipc.onEvent((event) => {
          applyEvent(event);
          // Recents and every name lookup read one cache; refresh it when the
          // phone says the log moved.
          if (event.type === 'historyChanged') {
            void loadHistory(ipc.history).catch(() => {});
          }
          if (event.type === 'contactsChanged') {
            void loadContacts(ipc.contacts);
          }
        });
      } catch (error) {
        startupError = error instanceof Error ? error.message : 'Cannot reach the Tandem daemon';
      }
    })();

    return () => unlisten?.();
  });

  // A live call takes over the main pane regardless of the selected tab.
  const showingCall = $derived($primaryCall !== null);
</script>

<div class="shell">
  <header>
    <div class="brand">
      <span class="mark" aria-hidden="true"></span>
      <span class="name">Tandem</span>
    </div>
    <div class="right">
      <PhoneSwitcher />
      <StatusBadge status={$connection} />
    </div>
  </header>

  {#if startupError}
    <p class="banner error" role="alert">{startupError}</p>
  {/if}

  {#if $revocation}
    <p class="banner error" role="alert">Unpaired from the phone: {$revocation}</p>
  {/if}

  <main class:calling={showingCall}>
    {#if showingCall}
      <ActiveCallView />
    {:else if tab === 'dialer'}
      <DialerView />
    {:else if tab === 'contacts'}
      <ContactsView />
    {:else if tab === 'history'}
      <HistoryView />
    {:else if tab === 'pairing'}
      <PairingView />
    {:else}
      <SettingsView />
    {/if}
  </main>

  {#if !showingCall}
    <nav aria-label="Sections">
      {#each tabs as [id, label, glyph] (id)}
        <button
          type="button"
          class:selected={tab === id}
          aria-current={tab === id ? 'page' : undefined}
          onclick={() => (tab = id)}
        >
          <span class="glyph" aria-hidden="true">{glyph}</span>
          <span class="cap">{label}</span>
        </button>
      {/each}
    </nav>
  {/if}
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 14px 16px 10px;
    flex: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* A small solid mark anchors the header without needing an image asset. */
  .mark {
    width: 16px;
    height: 16px;
    border-radius: 5px;
    background: var(--accent);
    box-shadow: 0 0 12px var(--accent-a35);
  }

  .name {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  /* The panel is a phone companion, not a page: past a comfortable reading
     width the content is centred rather than stretched across the monitor. */
  main {
    flex: 1;
    overflow-y: auto;
    padding: 4px 16px 16px;
    width: 100%;
    max-width: 460px;
    margin-inline: auto;
    box-sizing: border-box;
  }

  main.calling {
    padding-bottom: 20px;
  }

  nav {
    flex: none;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 2px;
    padding: 6px 8px 10px;
    border-top: 1px solid var(--hairline);
    background: var(--bg-void);
  }

  nav button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 7px 2px;
    border-radius: var(--radius-s);
    color: var(--text-3);
    transition: color 0.16s ease, background 0.16s ease;
  }

  nav button:hover {
    color: var(--text-2);
    background: var(--surface);
  }

  nav button.selected {
    color: var(--accent);
  }

  nav .glyph {
    font-size: 15px;
    line-height: 1;
  }

  nav .cap {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .banner {
    flex: none;
    margin: 0 16px 8px;
    padding: 9px 11px;
    border-radius: var(--radius-s);
    font-size: 12px;
    font-weight: 600;
  }

  .error {
    background: var(--danger-a15);
    color: var(--danger);
  }
</style>
