//! Cross-crate integration tests (P1-11): the normalized sensor contract, the
//! simulation pipeline, and the freshness + history round trip.

use shookshmam_hardware_core::{mock::SimulationProvider, now_unix_ms, Registry};
use shookshmam_sensor_model::{SensorStatus, Snapshot};
use shookshmam_telemetry::{HistoryStore, MemoryHistory, TelemetryTracker};

fn load_fixture() -> Snapshot {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/sample-snapshot.json");
    let raw = std::fs::read_to_string(path).expect("fixture file should exist");
    serde_json::from_str(&raw).expect("fixture should deserialize into a Snapshot")
}

#[test]
fn fixture_matches_normalized_contract() {
    let snap = load_fixture();
    assert!(snap.validate().is_ok(), "fixture must satisfy the sensor contract");

    // Non-available readings must not carry a value (no zero masquerading as data).
    let fan = snap.get("gpu.fan.rpm").unwrap();
    assert_eq!(fan.status, SensorStatus::Unsupported);
    assert_eq!(fan.value, None);

    let vrm = snap.get("mobo.vrm.temp").unwrap();
    assert_eq!(vrm.status, SensorStatus::PermissionDenied);
    assert_eq!(vrm.value, None);

    // A stale reading keeps its last-known value.
    let gpu = snap.get("gpu.temperature.hotspot").unwrap();
    assert_eq!(gpu.status, SensorStatus::Stale);
    assert_eq!(gpu.value, Some(61.0));
}

#[test]
fn simulation_pipeline_produces_valid_snapshot() {
    let mut reg = Registry::new();
    reg.register(Box::new(SimulationProvider::new()));
    let result = reg.collect(1_700_000_000_000);

    assert!(result.snapshot.validate().is_ok());
    assert!(result.providers.iter().all(|p| p.ok));
    assert!(result.snapshot.readings.iter().all(|r| r.source == "simulation"));
    // The simulated GPU deliberately exposes an unsupported fan sensor.
    assert!(result
        .snapshot
        .readings
        .iter()
        .any(|r| r.status == SensorStatus::Unsupported));
}

#[test]
fn freshness_and_history_round_trip() {
    let mut reg = Registry::new();
    reg.register(Box::new(SimulationProvider::new()));
    let mut tracker = TelemetryTracker::new(3000, 9000);
    let mut history = MemoryHistory::new();

    // The simulation provider stamps readings with the wall clock, so drive the
    // test from real "now" and query a generous window around it.
    let base = now_unix_ms();
    for now in [base, base + 1000] {
        let collected = reg.collect(now);
        let merged = tracker.ingest(collected.snapshot, now);
        history.insert_snapshot(&merged).unwrap();
    }

    let series = history
        .query("cpu.temperature.tctl", 0, base + 60_000)
        .unwrap();
    assert_eq!(series.len(), 2, "two samples should be persisted");
    assert!(series[0].timestamp_unix_ms <= series[1].timestamp_unix_ms);
}

#[test]
fn history_retention_removes_only_expired() {
    let mut history = MemoryHistory::new();
    let day = 24 * 60 * 60 * 1000i64;
    let snap_at = |ts: i64| {
        let r = shookshmam_sensor_model::SensorReading::available(
            "cpu.temp",
            "cpu0",
            "CPU",
            shookshmam_sensor_model::SensorCategory::Temperature,
            50.0,
            "sim",
            ts,
        );
        Snapshot::new(ts, vec![r])
    };
    history.insert_snapshot(&snap_at(day)).unwrap();
    history.insert_snapshot(&snap_at(40 * day)).unwrap();

    let cutoff = shookshmam_telemetry::retention_cutoff_ms(45 * day, 30);
    let removed = history.prune(cutoff).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(history.count().unwrap(), 1);
}
