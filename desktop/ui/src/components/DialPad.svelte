<script lang="ts">
  /**
   * Reusable 12-key dial pad emitting digit events; used by DialerView for
   * dialing and ActiveCallView for DTMF. Presentation only.
   */

  interface Props {
    disabled?: boolean;
    ondigit: (digit: string) => void;
  }

  const { disabled = false, ondigit }: Props = $props();

  const keys = [
    ['1', ''],
    ['2', 'ABC'],
    ['3', 'DEF'],
    ['4', 'GHI'],
    ['5', 'JKL'],
    ['6', 'MNO'],
    ['7', 'PQRS'],
    ['8', 'TUV'],
    ['9', 'WXYZ'],
    ['*', ''],
    ['0', '+'],
    ['#', ''],
  ];
</script>

<div class="dialpad" role="group" aria-label="Dial pad">
  {#each keys as [digit, letters] (digit)}
    <button type="button" {disabled} onclick={() => ondigit(digit)} aria-label={digit}>
      <span class="digit">{digit}</span>
      {#if letters}<span class="letters">{letters}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .dialpad {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
  }

  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 3.25rem;
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

  .digit {
    font-size: 1.25rem;
  }

  .letters {
    font-size: 0.625rem;
    letter-spacing: 0.08em;
    opacity: 0.6;
  }
</style>
