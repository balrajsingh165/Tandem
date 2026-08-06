<script lang="ts">
  /**
   * Reusable 12-key dial pad emitting digit events; used by DialerView for
   * dialing and ActiveCallView for DTMF. Presentation only.
   */

  interface Props {
    disabled?: boolean;
    compact?: boolean;
    ondigit: (digit: string) => void;
    /** Physical Backspace; omitted where deleting makes no sense (DTMF). */
    onbackspace?: () => void;
    /** Physical Enter. */
    onsubmit?: () => void;
  }

  const {
    disabled = false,
    compact = false,
    ondigit,
    onbackspace,
    onsubmit,
  }: Props = $props();

  const keys: Array<[string, string]> = [
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

  // Physical keyboard should drive the pad too; this is a desktop app.
  function onKeydown(event: KeyboardEvent) {
    if (disabled) return;
    // Typing into a field must not also drive the pad.
    const target = event.target as HTMLElement | null;
    if (target && /^(INPUT|TEXTAREA)$/.test(target.tagName)) return;

    const key = event.key;
    if (/^[0-9*#+]$/.test(key)) {
      ondigit(key);
      pulse(key);
      return;
    }
    if (key === 'Backspace' && onbackspace) {
      event.preventDefault();
      onbackspace();
      pulse('⌫');
      return;
    }
    if (key === 'Enter' && onsubmit) {
      event.preventDefault();
      onsubmit();
    }
  }

  let pressed = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function pulse(key: string) {
    pressed = key;
    clearTimeout(timer);
    timer = setTimeout(() => (pressed = null), 160);
  }

  function press(key: string) {
    ondigit(key);
    pulse(key);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="pad" class:compact role="group" aria-label="Dial pad">
  {#each keys as [digit, letters] (digit)}
    <button
      type="button"
      {disabled}
      class:active={pressed === digit}
      onclick={() => press(digit)}
      aria-label={digit}
    >
      <span class="digit numeric">{digit}</span>
      {#if letters}<span class="letters">{letters}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .pad {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  button {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    aspect-ratio: 1.5;
    border-radius: var(--radius-l);
    background: var(--surface);
    border: 1px solid var(--hairline);
    box-shadow: var(--shadow-1);
    overflow: hidden;
    transition:
      transform 0.16s var(--ease-spring),
      background 0.16s ease,
      border-color 0.16s ease;
  }

  /* A hairline of light along the top edge reads as a moulded key. */
  button::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--hairline-strong), transparent);
  }

  button:hover:not(:disabled) {
    background: var(--surface-hi);
    border-color: var(--hairline-strong);
  }

  button:active:not(:disabled),
  button.active:not(:disabled) {
    transform: scale(0.94);
    background: var(--surface-hi);
    border-color: var(--accent-a35);
  }

  button:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .digit {
    font-size: 22px;
    font-weight: 500;
    line-height: 1;
  }

  .compact .digit {
    font-size: 18px;
  }

  .compact button {
    aspect-ratio: 1.9;
    border-radius: var(--radius);
  }

  .letters {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.12em;
    color: var(--text-3);
  }

  .compact .letters {
    display: none;
  }
</style>
