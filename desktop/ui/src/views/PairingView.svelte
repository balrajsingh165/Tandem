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
  <h1>Pair with your phone</h1>

  <ol class="steps">
    <li>Open Tandem Gateway on your phone and choose <strong>Pair a computer</strong>.</li>
    <li>Scan the QR code shown there, or paste its contents below.</li>
    <li>Confirm the pairing prompt on the phone.</li>
  </ol>

  <label>
    <span>Pairing code contents</span>
    <textarea bind:value={payload} rows="4" placeholder={'{"v":1,"host":…}'}></textarea>
  </label>

  <button type="button" onclick={submit} disabled={submitting || !payload.trim()}>
    {submitting ? 'Pairing…' : 'Pair'}
  </button>

  {#if shortCode}
    <div class="shortcode" role="status">
      <p>Confirm this code matches the one on your phone:</p>
      <p class="code">{shortCode}</p>
      <p class="warn">If the codes differ, stop — do not confirm on the phone.</p>
    </div>
  {:else if $pairingState}
    <p class="progress" role="status">{$pairingState.state}</p>
  {/if}

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}
</section>

<style>
  .pairing {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-width: 26rem;
  }

  h1 {
    margin: 0;
    font-size: 1.25rem;
  }

  .steps {
    margin: 0;
    padding-left: 1.25rem;
    font-size: 0.875rem;
    line-height: 1.6;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.875rem;
  }

  textarea {
    font-family: ui-monospace, monospace;
    font-size: 0.8125rem;
    padding: 0.5rem;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
    resize: vertical;
  }

  button {
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

  .shortcode {
    padding: 0.75rem;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
    text-align: center;
  }

  .code {
    margin: 0.25rem 0;
    font-size: 2rem;
    letter-spacing: 0.35em;
    font-variant-numeric: tabular-nums;
  }

  .warn {
    margin: 0;
    font-size: 0.8125rem;
    color: #b3261e;
  }

  .progress {
    font-size: 0.875rem;
    opacity: 0.8;
  }

  .failure {
    color: #b3261e;
    font-size: 0.875rem;
  }
</style>
