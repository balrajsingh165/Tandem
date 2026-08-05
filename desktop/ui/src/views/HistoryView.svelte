<script lang="ts">
  /**
   * History view: the read-only mirrored call log with incremental loading and
   * call-back actions; displays the sync freshness state from state.ts.
   */

  import { ipc } from '$lib/ipc';
  import { history, isConnected } from '$lib/state';
  import { formatDuration, formatNumber, formatTimestamp } from '$lib/format';

  const PAGE_SIZE = 200;

  let loading = $state(false);
  let hasMore = $state(true);
  let failure = $state<string | null>(null);

  async function load(): Promise<void> {
    if (loading) return;
    loading = true;
    failure = null;
    try {
      const oldest = $history.at(-1)?.startedAtMs ?? 0;
      const page = await ipc.history(oldest, PAGE_SIZE);
      history.update((current) => {
        const seen = new Set(current.map((e) => e.entryId));
        return [...current, ...page.entries.filter((e) => !seen.has(e.entryId))];
      });
      hasMore = page.hasMore;
    } catch (error) {
      failure = error instanceof Error ? error.message : 'Could not load history';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if ($isConnected && $history.length === 0) void load();
  });
</script>

<section class="history">
  <header>
    <h1>Recents</h1>
    <p class="label">Mirrored from your phone · read-only</p>
  </header>

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#if $history.length === 0}
    <div class="empty">
      <span class="glyph" aria-hidden="true">↺</span>
      <p class="title">{loading ? 'Loading…' : 'No calls yet'}</p>
      <p class="sub">
        {#if !$isConnected}
          Pair a phone to see its call history.
        {:else}
          Calls made or received on your phone appear here.
        {/if}
      </p>
    </div>
  {/if}

  <ul>
    {#each $history as entry (entry.entryId)}
      <li class="rise">
        <span class="avatar" aria-hidden="true">
          {(entry.displayName || entry.number || '?').charAt(0).toUpperCase()}
        </span>
        <span class="who">
          <span class="name">{entry.displayName || formatNumber(entry.number)}</span>
          <span class="when">
            {formatTimestamp(entry.startedAtMs)}
            {#if entry.durationSeconds > 0}
              · <span class="numeric">{formatDuration(entry.durationSeconds)}</span>
            {/if}
          </span>
        </span>
        <button
          type="button"
          class="dial"
          disabled={!$isConnected}
          aria-label={`Call ${entry.displayName || entry.number}`}
          onclick={() => ipc.dial(entry.number)}
        >
          📞
        </button>
      </li>
    {/each}
  </ul>

  {#if hasMore && $history.length > 0}
    <button type="button" class="more" onclick={load} disabled={loading}>
      {loading ? 'Loading…' : 'Load older'}
    </button>
  {/if}
</section>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: 10px;
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

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 9px;
    border-radius: var(--radius-s);
    transition: background 0.16s ease;
  }

  li:hover {
    background: var(--surface);
  }

  .avatar {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 50%;
    background: var(--surface-hi);
    color: var(--text-2);
    font-size: 13px;
    font-weight: 700;
  }

  .who {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .when {
    font-size: 11px;
    color: var(--text-3);
  }

  .dial {
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--surface-hi);
    border: 1px solid var(--hairline);
    font-size: 13px;
    opacity: 0;
    transition: opacity 0.16s ease, transform 0.16s var(--ease-spring);
  }

  /* Revealing the action on hover keeps a long list calm. */
  li:hover .dial,
  .dial:focus-visible {
    opacity: 1;
  }

  .dial:hover:not(:disabled) {
    border-color: var(--accent-a35);
    transform: scale(1.06);
  }

  .dial:disabled {
    cursor: not-allowed;
  }

  .empty {
    text-align: center;
    padding: 44px 16px;
    color: var(--text-3);
  }

  .empty .glyph {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin: 0 auto 10px;
    border-radius: 50%;
    background: var(--surface);
    font-size: 18px;
  }

  .empty .title {
    margin: 0 0 3px;
    font-size: 14px;
    font-weight: 650;
    color: var(--text-2);
  }

  .empty .sub {
    margin: 0;
    font-size: 12px;
  }

  .more {
    align-self: center;
    padding: 7px 14px;
    border-radius: 999px;
    border: 1px solid var(--hairline);
    font-size: 12px;
    font-weight: 600;
  }

  .more:hover:not(:disabled) {
    background: var(--surface);
  }

  .failure {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
</style>
