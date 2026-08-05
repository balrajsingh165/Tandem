<script lang="ts">
  /**
   * Pairing view: QR-scan instructions and manual entry path, live pairing
   * progress, the 6-digit short-code comparison step, and success/failure
   * outcomes.
   */

  import { ipc } from '$lib/ipc';
  import { pairingState } from '$lib/state';

  let payload = $state('');
  let submitting = $state(false);
  let failure = $state<string | null>(null);

  const shortCode = $derived($pairingState?.shortCode ?? null);
  const phase = $derived($pairingState?.state ?? null);
  const accepted = $derived(phase === 'accepted');

  async function submit(): Promise<void> {
    if (!payload.trim()) return;
    submitting = true;
    failure = null;
    try {
      await ipc.pairing(payload.trim());
    } catch (error) {
      failure = error instanceof Error ? error.message : 'Pairing failed';
    } finally {
      submitting = false;
    }
  }
</script>

<section class="pairing">
  <header>
    <h1>Pair with your phone</h1>
    <p class="label">One-time setup</p>
  </header>

  <ol class="steps">
    <li>Open <strong>Tandem</strong> on your phone.</li>
    <li>Tap <strong>Pair a computer</strong>.</li>
    <li>Paste the code it shows below, then confirm on the phone.</li>
  </ol>

  <label class="field">
    <span class="label">Pairing code</span>
    <textarea
      bind:value={payload}
      rows="3"
      spellcheck="false"
      placeholder={'{"v":1,"host":"192.168.…"}'}
    ></textarea>
  </label>

  <button type="button" class="primary" onclick={submit} disabled={submitting || !payload.trim()}>
    {submitting ? 'Pairing…' : 'Pair'}
  </button>

  {#if shortCode}
    <div class="code rise" role="status">
      <p class="label">Confirm this matches your phone</p>
      <p class="digits numeric">{shortCode}</p>
      <p class="warn">If the codes differ, stop — do not confirm on the phone.</p>
    </div>
  {:else if accepted}
    <p class="ok rise" role="status">Paired. Your phone is connected.</p>
  {:else if phase}
    <p class="progress" role="status">{phase}</p>
  {/if}

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}
</section>

<style>
  .pairing {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  header h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  header .label {
    margin: 3px 0 0;
  }

  .steps {
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
    line-height: 1.7;
    color: var(--text-2);
  }

  .steps strong {
    color: var(--text);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  textarea {
    width: 100%;
    resize: vertical;
    padding: 9px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--hairline);
    background: var(--surface);
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.5;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent-a35);
  }

  .primary {
    min-height: 42px;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
    transition: transform 0.16s var(--ease-spring), filter 0.16s ease;
  }

  .primary:hover:not(:disabled) {
    filter: brightness(1.06);
  }

  .primary:active:not(:disabled) {
    transform: scale(0.98);
  }

  .primary:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .code {
    text-align: center;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--accent-a35);
  }

  .digits {
    margin: 6px 0;
    font-size: 32px;
    font-weight: 600;
    letter-spacing: 0.28em;
    text-indent: 0.28em;
    color: var(--accent);
  }

  .warn {
    margin: 0;
    font-size: 11px;
    color: var(--danger);
    font-weight: 600;
  }

  .ok {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--accent-a20);
    color: var(--accent);
    font-size: 12px;
    font-weight: 650;
    text-align: center;
  }

  .progress {
    margin: 0;
    font-size: 12px;
    color: var(--text-2);
    text-align: center;
  }

  .failure {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
