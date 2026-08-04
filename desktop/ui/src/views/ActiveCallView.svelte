<script lang="ts">
  /**
   * Active-call view: caller identity, call timer, CallControls, DTMF pad, and
   * the audio route indicator with attach/detach-to-desktop action where a Tier B
   * backend is present.
   */

  import CallControls from '../components/CallControls.svelte';
  import DialPad from '../components/DialPad.svelte';
  import { ipc } from '$lib/ipc';
  import { audioRoute, calls, desktopAudioAvailable, microphoneMuted, primaryCall } from '$lib/state';
  import { audioRouteLabel, callStateLabel, formatDuration, formatNumber } from '$lib/format';

  let showDtmf = $state(false);
  let elapsed = $state(0);

  $effect(() => {
    const call = $primaryCall;
    if (!call || call.state !== 'active') {
      elapsed = 0;
      return;
    }
    const tick = () => (elapsed = Math.floor((Date.now() - call.startedAtMs) / 1000));
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  const onDesktop = $derived($audioRoute === 'bluetooth');

  async function toggleRoute(): Promise<void> {
    await ipc.audioRoute(onDesktop ? 'earpiece' : 'bluetooth');
  }

  async function answer(callId: string): Promise<void> {
    await ipc.answer(callId);
  }

  function otherCallId(currentId: string): string {
    return $calls.find((c) => c.callId !== currentId && c.state !== 'disconnected')?.callId ?? '';
  }
</script>

{#if $primaryCall}
  {@const call = $primaryCall}
  <section class="call">
    <p class="state">{callStateLabel(call.state)}</p>
    <h1>{call.remoteDisplayName || formatNumber(call.remoteNumber)}</h1>
    {#if call.remoteDisplayName}
      <p class="number">{formatNumber(call.remoteNumber)}</p>
    {/if}

    {#if call.state === 'active'}
      <p class="timer" aria-label="Call duration">{formatDuration(elapsed)}</p>
    {/if}

    {#if call.state === 'ringing'}
      <div class="incoming">
        <button type="button" class="accept" onclick={() => answer(call.callId)}>Answer</button>
        <button type="button" onclick={() => ipc.reject(call.callId)}>Decline</button>
      </div>
    {:else}
      <CallControls
        {call}
        muted={$microphoneMuted}
        onmute={(muted) => ipc.mute(muted)}
        onhold={(hold) => (hold ? ipc.hold(call.callId) : ipc.unhold(call.callId))}
        onmerge={() => ipc.merge(call.callId, otherCallId(call.callId))}
        onend={() => ipc.end(call.callId)}
      />

      <div class="audio">
        <span>Audio: {audioRouteLabel($audioRoute)}</span>
        {#if $desktopAudioAvailable && !call.isEmergency}
          <button type="button" onclick={toggleRoute}>
            {onDesktop ? 'Move to phone' : 'Move to this computer'}
          </button>
        {:else if !$desktopAudioAvailable}
          <span class="hint">
            This build has no desktop audio path — talk on the handset or a device paired to your
            phone.
          </span>
        {/if}
      </div>

      <button type="button" class="dtmf-toggle" onclick={() => (showDtmf = !showDtmf)}>
        {showDtmf ? 'Hide keypad' : 'Show keypad'}
      </button>
      {#if showDtmf}
        <DialPad ondigit={(digit) => ipc.dtmf(call.callId, digit)} />
      {/if}
    {/if}
  </section>
{:else}
  <p class="idle">No active call.</p>
{/if}

<style>
  .call {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-width: 22rem;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
  }

  .state,
  .number,
  .hint {
    margin: 0;
    font-size: 0.875rem;
    opacity: 0.75;
  }

  .timer {
    margin: 0;
    font-variant-numeric: tabular-nums;
    font-size: 1.125rem;
  }

  .incoming {
    display: flex;
    gap: 0.5rem;
  }

  .incoming button,
  .dtmf-toggle,
  .audio button {
    min-height: 2.5rem;
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.5rem;
    background: var(--surface, #fff);
    cursor: pointer;
    font: inherit;
  }

  .incoming button {
    flex: 1;
  }

  .accept {
    background: #1b6e3c;
    color: #fff;
    border-color: #1b6e3c;
  }

  .audio {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    font-size: 0.875rem;
  }

  .idle {
    opacity: 0.7;
  }
</style>
