// TypeScript mirror of the Rust serde types (snake_case over the IPC boundary).

export type SensorStatus =
  | 'available'
  | 'unsupported'
  | 'unavailable'
  | 'permission_denied'
  | 'stale'
  | 'error';

export type SensorCategory =
  | 'temperature'
  | 'utilization'
  | 'frequency'
  | 'voltage'
  | 'power'
  | 'memory'
  | 'fan_speed'
  | 'storage_health';

export interface SensorReading {
  id: string;
  device_id: string;
  name: string;
  category: SensorCategory;
  value: number | null;
  unit: string;
  source: string;
  status: SensorStatus;
  timestamp_unix_ms: number;
  quality_note: string | null;
}

export interface Snapshot {
  timestamp_unix_ms: number;
  readings: SensorReading[];
}

export interface AppConfig {
  sample_interval_ms: number;
  history_retention_days: number;
  history_enabled: boolean;
  read_only: boolean;
  simulated: boolean;
}

export interface ProviderStatus {
  provider_id: string;
  devices: number;
  readings: number;
  ok: boolean;
  last_error: string | null;
}

export interface Diagnostics {
  collected_at_unix_ms: number;
  providers: ProviderStatus[];
  total_readings: number;
  available: number;
  stale: number;
  unavailable: number;
  unsupported: number;
  permission_denied: number;
  errored: number;
}
