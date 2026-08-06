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
  import { nameFor, suggest } from '$lib/contacts';

  let entry = $state('');
  let failure = $state<string | null>(null);
  let shaking = $state(false);

  // The saved name for exactly this number, and near-matches while typing.
  const matchedName = $derived(entry ? nameFor(entry, $history) : null);
  const suggestions = $derived(entry ? suggest(entry, $history) : []);

  function append(digit: string): void {
    entry += digit;
    failure = null;
    emergencyNotice.set(null);
  }

  function backspace(): void {
    entry = entry.slice(0, -1);
  }

  function reject(): void {
    shaking = true;
    setTimeout(() => (shaking = false), 420);
  }

  async function dial(): Promise<void> {
    if (!entry || !$isConnected) return;
    failure = null;
    emergencyNotice.set(null);
    try {
      await ipc.dial(entry);
      entry = '';
    } catch (error) {
      if (error instanceof IpcCallError && error.code === IPC_EMERGENCY_BLOCKED) {
        emergencyNotice.set({
          number: entry,
          guidance: 'Dial this on your phone — emergency services need the handset for location.',
        });
        reject();
        return;
      }
      failure = error instanceof Error ? error.message : 'Could not place the call';
      reject();
    }
  }
</script>

<section class="dialer">
  <div class="readout" class:shake={shaking}>
    {#if matchedName}
      <p class="who">{matchedName}</p>
    {/if}
    <input
      class="numeric"
      type="tel"
      bind:value={entry}
      placeholder="Enter a number"
      autocomplete="off"
      aria-label="Number to dial"
      onkeydown={(e) => e.key === 'Enter' && dial()}
    />
    {#if entry && !matchedName}
      <p class="hint">{formatNumber(entry)}</p>
    {/if}
  </div>

  {#if suggestions.length > 0 && !matchedName}
    <ul class="suggestions rise">
      {#each suggestions as contact (contact.number)}
        <li>
          <button type="button" onclick={() => (entry = contact.number)}>
            <span class="avatar" aria-hidden="true">{contact.name.charAt(0).toUpperCase()}</span>
            <span class="meta">
              <span class="cname">{contact.name}</span>
              <span class="cnum numeric">{formatNumber(contact.number)}</span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if $emergencyNotice}
    <p class="emergency" role="alert">
      <strong>{$emergencyNotice.number} is an emergency number.</strong>
      {$emergencyNotice.guidance}
    </p>
  {/if}

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  <DialPad
    disabled={!$isConnected}
    ondigit={append}
    onbackspace={backspace}
    onsubmit={dial}
  />

  <div class="actions">
    <button type="button" class="ghost" onclick={backspace} disabled={!entry} aria-label="Delete">
      ⌫
    </button>
    <button
      type="button"
      class="call"
      onclick={dial}
      disabled={!entry || !$isConnected}
      aria-label="Call"
    >
      <span aria-hidden="true">📞</span> Call
    </button>
  </div>

  {#if !$isConnected}
    <p class="offline">Pair a phone to place calls.</p>
  {/if}
</section>

<style>
  .dialer {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .readout {
    text-align: center;
    padding: 6px 0 2px;
  }

  .readout.shake {
    animation: shake 0.42s cubic-bezier(0.36, 0.07, 0.19, 0.97);
  }

  .who {
    margin: 0 0 2px;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 650;
    color: var(--accent);
  }

  input {
    width: 100%;
    border: 0;
    background: none;
    text-align: center;
    font-size: 30px;
    font-weight: 500;
    letter-spacing: 0.01em;
    padding: 2px 0;
  }

  input::placeholder {
    color: var(--text-3);
    font-size: 17px;
    font-weight: 400;
    letter-spacing: 0;
  }

  input:focus {
    outline: none;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-3);
  }

  .suggestions {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 132px;
    overflow-y: auto;
  }

  .suggestions button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 9px;
    border-radius: var(--radius-s);
    text-align: left;
    transition: background 0.16s ease;
  }

  .suggestions button:hover {
    background: var(--surface);
  }

  .avatar {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 50%;
    background: var(--accent-a20);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .cname {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cnum {
    font-size: 11px;
    color: var(--text-3);
  }

  .actions {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 10px;
  }

  .ghost {
    min-height: 46px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--hairline);
    font-size: 16px;
    transition: background 0.16s ease, transform 0.16s var(--ease-spring);
  }

  .ghost:hover:not(:disabled) {
    background: var(--surface-hi);
  }

  .ghost:active:not(:disabled) {
    transform: scale(0.96);
  }

  .call {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 46px;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
    box-shadow: var(--glow);
    transition: transform 0.16s var(--ease-spring), filter 0.16s ease;
  }

  .call:hover:not(:disabled) {
    filter: brightness(1.06);
  }

  .call:active:not(:disabled) {
    transform: scale(0.98);
  }

  .ghost:disabled,
  .call:disabled {
    opacity: 0.35;
    cursor: not-allowed;
    box-shadow: none;
  }

  .emergency {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    border: 1px solid var(--danger);
    background: var(--danger-a15);
    color: var(--danger);
    font-size: 12px;
    line-height: 1.5;
  }

  .failure {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
    text-align: center;
  }

  .offline {
    margin: 0;
    text-align: center;
    font-size: 12px;
    color: var(--text-3);
  }
</style>
