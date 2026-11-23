// Thin wrapper over the Tauri IPC boundary. All commands are read-only in
// Phase 1. When running outside the Tauri shell (e.g. `vite` in a browser),
// `isTauri` is false and callers should skip invoking.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppConfig, Diagnostics, Snapshot } from './types';

export const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const SNAPSHOT_EVENT = 'telemetry:snapshot';

export function getSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>('get_snapshot');
}

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

export function getDiagnostics(): Promise<Diagnostics> {
  return invoke<Diagnostics>('get_diagnostics');
}

export function onSnapshot(cb: (snap: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>(SNAPSHOT_EVENT, (event) => cb(event.payload));
}
