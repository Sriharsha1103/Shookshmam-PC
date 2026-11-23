// Thin wrapper over the Tauri IPC boundary. All commands are read-only in
// Phase 1. When running outside the Tauri shell (e.g. `vite` in a browser),
// `isTauri` is false and callers should skip invoking.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AppConfig,
  ApplyPlan,
  ChannelInfo,
  Diagnostics,
  Effect,
  FanCurve,
  LightingZone,
  Profile,
  Rgb,
  Snapshot,
  SupportBundle,
} from './types';

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

// Cooling (read-only / preview)
export function getFanChannels(): Promise<ChannelInfo[]> {
  return invoke<ChannelInfo[]>('get_fan_channels');
}
export function getFanCurves(): Promise<FanCurve[]> {
  return invoke<FanCurve[]>('get_fan_curves');
}

// Lighting (read-only / preview)
export function getLightingZones(): Promise<LightingZone[]> {
  return invoke<LightingZone[]>('get_lighting_zones');
}
export function previewEffect(effect: Effect, ledCount: number): Promise<Rgb[]> {
  return invoke<Rgb[]>('preview_effect', { effect, ledCount });
}

// Profiles (read-only / preview)
export function listProfiles(): Promise<Profile[]> {
  return invoke<Profile[]>('list_profiles');
}
export function previewProfile(id: string): Promise<ApplyPlan> {
  return invoke<ApplyPlan>('preview_profile', { id });
}
export function getSupportBundle(): Promise<SupportBundle> {
  return invoke<SupportBundle>('get_support_bundle');
}
