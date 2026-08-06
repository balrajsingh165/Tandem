/**
 * Typed wrapper over the JSON-RPC client for the daemon socket, using
 * ts-rs-generated types from tandem_ipc::api. The only module that talks to the
 * daemon; views never do.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type ConnectionStatus =
  | 'idle'
  | 'discovering'
  | 'connecting'
  | 'authenticating'
  | 'pairingProvisional'
  | 'resuming'
  | 'live'
  | 'backoff'
  | 'terminated';

export type AudioRoute = 'earpiece' | 'speaker' | 'wiredHeadset' | 'bluetooth';

export type CallState =
  | 'connecting'
  | 'dialing'
  | 'ringing'
  | 'active'
  | 'holding'
  | 'disconnecting'
  | 'disconnected';

export interface CallView {
  callId: string;
  state: CallState;
  remoteNumber: string;
  remoteDisplayName: string;
  startedAtMs: number;
  isConference: boolean;
  canHold: boolean;
  canMerge: boolean;
  isEmergency: boolean;
}

export interface HistoryEntry {
  entryId: string;
  number: string;
  displayName: string;
  startedAtMs: number;
  durationSeconds: number;
}

export interface AudioDeviceView {
  route: AudioRoute;
  btDeviceAddress: string;
  name: string;
}

export interface OfferResult {
  payload: string;
  desktopName: string;
}

export interface StatusResult {
  connection: ConnectionStatus;
  phoneName: string;
  calls: CallView[];
  audioRoute: AudioRoute;
  microphoneMuted: boolean;
  desktopAudioAvailable: boolean;
  audioDevices: AudioDeviceView[];
  activeBtDeviceAddress: string;
}

export type IpcEvent =
  | { type: 'connectionChanged'; connection: ConnectionStatus }
  | { type: 'callsChanged'; calls: CallView[] }
  | { type: 'audioRouteChanged'; route: AudioRoute; btDeviceAddress: string }
  | {
      type: 'audioDevicesChanged';
      devices: AudioDeviceView[];
      activeRoute: AudioRoute;
      activeBtDeviceAddress: string;
    }
  | { type: 'historyChanged'; logVersion: number }
  | { type: 'emergencyBlocked'; number: string; guidance: string }
  | { type: 'audioPipelineChanged'; scoActive: boolean; latencyMs: number | null }
  | { type: 'pairingProgress'; state: string; shortCode: string | null }
  | { type: 'pairingApprovalRequested'; phoneName: string; phoneFingerprint: string }
  | { type: 'revoked'; reason: string };

/** Stable codes from tandem_ipc::error, so callers branch on cause not text. */
export const IPC_EMERGENCY_BLOCKED = -32005;
export const IPC_ALREADY_HANDLED = -32006;
export const IPC_AUDIO_UNAVAILABLE = -32008;
export const IPC_DAEMON_UNAVAILABLE = -32000;

export class IpcCallError extends Error {
  constructor(
    readonly code: number,
    message: string,
  ) {
    super(message);
    this.name = 'IpcCallError';
  }

  /** Retrying an emergency refusal or a lost answer race would be wrong. */
  get isRetryable(): boolean {
    return this.code === IPC_DAEMON_UNAVAILABLE;
  }
}

async function call<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>('daemon_request', { method, params });
  } catch (raw) {
    const error = raw as { code?: number; message?: string };
    throw new IpcCallError(error.code ?? -32099, error.message ?? 'daemon request failed');
  }
}

export const ipc = {
  status: () => call<StatusResult>('status'),
  dial: (number: string, simSlot = -1) => call<void>('dial', { number, simSlot }),
  answer: (callId: string) => call<void>('answer', { callId }),
  reject: (callId: string) => call<void>('reject', { callId }),
  end: (callId: string) => call<void>('end', { callId }),
  mute: (muted: boolean) => call<void>('mute', { muted }),
  hold: (callId: string) => call<void>('hold', { callId }),
  unhold: (callId: string) => call<void>('unhold', { callId }),
  merge: (callId: string, otherCallId: string) => call<void>('merge', { callId, otherCallId }),
  dtmf: (callId: string, digits: string) => call<void>('dtmf', { callId, digits }),
  audioRoute: (route: AudioRoute, btDeviceAddress = '') =>
    call<void>('audioRoute', { route, btDeviceAddress }),
  history: (sinceMs: number, limit: number) =>
    call<{ entries: HistoryEntry[]; hasMore: boolean }>('history', { sinceMs, limit }),
  pairing: (qrPayload: string) => call<void>('pairing', { qrPayload }),
  pairingOffer: () => call<OfferResult>('pairingOffer'),
  pairingConfirm: (accept: boolean) => call<void>('pairingConfirm', { accept }),
  unpair: () => call<void>('unpair'),

  onEvent: (handler: (event: IpcEvent) => void) =>
    listen<IpcEvent>('tandem://event', (message) => handler(message.payload)),
};
