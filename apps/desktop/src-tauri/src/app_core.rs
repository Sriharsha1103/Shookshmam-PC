//! Application core: owns the provider registry, freshness tracker, history and
//! polling policy behind a single mutex, and exposes typed Tauri commands.
//!
//! Phase 1 is read-only (ADR-0002): there is no command that writes to hardware.

use std::sync::{Arc, Mutex};

use serde::Serialize;
#[cfg(target_os = "linux")]
use shookshmam_adapter_linux::HwmonProvider;
#[cfg(target_os = "windows")]
use shookshmam_adapter_windows::WindowsSensorProvider;
use shookshmam_hardware_core::{
    mock::SimulationProvider, now_unix_ms, CollectResult, Diagnostics, ProviderStatus, Registry,
};
use shookshmam_sensor_model::{DeviceDescriptor, Snapshot};
use shookshmam_telemetry::{
    retention_cutoff_ms, HistoryStore, MemoryHistory, Sample, Scheduler, SqliteHistory,
    TelemetryTracker,
};

use shookshmam_control::{preset as fan_preset, ChannelInfo, FanCurve};
use shookshmam_device_model::Capability;
use shookshmam_lighting::{render, Effect, EffectClass, LightingZone, Rgb};
use shookshmam_profiles::{
    plan as profile_plan, preset as profile_preset, ApplyPlan, DeviceEnvironment, Platform, Profile,
    SupportBundle,
};
use std::collections::HashSet;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Phase 1 configuration surfaced to the UI (defaults mirror config/defaults.toml).
#[derive(Debug, Clone, Serialize)]
pub struct AppConfig {
    pub sample_interval_ms: u64,
    pub history_retention_days: u32,
    pub history_enabled: bool,
    pub read_only: bool,
    /// True when the live data is coming only from the simulation provider.
    pub simulated: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            sample_interval_ms: 1000,
            history_retention_days: 30,
            history_enabled: true,
            read_only: true,
            simulated: true,
        }
    }
}

pub struct Core {
    registry: Registry,
    tracker: TelemetryTracker,
    scheduler: Scheduler,
    history: Box<dyn HistoryStore + Send>,
    config: AppConfig,
    latest: Snapshot,
    last_providers: Vec<ProviderStatus>,
    last_prune_day: i64,
    // Phase 2-5 modeled surfaces (all unvalidated / read-only until HIL).
    fan_channels: Vec<ChannelInfo>,
    fan_curves: Vec<FanCurve>,
    lighting_zones: Vec<LightingZone>,
    profiles: Vec<Profile>,
}

impl Core {
    /// Build the core, registering the simulation provider plus the native
    /// provider for this platform. Native providers currently report
    /// `NotImplemented`, so the simulation drives the UI until they are
    /// validated — every simulated reading is labeled `source = "simulation"`.
    pub fn build(history: Box<dyn HistoryStore + Send>) -> Self {
        let mut registry = Registry::new();
        registry.register(Box::new(SimulationProvider::new()));
        #[cfg(target_os = "linux")]
        registry.register(Box::new(HwmonProvider::new()));
        #[cfg(target_os = "windows")]
        registry.register(Box::new(WindowsSensorProvider::new()));

        let config = AppConfig::default();
        let interval = registry.effective_interval_ms(config.sample_interval_ms);

        Core {
            registry,
            // A reading is stale after ~3 missed intervals, dropped after ~10.
            tracker: TelemetryTracker::new(interval as i64 * 3, interval as i64 * 10),
            scheduler: Scheduler::new(interval),
            history,
            config,
            latest: Snapshot::new(now_unix_ms(), Vec::new()),
            last_providers: Vec::new(),
            last_prune_day: 0,
            fan_channels: model_fan_channels(),
            fan_curves: ["quiet", "balanced", "performance"]
                .into_iter()
                .filter_map(fan_preset)
                .collect(),
            lighting_zones: model_lighting_zones(),
            profiles: ["quiet", "balanced", "performance"]
                .into_iter()
                .filter_map(|n| profile_preset(n, now_unix_ms()))
                .collect(),
        }
    }

    /// Run one collection + freshness + history cycle. Returns the merged
    /// snapshot that should be emitted to the UI.
    pub fn poll_once(&mut self, now_ms: i64) -> Snapshot {
        let CollectResult { snapshot, providers } = self.registry.collect(now_ms);
        self.last_providers = providers;

        let merged = self.tracker.ingest(snapshot, now_ms);

        if self.config.history_enabled {
            let _ = self.history.insert_snapshot(&merged);
            self.maybe_prune(now_ms);
        }

        self.config.simulated = !merged.readings.is_empty()
            && merged.readings.iter().all(|r| r.source == "simulation");
        self.latest = merged.clone();
        merged
    }

