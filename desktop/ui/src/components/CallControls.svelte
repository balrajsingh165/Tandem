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
    canMerge: boolean;
    onmute: (muted: boolean) => void;
    onhold: (hold: boolean) => void;
    onmerge: () => void;
    onend: () => void;
  }

  const { call, muted, canMerge, onmute, onhold, onmerge, onend }: Props = $props();

  // An emergency call is surfaced read-only: it is controllable only on the
  // handset (ADR-0008).
  const locked = $derived(call.isEmergency);
  const held = $derived(call.state === 'holding');
</script>

<div class="grid" role="group" aria-label="Call controls">
  <button
    type="button"
    class="tile"
    class:on={muted}
    disabled={locked}
    aria-pressed={muted}
    onclick={() => onmute(!muted)}
  >
    <span class="glyph" aria-hidden="true">{muted ? '🔇' : '🎙'}</span>
    <span class="cap">{muted ? 'Unmute' : 'Mute'}</span>
  </button>

  <button
    type="button"
    class="tile"
    class:on={held}
    disabled={locked || !call.canHold}
    aria-pressed={held}
    onclick={() => onhold(!held)}
  >
    <span class="glyph" aria-hidden="true">{held ? '▶' : '⏸'}</span>
    <span class="cap">{held ? 'Resume' : 'Hold'}</span>
  </button>

  <button
    type="button"
    class="tile"
    disabled={locked || !call.canMerge || !canMerge}
    onclick={onmerge}
  >
    <span class="glyph" aria-hidden="true">⇄</span>
    <span class="cap">Merge</span>
  </button>
</div>

<button type="button" class="end" disabled={locked} onclick={onend}>
  <span class="glyph" aria-hidden="true">✕</span>
  End call
</button>

{#if locked}
  <p class="notice" role="note">
    Emergency call — controlled on the handset only.
  </p>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 12px 6px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--hairline);
    transition:
      transform 0.16s var(--ease-spring),
      background 0.16s ease,
      border-color 0.16s ease,
      color 0.16s ease;
  }

  .tile:hover:not(:disabled) {
    background: var(--surface-hi);
    border-color: var(--hairline-strong);
  }

  .tile:active:not(:disabled) {
    transform: scale(0.95);
  }

  /* An engaged toggle has to be obvious at a glance mid-call. */
  .tile.on:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent-a35);
    background: var(--accent-a20);
  }

  .tile:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .glyph {
    font-size: 16px;
    line-height: 1;
  }

  .cap {
    font-size: 11px;
    font-weight: 600;
  }

  .end {
    width: 100%;
    margin-top: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 46px;
    border-radius: var(--radius);
    background: var(--danger);
    color: #fff;
    font-weight: 650;
    letter-spacing: 0.01em;
    box-shadow: var(--shadow-2);
    transition: transform 0.16s var(--ease-spring), filter 0.16s ease;
  }

  .end:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .end:active:not(:disabled) {
    transform: scale(0.98);
  }

  .end:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    box-shadow: none;
  }

  .notice {
    margin: 10px 0 0;
    padding: 9px 11px;
    border-radius: var(--radius-s);
    background: var(--danger-a15);
    color: var(--danger);
    font-size: 12px;
    font-weight: 600;
  }
</style>
