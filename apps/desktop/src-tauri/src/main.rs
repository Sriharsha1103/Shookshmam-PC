#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_core;

use std::thread;
use std::time::Duration;

use app_core::{open_history, AppState, Core};
use shookshmam_hardware_core::now_unix_ms;
use tauri::{Emitter, Manager};

/// Event name the frontend subscribes to for live snapshots.
const SNAPSHOT_EVENT: &str = "telemetry:snapshot";

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Persist history under the OS app-data directory; fall back to
            // in-memory if unavailable so the app still starts.
            let data_dir = app.path().app_data_dir().ok();
            let history = open_history(data_dir.clone());

            let mut core = Core::build(history);
            // Export a flat telemetry file for external widgets (Rainmeter skin).
            if let Some(dir) = &data_dir {
                let _ = std::fs::create_dir_all(dir);
                core.set_export_path(dir.join("telemetry.txt"));
            }
            let interval = core.interval_ms().max(250);
            let handle = app.handle().clone();

            // Prime the first snapshot so the UI has data on load.
            let first = core.poll_once(now_unix_ms());
            let _ = handle.emit(SNAPSHOT_EVENT, first);

            let state = AppState::new(core);
            app.manage(state.clone());

            // Background polling loop. Telemetry collection is isolated from the
            // UI thread (PHASE-1-SPEC §5: UI must not block collection).
            thread::spawn(move || loop {
                thread::sleep(Duration::from_millis(interval));
                let now = now_unix_ms();
                let snapshot = match state.0.lock() {
                    Ok(mut core) => core.poll_once(now),
                    Err(_) => continue, // a poisoned lock must not crash the app
                };
                let _ = handle.emit(SNAPSHOT_EVENT, snapshot);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_core::get_snapshot,
            app_core::get_devices,
            app_core::get_diagnostics,
            app_core::get_config,
            app_core::get_history,
            app_core::get_fan_channels,
            app_core::get_fan_curves,
            app_core::get_lighting_zones,
            app_core::preview_effect,
            app_core::list_profiles,
            app_core::preview_profile,
            app_core::get_support_bundle,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Shookshmam PC");
}
