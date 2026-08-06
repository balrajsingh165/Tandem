/**
 * Resolves a dialled number to a display name using the mirrored call history,
 * which already carries the phone's cached contact names. This is a projection
 * of what the phone knows, not a contacts database: a number never called
 * before has no name here.
 */

import type { HistoryEntry } from './ipc';

/** Digits only, so formatting differences never prevent a match. */
export function normalize(number: string): string {
  return number.replace(/\D/g, '');
}

/**
 * Matches on the last 7 digits so local and internationally-formatted spellings
 * of the same number resolve to one contact.
 */
function matchKey(number: string): string {
  const digits = normalize(number);
  return digits.length > 7 ? digits.slice(-7) : digits;
}

export interface Contact {
  /** "Mobile", "Work"… when the phone supplied one. */
  label?: string;
  name: string;
  number: string;
  lastCalledMs: number;
}

/** One entry per distinct number that has a known name, newest first. */
export function contactsFromHistory(history: HistoryEntry[]): Contact[] {
  const byKey = new Map<string, Contact>();

  for (const entry of history) {
    if (!entry.displayName) continue;
    const key = matchKey(entry.number);
    if (!key) continue;

    const existing = byKey.get(key);
    if (!existing || entry.startedAtMs > existing.lastCalledMs) {
      byKey.set(key, {
        name: entry.displayName,
        number: entry.number,
        lastCalledMs: entry.startedAtMs,
      });
    }
  }

  return [...byKey.values()].sort((a, b) => b.lastCalledMs - a.lastCalledMs);
}

/** The saved name for a number, or null when it is not a known contact. */
export function nameFor(number: string, history: HistoryEntry[]): string | null {
  const key = matchKey(number);
  if (!key) return null;
  return contactsFromHistory(history).find((c) => matchKey(c.number) === key)?.name ?? null;
}

/**
 * Live suggestions while dialling: matches a name prefix or any run of digits,
 * so both "ali" and "5550" narrow the list.
 */
export function suggest(query: string, history: HistoryEntry[], limit = 4): Contact[] {
  const trimmed = query.trim();
  if (!trimmed) return [];

  const digits = normalize(trimmed);
  const lowered = trimmed.toLowerCase();

  return contactsFromHistory(history)
    .filter((contact) => {
      const byName = contact.name.toLowerCase().includes(lowered);
      const byNumber = digits.length > 0 && normalize(contact.number).includes(digits);
      return byName || byNumber;
    })
    .slice(0, limit);
}
