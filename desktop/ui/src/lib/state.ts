/**
 * Svelte stores derived from daemon events: mirrored call snapshot, connection
 * state, history cache, pairing progress. Read-only projections; commands go
 * through ipc.ts.
 */

import { derived, writable, type Readable } from 'svelte/store';
import type {
  AudioRoute,
  CallView,
  ConnectionStatus,
  HistoryEntry,
  IpcEvent,
  StatusResult,
} from './ipc';

export const connection = writable<ConnectionStatus>('idle');
export const phoneName = writable<string>('');
export const calls = writable<CallView[]>([]);
export const audioRoute = writable<AudioRoute>('earpiece');
export const microphoneMuted = writable<boolean>(false);
export const desktopAudioAvailable = writable<boolean>(false);
export const history = writable<HistoryEntry[]>([]);
export const pairingState = writable<{ state: string; shortCode: string | null } | null>(null);

/** Non-null while the user must be told why a dial was refused (ADR-0008). */
export const emergencyNotice = writable<{ number: string; guidance: string } | null>(null);

export const revocation = writable<string | null>(null);

/** The call the user is acting on: ringing first, then the active one. */
export const primaryCall: Readable<CallView | null> = derived(calls, ($calls) => {
  const ringing = $calls.find((c) => c.state === 'ringing');
  if (ringing) return ringing;
  const live = $calls.find((c) => c.state !== 'disconnected');
  return live ?? null;
});

export const isConnected: Readable<boolean> = derived(
  connection,
  ($connection) => $connection === 'live',
);

/** An active emergency call is surfaced read-only; controls must be disabled. */
export const hasActiveEmergency: Readable<boolean> = derived(calls, ($calls) =>
  $calls.some((c) => c.isEmergency && c.state !== 'disconnected'),
);

export function applyStatus(status: StatusResult): void {
  connection.set(status.connection);
  phoneName.set(status.phoneName);
  calls.set(status.calls);
  audioRoute.set(status.audioRoute);
  microphoneMuted.set(status.microphoneMuted);
  desktopAudioAvailable.set(status.desktopAudioAvailable);
}

export function applyEvent(event: IpcEvent): void {
  switch (event.type) {
    case 'connectionChanged':
      connection.set(event.connection);
      break;
    case 'callsChanged':
      calls.set(event.calls);
      break;
    case 'audioRouteChanged':
      audioRoute.set(event.route);
      break;
    case 'historyChanged':
      break;
    case 'emergencyBlocked':
      emergencyNotice.set({ number: event.number, guidance: event.guidance });
      break;
    case 'audioPipelineChanged':
      break;
    case 'pairingProgress':
      pairingState.set({ state: event.state, shortCode: event.shortCode });
      break;
    case 'revoked':
      revocation.set(event.reason);
      connection.set('terminated');
      calls.set([]);
      break;
  }
}
