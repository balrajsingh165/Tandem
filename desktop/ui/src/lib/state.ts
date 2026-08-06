/**
 * Svelte stores derived from daemon events: mirrored call snapshot, connection
 * state, history cache, pairing progress. Read-only projections; commands go
 * through ipc.ts.
 */

import { derived, writable, type Readable } from 'svelte/store';
import type {
  AudioDeviceView,
  ContactView,
  PhoneSummary,
  AudioRoute,
  CallView,
  ConnectionStatus,
  HistoryEntry,
  IpcEvent,
  StatusResult,
} from './ipc';

export const connection = writable<ConnectionStatus>('idle');

/** Every paired phone, and which one commands act on. */
export const phones = writable<PhoneSummary[]>([]);
export const selectedPhoneId = writable<string>('');
export const phoneName = writable<string>('');
export const calls = writable<CallView[]>([]);
export const audioRoute = writable<AudioRoute>('earpiece');

/** Everywhere the live call can be played, as the phone reports it. */
export const audioDevices = writable<AudioDeviceView[]>([]);
export const activeBtDeviceAddress = writable<string>('');
export const microphoneMuted = writable<boolean>(false);
export const desktopAudioAvailable = writable<boolean>(false);
export const history = writable<HistoryEntry[]>([]);

/** The phones' address books, as synced. Empty until a phone reports them. */
export const contacts = writable<ContactView[]>([]);
export const pairingState = writable<{ state: string; shortCode: string | null } | null>(null);

/** Non-null while a phone that scanned this desktop's code awaits a verdict. */
export const pairingApproval = writable<{
  phoneName: string;
  phoneFingerprint: string;
} | null>(null);

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

/**
 * Fills the shared history cache. Name resolution and dial suggestions read from
 * it, so nothing that needs a contact name works until this has run.
 */
export async function loadHistory(
  fetch: (sinceMs: number, limit: number) => Promise<{ entries: HistoryEntry[] }>,
): Promise<void> {
  const page = await fetch(0, 5000);
  history.set(page.entries);
}

/** Fills the shared contact directory the dialer and Contacts view read. */
export async function loadContacts(
  fetch: () => Promise<{ entries: ContactView[] }>,
): Promise<void> {
  const page = await fetch();
  contacts.set(page.entries);
}

export function applyStatus(status: StatusResult): void {
  connection.set(status.connection);
  phones.set(status.phones);
  selectedPhoneId.set(status.selectedPhoneId);
  phoneName.set(status.phoneName);
  calls.set(status.calls);
  audioRoute.set(status.audioRoute);
  audioDevices.set(status.audioDevices);
  activeBtDeviceAddress.set(status.activeBtDeviceAddress);
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
      activeBtDeviceAddress.set(event.btDeviceAddress);
      break;
    case 'audioDevicesChanged':
      audioDevices.set(event.devices);
      audioRoute.set(event.activeRoute);
      activeBtDeviceAddress.set(event.activeBtDeviceAddress);
      break;
    case 'historyChanged':
      break;
    case 'contactsChanged':
      break;
    case 'phonesChanged':
      phones.set(event.phones);
      selectedPhoneId.set(event.selectedPhoneId);
      break;
    case 'emergencyBlocked':
      emergencyNotice.set({ number: event.number, guidance: event.guidance });
      break;
    case 'audioPipelineChanged':
      break;
    case 'pairingProgress':
      pairingState.set({ state: event.state, shortCode: event.shortCode });
      if (!event.state.startsWith('approve:')) pairingApproval.set(null);
      break;
    case 'pairingApprovalRequested':
      pairingApproval.set({
        phoneName: event.phoneName,
        phoneFingerprint: event.phoneFingerprint,
      });
      break;
    case 'revoked':
      revocation.set(event.reason);
      connection.set('terminated');
      calls.set([]);
      break;
  }
}
