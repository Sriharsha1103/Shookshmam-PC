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
}
