<script lang="ts">
  /**
   * Dialer view: number entry via DialPad, recent-call shortcuts, and dial
   * dispatch. Shows the emergency-block explanation when core/emergency refuses
   * a number (ADR-0008).
   */

  import DialPad from '../components/DialPad.svelte';
  import { ipc, IpcCallError, IPC_EMERGENCY_BLOCKED } from '$lib/ipc';
  import { emergencyNotice, history, isConnected } from '$lib/state';
  import { formatNumber } from '$lib/format';

  let entry = $state('');
  let failure = $state<string | null>(null);

  const recent = $derived($history.slice(0, 5));

  function append(digit: string): void {
    entry += digit;
    failure = null;
  }

  function backspace(): void {
    entry = entry.slice(0, -1);
  }

  async function dial(): Promise<void> {
    if (!entry) return;
    failure = null;
    emergencyNotice.set(null);
    try {
      await ipc.dial(entry);
      entry = '';
    } catch (error) {
      if (error instanceof IpcCallError && error.code === IPC_EMERGENCY_BLOCKED) {
        emergencyNotice.set({
          number: entry,
          guidance: 'Dial this number on your phone. Emergency calls need the handset for location.',
        });
        return;
      }
      failure = error instanceof Error ? error.message : 'Could not place the call';
    }
  }
</script>

<section class="dialer">
  <label class="entry">
    <span class="sr-only">Number to dial</span>
    <input
      type="tel"
      bind:value={entry}
      placeholder="Enter a number"
      autocomplete="off"
      onkeydown={(e) => e.key === 'Enter' && dial()}
    />
  </label>

  <DialPad disabled={!$isConnected} ondigit={append} />

  <div class="actions">
    <button type="button" onclick={backspace} disabled={!entry}>Delete</button>
    <button type="button" class="call" onclick={dial} disabled={!entry || !$isConnected}>
      Call
    </button>
  </div>

  {#if $emergencyNotice}
    <p class="emergency" role="alert">
      <strong>{$emergencyNotice.number} is an emergency number.</strong>
      {$emergencyNotice.guidance}
    </p>
  {/if}

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#if recent.length > 0}
    <h2>Recent</h2>
    <ul class="recent">
      {#each recent as item (item.entryId)}
        <li>
          <button type="button" onclick={() => (entry = item.number)}>
            {item.displayName || formatNumber(item.number)}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .dialer {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-width: 20rem;
  }

  input {
    width: 100%;
    padding: 0.625rem;
    font-size: 1.25rem;
    text-align: center;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .actions button {
    flex: 1;
    min-height: 2.75rem;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
    background: var(--surface, #fff);
    cursor: pointer;
    font: inherit;
  }

  .actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .call {
    background: #1b6e3c;
    color: #fff;
    border-color: #1b6e3c;
  }

  .emergency {
    padding: 0.625rem;
    border: 1px solid #b3261e;
    border-radius: 0.5rem;
    font-size: 0.875rem;
  }

  .failure {
    color: #b3261e;
    font-size: 0.875rem;
  }

  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .recent button {
    width: 100%;
    text-align: left;
    padding: 0.5rem;
    border: 0;
    background: none;
    cursor: pointer;
    font: inherit;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