    fn maybe_prune(&mut self, now_ms: i64) {
        let day = now_ms / DAY_MS;
        if day != self.last_prune_day {
            let cutoff = retention_cutoff_ms(now_ms, self.config.history_retention_days);
            let _ = self.history.prune(cutoff);
            self.last_prune_day = day;
        }
    }

    pub fn interval_ms(&self) -> u64 {
        self.scheduler.interval_ms()
    }

    pub fn due(&mut self, now_ms: i64) -> bool {
        self.scheduler.due(now_ms)
    }

    fn diagnostics(&self) -> Diagnostics {
        let result = CollectResult {
            snapshot: self.latest.clone(),
            providers: self.last_providers.clone(),
        };
        Diagnostics::from_collect(&result)
    }
}

/// Shared, thread-safe handle to the core.
#[derive(Clone)]
pub struct AppState(pub Arc<Mutex<Core>>);

impl AppState {
    pub fn new(core: Core) -> Self {
        AppState(Arc::new(Mutex::new(core)))
    }
}

/// Open the SQLite history at the app data directory, falling back to in-memory
/// if the path cannot be created/opened so the app still runs.
pub fn open_history(data_dir: Option<std::path::PathBuf>) -> Box<dyn HistoryStore + Send> {
    if let Some(dir) = data_dir {
        if std::fs::create_dir_all(&dir).is_ok() {
            if let Ok(db) = SqliteHistory::open(dir.join("history.db")) {
                return Box::new(db);
            }
        }
    }
    Box::new(MemoryHistory::new())
}

// ---------------------------------------------------------------------------
// Tauri commands (read-only)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_snapshot(state: tauri::State<'_, AppState>) -> Result<Snapshot, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.latest.clone())
}

#[tauri::command]
pub fn get_devices(state: tauri::State<'_, AppState>) -> Result<Vec<DeviceDescriptor>, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.registry.discover_devices())
}

#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, AppState>) -> Result<Diagnostics, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.diagnostics())
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.config.clone())
}

#[tauri::command]
pub fn get_history(
    state: tauri::State<'_, AppState>,
    sensor_id: String,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<Sample>, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    core.history.query(&sensor_id, from_ms, to_ms)
}

// ---------------------------------------------------------------------------
// Phase 2-5 modeled surfaces + preview commands (read-only; no hardware writes)
// ---------------------------------------------------------------------------

fn current_platform() -> Platform {
    if cfg!(target_os = "linux") {
        Platform::BazziteLinux
    } else {
        Platform::Windows
    }
}

/// Representative controllable channels for the target rig. All `validated:
/// false`, so the control layer refuses any duty write.
fn model_fan_channels() -> Vec<ChannelInfo> {
    let ch = |id: &str, label: &str| ChannelInfo {
        id: id.into(),
        label: label.into(),
        min_duty: 20.0,
        max_duty: 100.0,
        allow_stop: false,
        validated: false,
    };
    vec![
        ch("cpu_fan", "CPU / AIO Fan Header"),
        ch("chassis_fan_1", "Chassis Fan 1"),
        ch("lianli_tl", "Lian Li TL Fans"),
    ]
}

/// Representative lighting zones. Electrical type stays `Unknown` (never inferred
/// from software names), so the lighting layer refuses writes.
fn model_lighting_zones() -> Vec<LightingZone> {
    let mut mobo = LightingZone::new("mobo_argb", "Motherboard ARGB header", "asus", "argb_header_1");
    mobo.supported_effects = vec![
        EffectClass::Static,
        EffectClass::Breathing,
        EffectClass::ColorCycle,
        EffectClass::Gradient,
        EffectClass::Off,
    ];
    let mut aio = LightingZone::new("aio_argb", "Arctic AIO ARGB", "arctic", "via_mobo_header");
    aio.supported_effects = vec![EffectClass::Static, EffectClass::ColorCycle, EffectClass::Off];
    vec![mobo, aio]
}

/// A `DeviceEnvironment` reflecting the current (unvalidated) hardware: channels
/// and zones exist, but no write capabilities are validated, so profile previews
/// honestly show control/lighting components as unsupported.
struct PreviewEnv {
    platform: Platform,
    caps: HashSet<Capability>,
    channels: HashSet<String>,
    zones: HashSet<String>,
    displays: HashSet<String>,
}

impl DeviceEnvironment for PreviewEnv {
    fn platform(&self) -> Platform {
        self.platform
    }
    fn has_capability(&self, cap: Capability) -> bool {
        self.caps.contains(&cap)
    }
    fn has_display(&self, id: &str) -> bool {
        self.displays.contains(id)
    }
    fn has_channel(&self, id: &str) -> bool {
        self.channels.contains(id)
    }
    fn has_zone(&self, id: &str) -> bool {
        self.zones.contains(id)
    }
    fn apply_change(
        &mut self,
        _change: &shookshmam_profiles::PlannedChange,
    ) -> Result<(), String> {
        Err("preview environment does not apply hardware changes".into())
    }
    fn rollback(&mut self, _applied: &[shookshmam_profiles::PlannedChange]) {}
}

