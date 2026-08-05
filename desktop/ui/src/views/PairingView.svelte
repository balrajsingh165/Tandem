<script lang="ts">
  /**
   * Pairing view: renders this desktop's one-time offer as a QR code for the
   * phone's camera, tracks live pairing progress, and offers the typed-code
   * fallback for when a camera is unavailable.
   */

  import QRCode from 'qrcode';
  import { ipc } from '$lib/ipc';
  import { pairingApproval, pairingState } from '$lib/state';

  let canvas = $state<HTMLCanvasElement | null>(null);
  let offer = $state<string | null>(null);
  let starting = $state(false);
  let failure = $state<string | null>(null);
  let manual = $state(false);
  let typed = $state('');

  const phase = $derived($pairingState?.state ?? null);
  const accepted = $derived(phase === 'accepted');
  const progress = $derived(describe(phase));

  function describe(state: string | null): string | null {
    if (!state) return null;
    if (state === 'waitingForScan') return 'Waiting for your phone to scan…';
    if (state.startsWith('retrying: ')) return `Waiting for your phone to scan… (${state.slice(10)})`;
    if (state === 'awaitingConfirmation') return 'Confirm on your phone to finish.';
    if (state === 'accepted') return 'Paired. Your phone is connected.';
    if (state.startsWith('connecting:')) return `Connecting to ${state.slice(11)}…`;
    if (state.startsWith('approve:')) return null;
    if (state.startsWith('failed: ')) return null;
    return state;
  }

  async function decide(accept: boolean): Promise<void> {
    try {
      await ipc.pairingConfirm(accept);
    } catch (error) {
      failure = error instanceof Error ? error.message : 'Could not send your answer';
    } finally {
      pairingApproval.set(null);
    }
  }

  const failedText = $derived(phase?.startsWith('failed: ') ? phase.slice(8) : null);

  async function start(): Promise<void> {
    starting = true;
    failure = null;
    try {
      const result = await ipc.pairingOffer();
      offer = result.payload;
    } catch (error) {
      failure = error instanceof Error ? error.message : 'Could not start pairing';
    } finally {
      starting = false;
    }
  }

  async function submitTyped(): Promise<void> {
    if (!typed.trim()) return;
    failure = null;
    try {
      await ipc.pairing(typed.trim());
    } catch (error) {
      failure = error instanceof Error ? error.message : 'Pairing failed';
    }
  }

  $effect(() => {
    const target = canvas;
    const text = offer;
    if (!target || !text) return;
    QRCode.toCanvas(target, text, {
      width: 216,
      margin: 1,
      errorCorrectionLevel: 'M',
      color: { dark: '#0b0b0d', light: '#ffffff' },
    }).catch(() => {
      failure = 'Could not draw the pairing code';
    });
  });
</script>

