<script lang="ts">
  /**
   * Small status badge for connection and audio-route states with accessible
   * labels; used in the header and settings.
   */

  import type { ConnectionStatus } from '$lib/ipc';
  import { connectionLabel } from '$lib/format';

  interface Props {
    status: ConnectionStatus;
  }

  const { status }: Props = $props();

  // Three tones only: settled, working, or off. More would be noise at 11px.
  const tone = $derived(
    status === 'live'
      ? 'ok'
      : status === 'terminated' || status === 'idle'
        ? 'off'
        : 'pending',
  );

  const busy = $derived(tone === 'pending');
</script>

<span
  class="badge {tone}"
  role="status"
  aria-label={`Connection: ${connectionLabel(status)}`}
>
  <span class="dot" class:busy aria-hidden="true"></span>
  {connectionLabel(status)}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.01em;
    border: 1px solid var(--hairline);
    background: var(--surface);
    white-space: nowrap;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex: none;
  }

  .dot.busy {
    animation: halo 1.8s ease-out infinite;
  }

  .ok {
    color: var(--accent);
    border-color: var(--accent-a35);
  }

  .pending {
    color: var(--warn);
  }

  .off {
    color: var(--text-3);
  }
</style>
