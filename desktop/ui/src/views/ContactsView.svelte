<script lang="ts">
  /**
   * Contacts view: everyone the synced call log knows about, searchable by name or
   * number, one click to dial. Names come from the phone, which resolves them
   * against its own address book.
   */

  import { ipc } from '$lib/ipc';
  import {
    contacts,
    contactsError,
    contactsSync,
    history,
    isConnected,
    loadContacts,
  } from '$lib/state';
  import { contactsFromHistory, normalize, type Contact } from '$lib/contacts';
  import { formatNumber } from '$lib/format';

  let query = $state('');
  let failure = $state<string | null>(null);

  const syncing = $derived($contactsSync === 'syncing');

  // The phone's address book is authoritative. Call history fills in anyone not
  // saved as a contact, so a number you have spoken to is still reachable here.
  const all = $derived.by(() => {
    const synced: Contact[] = $contacts.map((entry) => ({
      name: entry.displayName || entry.number,
      number: entry.number,
      label: entry.label,
      // A saved contact you have never called has no last-called time; 0 sorts it
      // after anyone you have actually spoken to.
      lastCalledMs: 0,
    }));

    const seen = new Set(synced.map((c) => normalize(c.number)));
    const fromHistory = contactsFromHistory($history).filter(
      (c) => !seen.has(normalize(c.number)),
    );

    return [...synced, ...fromHistory];
  });

  // Named people first: a bare number is a weaker match for a human scanning.
  const matches = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const digits = normalize(query);

    const filtered = needle
      ? all.filter(
          (contact) =>
            contact.name.toLowerCase().includes(needle) ||
            (digits.length > 0 && normalize(contact.number).includes(digits)),
        )
      : all;

    return filtered.slice(0, 300);
  });

  const named = $derived(matches.filter((c) => c.name !== c.number));
  const unnamed = $derived(matches.filter((c) => c.name === c.number));

  async function call(contact: Contact): Promise<void> {
    if (!$isConnected) return;
    failure = null;
    try {
      await ipc.dial(contact.number);
    } catch (error) {
      failure = error instanceof Error ? error.message : 'Could not place the call';
    }
  }
</script>

<section class="contacts">
  <header>
    <h1>Contacts</h1>
    <p class="label">
      {#if syncing}
        Syncing…
      {:else if $contacts.length > 0}
        {all.length} contacts · {$contacts.length} from your phone
      {:else}
        {all.length} from your call history
      {/if}
    </p>
  </header>

  <input
    class="search"
    type="search"
    bind:value={query}
    placeholder="Search name or number"
    aria-label="Search contacts"
    autocomplete="off"
  />

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  {#if $contactsError}
    <div class="notice">
      <p class="failure" role="alert">{$contactsError}</p>
      <button type="button" class="retry" onclick={() => loadContacts(ipc.contacts)}>
        Try again
      </button>
    </div>
  {/if}

  {#if syncing && all.length === 0}
    <p class="empty"><span class="spinner" aria-hidden="true"></span> Reading contacts from your phone…</p>
  {:else if all.length === 0}
    <p class="empty">
      {$isConnected
        ? 'No contacts yet. Grant Tandem access to Contacts on your phone, then sync again.'
        : "Pair a phone to read its contacts."}
    </p>
    {#if $isConnected}
      <button type="button" class="retry" onclick={() => loadContacts(ipc.contacts)}>
        Sync now
      </button>
    {/if}
  {:else if matches.length === 0}
    <p class="empty">No contact matches “{query}”.</p>
  {:else}
    <ul>
      {#each [...named, ...unnamed] as contact (contact.number)}
        <li>
          <button type="button" onclick={() => call(contact)} disabled={!$isConnected}>
            <span class="avatar" aria-hidden="true">{contact.name.charAt(0).toUpperCase()}</span>
            <span class="meta">
              <span class="name">{contact.name}</span>
              <span class="num numeric">
                {formatNumber(contact.number)}{contact.label ? ` · ${contact.label}` : ''}
              </span>
            </span>
            <span class="go" aria-hidden="true">Call</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if !$isConnected}
    <p class="empty">Pair a phone to place calls.</p>
  {/if}
</section>

<style>
  .contacts {
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

  .search {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--hairline);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
  }

  .search:focus {
    outline: none;
    border-color: var(--accent-a35);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 9px 10px;
    border-radius: var(--radius-s);
    text-align: left;
    transition: background 0.14s ease;
  }

  li button:hover:not(:disabled) {
    background: var(--surface);
  }

  li button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .avatar {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: 50%;
    background: var(--accent-a20);
    color: var(--accent);
    font-size: 13px;
    font-weight: 700;
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .name {
    font-size: 13.5px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .num {
    font-size: 11px;
    color: var(--text-3);
  }

  .go {
    flex: none;
    font-size: 11px;
    font-weight: 700;
    color: var(--accent);
    opacity: 0;
    transition: opacity 0.14s ease;
  }

  li button:hover:not(:disabled) .go {
    opacity: 1;
  }

  .empty,
  .failure {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--text-3);
  }

  .failure {
    color: var(--danger);
  }

  .notice {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--danger-a15);
  }

  .retry {
    align-self: flex-start;
    padding: 7px 12px;
    border-radius: var(--radius-s);
    border: 1px solid var(--hairline);
    background: var(--surface);
    color: var(--accent);
    font-size: 12px;
    font-weight: 650;
  }

  .spinner {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-right: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: halo 1.3s ease-in-out infinite;
  }
</style>
