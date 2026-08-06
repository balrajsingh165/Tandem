<script lang="ts">
  /**
   * Phone switcher: names the phone a command will be sent to, and lets the user
   * change it when more than one is paired. Hidden entirely for a single phone,
   * where there is nothing to choose.
   */

  import { ipc, type ConnectionStatus, type PhoneSummary } from '$lib/ipc';
  import { phones, selectedPhoneId } from '$lib/state';

  let open = $state(false);

  const selected = $derived(
    $phones.find((phone) => phone.deviceId === $selectedPhoneId) ?? $phones[0],
  );

  /** A phone mid-call must be obvious in the list: switching away is not ending. */
  function detail(phone: PhoneSummary): string {
    const live = phone.calls.find((call) => call.state !== 'disconnected');
    if (live) return live.remoteDisplayName || live.remoteNumber || 'in a call';
    return label(phone.connection);
  }

  function label(status: ConnectionStatus): string {
    if (status === 'live') return 'Ready';
    if (status === 'connecting' || status === 'resuming') return 'Connecting…';
    if (status === 'backoff') return 'Reconnecting…';
    if (status === 'terminated') return 'Disconnected';
    return 'Offline';
  }

  async function choose(phone: PhoneSummary): Promise<void> {
    open = false;
    if (phone.deviceId === $selectedPhoneId) return;
    await ipc.selectPhone(phone.deviceId);
  }
</script>

{#if $phones.length > 1 && selected}
  <div class="switcher">
    <button
      type="button"
      class="current"
      aria-haspopup="listbox"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      <span class="dot" class:live={selected.connection === 'live'} aria-hidden="true"></span>
      <span class="name">{selected.name}</span>
      <span class="caret" aria-hidden="true">{open ? '▲' : '▼'}</span>
    </button>

    {#if open}
      <ul class="list rise" role="listbox" aria-label="Call from">
        {#each $phones as phone (phone.deviceId)}
          <li>
            <button
              type="button"
              role="option"
              aria-selected={phone.deviceId === $selectedPhoneId}
              class:selected={phone.deviceId === $selectedPhoneId}
              onclick={() => choose(phone)}
            >
              <span class="dot" class:live={phone.connection === 'live'} aria-hidden="true"></span>
              <span class="text">
                <span class="name">{phone.name}</span>
                <span class="detail">{detail(phone)}</span>
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .switcher {
    position: relative;
  }

  .current {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 5px 9px;
    border-radius: 999px;
    border: 1px solid var(--hairline);
    background: var(--surface);
    color: var(--text-2);
    font-size: 11.5px;
    font-weight: 650;
  }

  .current:hover {
    color: var(--text);
    border-color: var(--accent-a35);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-3);
    flex: none;
  }

  .dot.live {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent-a35);
  }

  .caret {
    font-size: 7px;
    opacity: 0.6;
  }

  .list {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 30;
    min-width: 210px;
    margin: 0;
    padding: 5px;
    list-style: none;
    border-radius: var(--radius);
    border: 1px solid var(--hairline-strong);
    background: var(--surface-hi, var(--surface));
    box-shadow: 0 14px 34px rgb(0 0 0 / 0.45);
  }

  .list button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px 9px;
    border-radius: var(--radius-s);
    background: none;
    color: var(--text-2);
    text-align: left;
  }

  .list button:hover {
    background: var(--surface);
    color: var(--text);
  }

  .list button.selected {
    color: var(--accent);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .name {
    font-size: 12.5px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .detail {
    font-size: 10.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
