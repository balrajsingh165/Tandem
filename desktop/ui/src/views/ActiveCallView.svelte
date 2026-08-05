<script lang="ts">
  /**
   * Active-call view: caller identity, call timer, CallControls, DTMF pad, and
   * the audio route indicator with attach/detach-to-desktop action where a Tier B
   * backend is present.
   */

  import CallControls from '../components/CallControls.svelte';
  import DialPad from '../components/DialPad.svelte';
  import { ipc } from '$lib/ipc';
  import {
    audioRoute,
    calls,
    desktopAudioAvailable,
    history,
    microphoneMuted,
    primaryCall,
  } from '$lib/state';
  import { audioRouteLabel, callStateLabel, formatDuration, formatNumber } from '$lib/format';
  import { nameFor } from '$lib/contacts';

  let showDtmf = $state(false);
  let elapsed = $state(0);

  $effect(() => {
    const call = $primaryCall;
    if (!call || call.state !== 'active') {
      elapsed = 0;
      return;
    }
    const tick = () => (elapsed = Math.max(0, Math.floor((Date.now() - call.startedAtMs) / 1000)));
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  // The phone resolves the name when it can; history fills the gap otherwise.
  const displayName = $derived(
    $primaryCall
      ? $primaryCall.remoteDisplayName || nameFor($primaryCall.remoteNumber, $history)
      : null,
  );

  const onDesktop = $derived($audioRoute === 'bluetooth');
  const ringing = $derived($primaryCall?.state === 'ringing');

  function otherCallId(currentId: string): string {
    return $calls.find((c) => c.callId !== currentId && c.state !== 'disconnected')?.callId ?? '';
  }
</script>

{#if $primaryCall}
  {@const call = $primaryCall}
  <section class="call rise">
    <div class="identity">
      <div class="avatar" class:pulsing={ringing || call.state === 'active'} aria-hidden="true">
        {(displayName ?? call.remoteNumber ?? '?').charAt(0).toUpperCase()}
      </div>
      <h1>{displayName ?? formatNumber(call.remoteNumber) ?? 'Unknown'}</h1>
      {#if displayName}
        <p class="sub numeric">{formatNumber(call.remoteNumber)}</p>
      {/if}
      <p class="state" class:live={call.state === 'active'}>
        {callStateLabel(call.state)}
        {#if call.state === 'active'}
          <span class="dot" aria-hidden="true">·</span>
          <span class="timer numeric">{formatDuration(elapsed)}</span>
        {/if}
      </p>
    </div>

    {#if ringing}
      <div class="incoming">
        <button type="button" class="decline" onclick={() => ipc.reject(call.callId)}>
          Decline
        </button>
        <button type="button" class="accept" onclick={() => ipc.answer(call.callId)}>
          Answer
        </button>
      </div>
    {:else}
      <CallControls
        {call}
        muted={$microphoneMuted}
        canMerge={otherCallId(call.callId) !== ''}
        onmute={(muted) => ipc.mute(muted)}
        onhold={(hold) => (hold ? ipc.hold(call.callId) : ipc.unhold(call.callId))}
        onmerge={() => ipc.merge(call.callId, otherCallId(call.callId))}
        onend={() => ipc.end(call.callId)}
      />

      <div class="audio">
        <div class="row">
          <span class="label">Audio</span>
          <span class="route">{audioRouteLabel($audioRoute)}</span>
        </div>
        {#if $desktopAudioAvailable && !call.isEmergency}
          <button
            type="button"
            class="switch"
            onclick={() => ipc.audioRoute(onDesktop ? 'earpiece' : 'bluetooth')}
          >
            {onDesktop ? 'Move audio to phone' : 'Move audio to this computer'}
          </button>
        {:else if !$desktopAudioAvailable}
          <p class="hint">
            This build has no desktop audio path — talk on the handset or a device paired to your
            phone.
          </p>
        {/if}
      </div>

      <button type="button" class="keypad-toggle" onclick={() => (showDtmf = !showDtmf)}>
        {showDtmf ? 'Hide keypad' : 'Keypad'}
      </button>
      {#if showDtmf}
        <div class="rise">
          <DialPad compact ondigit={(digit) => ipc.dtmf(call.callId, digit)} />
        </div>
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
    gap: 14px;
  }

  .identity {
    text-align: center;
    padding: 8px 0 2px;
  }

  .avatar {
    width: 76px;
    height: 76px;
    margin: 0 auto 12px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--surface-hi);
    border: 1px solid var(--hairline-strong);
    color: var(--accent);
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 650;
  }

  /* A slow halo signals a live line without demanding attention. */
  .avatar.pulsing {
    animation: halo 2.4s ease-out infinite;
    border-color: var(--accent-a35);
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }

  .sub {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-3);
  }

  .state {
    margin: 8px 0 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }

  .state.live {
    color: var(--accent);
  }

  .timer {
    font-size: 13px;
  }

  .dot {
    opacity: 0.5;
  }

  .incoming {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .incoming button {
    min-height: 50px;
    border-radius: var(--radius);
    font-weight: 700;
    transition: transform 0.16s var(--ease-spring), filter 0.16s ease;
  }

  .incoming button:active {
    transform: scale(0.97);
  }

  .accept {
    background: var(--accent);
    color: var(--accent-ink);
    box-shadow: var(--glow);
  }

  .decline {
    background: var(--surface);
    border: 1px solid var(--hairline-strong);
    color: var(--danger);
  }

  .audio {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 11px 12px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--hairline);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .route {
    font-size: 12px;
    font-weight: 600;
  }

  .switch {
    text-align: left;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }

  .switch:hover {
    text-decoration: underline;
  }

  .hint {
    margin: 0;
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-3);
  }

  .keypad-toggle {
    align-self: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--hairline);
  }

  .keypad-toggle:hover {
    background: var(--surface);
  }

  .idle {
    text-align: center;
    color: var(--text-3);
    padding-top: 40px;
  }
</style>
