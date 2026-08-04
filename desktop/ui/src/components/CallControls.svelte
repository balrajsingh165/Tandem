<script lang="ts">
  /**
   * Reusable call-control cluster (mute, hold, merge, end) rendering
   * capability-gated buttons from the mirrored call state; emits intents upward,
   * never calls IPC itself.
   */

  import type { CallView } from '$lib/ipc';

  interface Props {
    call: CallView;
    muted: boolean;
    onmute: (muted: boolean) => void;
    onhold: (hold: boolean) => void;
    onmerge: () => void;
    onend: () => void;
  }

  const { call, muted, onmute, onhold, onmerge, onend }: Props = $props();

  // An emergency call is surfaced read-only: it is controllable only on the
  // handset (ADR-0008).
  const locked = $derived(call.isEmergency);
  const held = $derived(call.state === 'holding');
</script>

<div class="controls" role="group" aria-label="Call controls">
  <button type="button" disabled={locked} aria-pressed={muted} onclick={() => onmute(!muted)}>
    {muted ? 'Unmute' : 'Mute'}
  </button>

  <button
    type="button"
    disabled={locked || !call.canHold}
    aria-pressed={held}
    onclick={() => onhold(!held)}
  >
    {held ? 'Resume' : 'Hold'}
  </button>

  <button type="button" disabled={locked || !call.canMerge} onclick={onmerge}>Merge</button>

  <button type="button" class="end" disabled={locked} onclick={onend}>End</button>
</div>

{#if locked}
  <p class="notice">
    Emergency calls are controlled on the handset only. Tandem shows this call read-only.
  </p>
{/if}

<style>
  .controls {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  button {
    flex: 1 1 5rem;
    min-height: 2.5rem;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
    background: var(--surface, #fff);
    cursor: pointer;
    font: inherit;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .end {
    border-color: #b3261e;
    color: #b3261e;
  }

  .notice {
    margin: 0.5rem 0 0;
    font-size: 0.8125rem;
    opacity: 0.8;
  }
</style>