<section class="pairing">
  <header>
    <h1>Pair with your phone</h1>
    <p class="label">One-time setup</p>
  </header>

  {#if accepted}
    <p class="ok rise" role="status">Paired. Your phone is connected.</p>
  {:else if offer}
    <div class="stage rise">
      <div class="frame">
        <canvas bind:this={canvas} width="216" height="216" aria-label="Pairing QR code"></canvas>
        {#if !failedText}<span class="halo"></span>{/if}
      </div>
      <ol class="steps">
        <li>Open <strong>Tandem</strong> on your phone.</li>
        <li>Tap <strong>Scan a computer</strong>.</li>
        <li>Point the camera at this code, then confirm.</li>
      </ol>
    </div>

    {#if failedText}
      <p class="failure" role="alert">{failedText}</p>
      <button type="button" class="primary" onclick={start}>Show a new code</button>
    {:else if progress}
      <p class="progress" role="status"><span class="dot"></span>{progress}</p>
    {/if}
  {:else}
    <p class="intro">
      Tandem shows a code here; your phone scans it. Both devices must be on the same Wi-Fi.
    </p>
    <button type="button" class="primary" onclick={start} disabled={starting}>
      {starting ? 'Preparing…' : 'Show pairing code'}
    </button>
    {#if failure}<p class="failure" role="alert">{failure}</p>{/if}
  {/if}

  {#if !accepted}
    <button type="button" class="link" onclick={() => (manual = !manual)}>
      {manual ? 'Hide' : 'No camera on the phone?'}
    </button>
    {#if manual}
      <label class="field">
        <span class="label">Paste the code your phone shows</span>
        <textarea bind:value={typed} rows="3" spellcheck="false"></textarea>
      </label>
      <button type="button" class="ghost" onclick={submitTyped} disabled={!typed.trim()}>
        Pair from pasted code
      </button>
    {/if}
  {/if}
</section>

{#if $pairingApproval}
  <div class="scrim" role="dialog" aria-modal="true" aria-labelledby="approve-title">
    <div class="sheet rise">
      <p class="label">Pairing request</p>
      <h2 id="approve-title">{$pairingApproval.phoneName} scanned your code</h2>
      <p class="body">
        Allow this phone to place and control calls from this computer? Nothing has been shared
        with it yet.
      </p>
      <p class="fingerprint numeric">{$pairingApproval.phoneFingerprint}</p>
      <div class="actions">
        <button type="button" class="primary" onclick={() => decide(true)}>Allow</button>
        <button type="button" class="ghost" onclick={() => decide(false)}>Deny</button>
      </div>
    </div>
  </div>
{/if}

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

  .intro {
    margin: 0;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-2);
  }

  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    padding: 16px 12px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--hairline);
    max-width: 100%;
    box-sizing: border-box;
    overflow: hidden;
  }

  .frame {
    position: relative;
    padding: 10px;
    border-radius: 14px;
    background: #fff;
    box-shadow: 0 8px 28px rgb(0 0 0 / 0.32);
    max-width: 100%;
    box-sizing: border-box;
  }

  /* The canvas carries intrinsic pixel dimensions, so it has to be told to
     shrink or a narrow window is scrolled sideways instead of scaled. */
  .frame canvas {
    display: block;
    width: 216px;
    height: auto;
    max-width: 100%;
    aspect-ratio: 1;
    image-rendering: pixelated;
  }

  .halo {
    position: absolute;
    inset: -6px;
    border-radius: 20px;
    border: 1px solid var(--accent-a35);
    animation: halo 2.4s ease-in-out infinite;
    pointer-events: none;
  }

  .steps {
    margin: 0;
    padding-left: 18px;
    font-size: 12.5px;
    line-height: 1.7;
    color: var(--text-2);
    align-self: stretch;
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

  .ghost {
    min-height: 38px;
    border-radius: var(--radius);
    border: 1px solid var(--hairline);
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    font-size: 13px;
  }

  .ghost:disabled {
    opacity: 0.35;
  }

  .link {
    align-self: flex-start;
    padding: 0;
    background: none;
    border: none;
    color: var(--text-3);
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
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
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    margin: 0;
    font-size: 12px;
    color: var(--text-2);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: halo 1.4s ease-in-out infinite;
  }

  .failure {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
    text-align: center;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgb(0 0 0 / 0.58);
    backdrop-filter: blur(3px);
  }

  .sheet {
    width: 100%;
    max-width: 340px;
    box-sizing: border-box;
    padding: 18px;
    border-radius: var(--radius);
    background: var(--surface-2, var(--surface));
    border: 1px solid var(--hairline);
    box-shadow: 0 18px 48px rgb(0 0 0 / 0.5);
  }

  .sheet h2 {
    margin: 6px 0 8px;
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  .body {
    margin: 0 0 10px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-2);
  }

  .fingerprint {
    margin: 0 0 14px;
    padding: 7px 9px;
    border-radius: var(--radius-s);
    background: var(--surface);
    border: 1px solid var(--hairline);
    font-family: var(--font-mono);
    font-size: 10.5px;
    line-height: 1.5;
    color: var(--text-3);
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .actions button {
    flex: 1;
  }
</style>
