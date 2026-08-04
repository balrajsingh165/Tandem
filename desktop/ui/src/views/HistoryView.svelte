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
  <h1>Recent calls</h1>
  <p class="source">Mirrored from your phone. This view is read-only.</p>

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#if $history.length === 0 && !loading}
    <p class="empty">No calls yet.</p>
  {/if}

  <ul>
    {#each $history as entry (entry.entryId)}
      <li>
        <div class="who">
          <span class="name">{entry.displayName || formatNumber(entry.number)}</span>
          <span class="when">{formatTimestamp(entry.startedAtMs)}</span>
        </div>
        <span class="duration">{formatDuration(entry.durationSeconds)}</span>
        <button type="button" disabled={!$isConnected} onclick={() => ipc.dial(entry.number)}>
          Call
        </button>
      </li>
    {/each}
  </ul>

  {#if hasMore && $history.length > 0}
    <button type="button" onclick={load} disabled={loading}>
      {loading ? 'Loading…' : 'Load older'}
    </button>
  {/if}
</section>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 30rem;
  }

  h1 {
    margin: 0;
    font-size: 1.25rem;
  }

  .source,
  .empty {
    margin: 0;
    font-size: 0.8125rem;
    opacity: 0.7;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border, #e5e5ea);
  }

  .who {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .when {
    font-size: 0.75rem;
    opacity: 0.65;
  }

  .duration {
    font-variant-numeric: tabular-nums;
    font-size: 0.8125rem;
    opacity: 0.75;
  }

  button {
    border: 1px solid var(--border, #d0d0d5);
    border-radius: 0.375rem;
    background: var(--surface, #fff);
    padding: 0.25rem 0.625rem;
    cursor: pointer;
    font: inherit;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .failure {
    color: #b3261e;
    font-size: 0.875rem;
  }
</style>
