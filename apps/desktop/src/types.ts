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

// --- Phase 3: cooling ---

export interface ChannelInfo {
  id: string;
  label: string;
  min_duty: number;
  max_duty: number;
  allow_stop: boolean;
  validated: boolean;
}

export interface CurvePoint {
  temperature_c: number;
  duty_percent: number;
}

export type SensorLoss = { safe_fallback_duty: number } | 'release_to_firmware';

export interface FanCurve {
  id: string;
  name: string;
  temp_source_ids: string[];
  source_policy: 'max' | 'average';
  points: CurvePoint[];
  min_duty: number;
  max_duty: number;
  hysteresis_c: number;
  ramp_up_pct_per_s: number;
  ramp_down_pct_per_s: number;
  sensor_loss: SensorLoss;
  channels: string[];
}

// --- Phase 4: lighting ---

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export type ElectricalType = 'argb_5v_3pin' | 'rgb_12v_4pin' | 'unknown';
export type EffectClass = 'static' | 'breathing' | 'color_cycle' | 'gradient' | 'off';

export interface LightingZone {
  id: string;
  name: string;
  controller: string;
  connector: string;
  electrical_type: ElectricalType;
  led_count: number | null;
  supported_effects: EffectClass[];
  readback_available: boolean;
  validation: 'unvalidated' | 'validated' | 'unsupported';
  connection: 'connected' | 'disconnected' | 'unknown';
}

export type Effect =
  | { kind: 'static'; color: Rgb }
  | { kind: 'breathing'; color: Rgb; speed: number }
  | { kind: 'color_cycle'; speed: number; brightness: number }
  | { kind: 'gradient'; colors: Rgb[]; speed: number }
  | { kind: 'off' };

// --- Phase 5: profiles ---

export interface Profile {
  schema_version: number;
  id: string;
  name: string;
  description: string;
  platforms: string[];
  required_capabilities: string[];
  layout_assignments: Record<string, string>;
  fan_curve_assignments: Record<string, string>;
  lighting_assignments: Record<string, string>;
  preferred_control_mode: 'automatic' | 'manual';
  launch_at_login: boolean;
  background_operation: boolean;
  created_unix_ms: number;
  updated_unix_ms: number;
}

export interface PlannedChange {
  kind: 'layout' | 'fan_curve' | 'lighting';
  target: string;
  value: string;
  supported: boolean;
  reason: string | null;
}

export interface ApplyPlan {
  platform_compatible: boolean;
  missing_capabilities: string[];
  changes: PlannedChange[];
}

export interface SupportBundle {
  app_version: string;
  os: string;
  adapter_versions: Record<string, string>;
  device_inventory: string[];
  capability_states: Record<string, string>;
  log_tail: string[];
}
