/**
 * Pure formatting helpers: phone-number display, call duration, timestamps, and
 * BT/route labels. No state, no IPC.
 */

import type { AudioRoute, CallState, ConnectionStatus } from './ipc';

/** Groups a North American number for readability, leaving others untouched. */
export function formatNumber(raw: string): string {
  if (!raw) return 'Unknown';
  const digits = raw.replace(/\D/g, '');
  if (raw.startsWith('+1') && digits.length === 11) {
    return `+1 (${digits.slice(1, 4)}) ${digits.slice(4, 7)}-${digits.slice(7)}`;
  }
  if (digits.length === 10 && !raw.startsWith('+')) {
    return `(${digits.slice(0, 3)}) ${digits.slice(3, 6)}-${digits.slice(6)}`;
  }
  return raw;
}

export function formatDuration(seconds: number): string {
  const safe = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const secs = safe % 60;
  const pad = (n: number) => n.toString().padStart(2, '0');
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(secs)}` : `${minutes}:${pad(secs)}`;
}

export function formatTimestamp(epochMs: number): string {
  if (!epochMs) return '';
  return new Date(epochMs).toLocaleString();
}

export function callStateLabel(state: CallState): string {
  const labels: Record<CallState, string> = {
    connecting: 'Connecting',
    dialing: 'Dialing',
    ringing: 'Incoming call',
    active: 'In call',
    holding: 'On hold',
    disconnecting: 'Ending',
    disconnected: 'Ended',
  };
  return labels[state];
}

export function audioRouteLabel(route: AudioRoute): string {
  const labels: Record<AudioRoute, string> = {
    earpiece: 'Phone earpiece',
    speaker: 'Phone speaker',
    wiredHeadset: 'Wired headset',
    bluetooth: 'Bluetooth',
  };
  return labels[route];
}

export function connectionLabel(status: ConnectionStatus): string {
  const labels: Record<ConnectionStatus, string> = {
    idle: 'Not connected',
    discovering: 'Looking for your phone',
    connecting: 'Connecting',
    authenticating: 'Authenticating',
    pairingProvisional: 'Pairing',
    resuming: 'Syncing',
    live: 'Connected',
    backoff: 'Reconnecting',
    terminated: 'Disconnected',
  };
  return labels[status];
}
