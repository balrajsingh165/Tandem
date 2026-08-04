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

  const tone = $derived(
    status === 'live'
      ? 'ok'
      : status === 'terminated' || status === 'idle'
        ? 'off'
        : 'pending',
  );
</script>

<span class="badge {tone}" role="status" aria-label={`Connection: ${connectionLabel(status)}`}>
  <span class="dot" aria-hidden="true"></span>
  {connectionLabel(status)}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.125rem 0.5rem;
    border-radius: 999px;
    font-size: 0.8125rem;
    border: 1px solid var(--border, #d0d0d5);
  }

  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: currentColor;
  }

  .ok {
    color: #1b6e3c;
  }

  .pending {
    color: #8a6100;
  }

  .off {
    color: #6b6b70;
  }
</style>