impl Core {
    fn preview_env(&self) -> PreviewEnv {
        PreviewEnv {
            platform: current_platform(),
            // Nothing is validated yet, so no write capability is advertised.
            caps: HashSet::new(),
            channels: self.fan_channels.iter().map(|c| c.id.clone()).collect(),
            zones: self.lighting_zones.iter().map(|z| z.id.clone()).collect(),
            displays: HashSet::new(),
        }
    }

    fn support_bundle(&self) -> SupportBundle {
        let inventory: Vec<String> = self
            .registry
            .discover_devices()
            .into_iter()
            .map(|d| format!("{} [{}]", d.name, d.source))
            .collect();
        let mut bundle = SupportBundle::new(env!("CARGO_PKG_VERSION"), std::env::consts::OS)
            .with_adapter("simulation", env!("CARGO_PKG_VERSION"))
            .with_inventory(inventory);
        for c in &self.fan_channels {
            let state = if c.validated { "validated" } else { "unvalidated" };
            bundle = bundle.with_capability_state(format!("fan:{}", c.id), state);
        }
        for z in &self.lighting_zones {
            bundle = bundle.with_capability_state(format!("rgb:{}", z.id), "unvalidated");
        }
        bundle
    }
}

#[tauri::command]
pub fn get_fan_channels(state: tauri::State<'_, AppState>) -> Result<Vec<ChannelInfo>, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.fan_channels.clone())
}

#[tauri::command]
pub fn get_fan_curves(state: tauri::State<'_, AppState>) -> Result<Vec<FanCurve>, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.fan_curves.clone())
}

#[tauri::command]
pub fn get_lighting_zones(state: tauri::State<'_, AppState>) -> Result<Vec<LightingZone>, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.lighting_zones.clone())
}

/// Pure, read-only effect preview (the first frame). Never touches hardware.
#[tauri::command]
pub fn preview_effect(effect: Effect, led_count: u32) -> Result<Vec<Rgb>, String> {
    effect.validate().map_err(|e| e.to_string())?;
    Ok(render(&effect, led_count.clamp(1, 256), 0))
}

#[tauri::command]
pub fn list_profiles(state: tauri::State<'_, AppState>) -> Result<Vec<Profile>, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.profiles.clone())
}

/// Preview what applying a profile would change against the current hardware.
/// Read-only: produces the plan, never applies it.
#[tauri::command]
pub fn preview_profile(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<ApplyPlan, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    let env = core.preview_env();
    let profile = core
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("unknown profile `{id}`"))?;
    Ok(profile_plan(profile, &env))
}

#[tauri::command]
pub fn get_support_bundle(state: tauri::State<'_, AppState>) -> Result<SupportBundle, String> {
    let core = state.0.lock().map_err(|e| e.to_string())?;
    Ok(core.support_bundle())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_produces_simulated_snapshot_and_history() {
        let mut core = Core::build(Box::new(MemoryHistory::new()));
        let snap = core.poll_once(now_unix_ms());
        assert!(!snap.readings.is_empty());
        assert!(snap.validate().is_ok());
        assert!(core.config.simulated);
        assert!(core.history.count().unwrap() > 0);
    }

    #[test]
    fn diagnostics_reflect_providers() {
        let mut core = Core::build(Box::new(MemoryHistory::new()));
        core.poll_once(now_unix_ms());
        let d = core.diagnostics();
        assert!(d.total_readings > 0);
        assert!(d.providers.iter().any(|p| p.provider_id == "simulation" && p.ok));
    }

    #[test]
    fn modeled_surfaces_present() {
        let core = Core::build(Box::new(MemoryHistory::new()));
        assert_eq!(core.fan_channels.len(), 3);
        assert_eq!(core.fan_curves.len(), 3);
        assert_eq!(core.lighting_zones.len(), 2);
        assert_eq!(core.profiles.len(), 3);
        assert!(core.fan_channels.iter().all(|c| !c.validated));
    }

    #[test]
    fn effect_preview_renders_requested_count() {
        let out = preview_effect(Effect::Static { color: Rgb::new(1, 2, 3) }, 8).unwrap();
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn profile_preview_marks_control_unsupported() {
        // No validated write capabilities -> fan/lighting assignments unsupported.
        let core = Core::build(Box::new(MemoryHistory::new()));
        let env = core.preview_env();
        let profile = core.profiles.iter().find(|p| p.id == "balanced").unwrap();
        let plan = profile_plan(profile, &env);
        assert!(plan.has_unsupported());
    }
}
