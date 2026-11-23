//! Deterministic simulation provider.
//!
//! This is **not** a hardware reading. Every value it produces is synthetic and
//! labeled with `source = "simulation"` and a quality note, so the UI can show a
//! clear "simulated data" banner. It exists so the full pipeline (registry →
//! telemetry → history → dashboard → LCD renderer) can be developed and tested
//! on any machine before the real OS/device adapters are validated.

use crate::provider::{HardwareError, SensorProvider};
use shookshmam_sensor_model::{
    DeviceDescriptor, DeviceKind, SensorCategory, SensorReading, SensorStatus,
};

pub const SOURCE: &str = "simulation";

/// A provider that emits plausible, deterministic telemetry derived from the
/// collection timestamp.
pub struct SimulationProvider;

impl SimulationProvider {
    pub fn new() -> Self {
        SimulationProvider
    }
}

impl Default for SimulationProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic oscillation in `[min, max]` as a function of time, using a
/// cheap triangle wave so no floating-point RNG or external crate is needed.
fn osc(ts_ms: i64, period_ms: i64, min: f64, max: f64, phase_ms: i64) -> f64 {
    let p = period_ms.max(1);
    let t = (ts_ms + phase_ms).rem_euclid(p) as f64 / p as f64; // 0.0..1.0
    let tri = if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 }; // 0→1→0
    min + (max - min) * tri
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

impl SensorProvider for SimulationProvider {
    fn provider_id(&self) -> &'static str {
        SOURCE
    }

    fn discover(&self) -> Result<Vec<DeviceDescriptor>, HardwareError> {
        Ok(vec![
            dev("cpu0", "AMD Ryzen 5 5600G (simulated)", DeviceKind::Cpu),
            dev("gpu0", "Radeon RX 9070 (simulated)", DeviceKind::Gpu),
            dev("mem0", "System Memory (simulated)", DeviceKind::Memory),
            dev("nvme0", "Crucial P5 Plus (simulated)", DeviceKind::Storage),
        ])
    }

    fn read_snapshot(&self, device_id: &str) -> Result<Vec<SensorReading>, HardwareError> {
        // The timestamp drives the deterministic values; the registry passes
        // "now" implicitly via the snapshot, so we read the clock here.
        let ts = crate::now_unix_ms();
        let note = "simulated data — not a real hardware reading";

        let mk = |id: &str, name: &str, cat: SensorCategory, val: f64| {
            SensorReading::available(id, device_id, name, cat, round1(val), SOURCE, ts)
                .with_note(note)
        };

        let readings = match device_id {
            "cpu0" => vec![
                mk("cpu.temperature.tctl", "CPU Tctl", SensorCategory::Temperature, osc(ts, 7000, 38.0, 72.0, 0)),
                mk("cpu.utilization.total", "CPU Usage", SensorCategory::Utilization, osc(ts, 4000, 3.0, 85.0, 1000)),
                mk("cpu.frequency.core", "CPU Clock", SensorCategory::Frequency, osc(ts, 4000, 2200.0, 4400.0, 1000)),
                mk("cpu.power.package", "CPU Package Power", SensorCategory::Power, osc(ts, 6000, 15.0, 65.0, 500)),
            ],
            "gpu0" => vec![
                mk("gpu.temperature.edge", "GPU Edge", SensorCategory::Temperature, osc(ts, 9000, 40.0, 68.0, 0)),
                mk("gpu.temperature.hotspot", "GPU Hotspot", SensorCategory::Temperature, osc(ts, 9000, 45.0, 85.0, 300)),
                mk("gpu.utilization.core", "GPU Usage", SensorCategory::Utilization, osc(ts, 5000, 0.0, 99.0, 0)),
                mk("gpu.frequency.core", "GPU Core Clock", SensorCategory::Frequency, osc(ts, 5000, 500.0, 2800.0, 0)),
                mk("gpu.memory.used", "VRAM Used", SensorCategory::Memory, osc(ts, 12000, 2.0e9, 14.0e9, 0)),
                // Exercise the "unsupported" path explicitly.
                SensorReading::without_value(
                    "gpu.fan.rpm",
                    device_id,
                    "GPU Fan",
                    SensorCategory::FanSpeed,
                    SOURCE,
                    SensorStatus::Unsupported,
                    ts,
                )
                .with_note("fan tach not exposed by simulated GPU"),
            ],
            "mem0" => vec![
                mk("mem.used", "Memory Used", SensorCategory::Memory, osc(ts, 15000, 6.0e9, 24.0e9, 0)),
                mk("mem.total", "Memory Total", SensorCategory::Memory, 32.0e9),
            ],
            "nvme0" => vec![
                mk("nvme0.temperature", "SSD Temperature", SensorCategory::Temperature, osc(ts, 20000, 32.0, 55.0, 0)),
            ],
            _ => return Err(HardwareError::DeviceUnavailable),
        };
        Ok(readings)
    }
}

fn dev(id: &str, name: &str, kind: DeviceKind) -> DeviceDescriptor {
    DeviceDescriptor { id: id.into(), name: name.into(), kind, source: SOURCE.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_simulated_devices() {
        let p = SimulationProvider::new();
        let devices = p.discover().unwrap();
        assert_eq!(devices.len(), 4);
        assert!(devices.iter().all(|d| d.source == SOURCE));
    }

    #[test]
    fn readings_validate_and_are_labeled() {
        let p = SimulationProvider::new();
        for d in p.discover().unwrap() {
            for r in p.read_snapshot(&d.id).unwrap() {
                assert!(r.validate().is_ok(), "{} failed validation", r.id);
                assert_eq!(r.source, SOURCE);
            }
        }
    }

    #[test]
    fn unknown_device_is_unavailable() {
        let p = SimulationProvider::new();
        assert!(matches!(p.read_snapshot("nope"), Err(HardwareError::DeviceUnavailable)));
    }

    #[test]
    fn oscillation_stays_in_bounds() {
        for ts in (0..30_000).step_by(137) {
            let v = osc(ts, 7000, 38.0, 72.0, 0);
            assert!((38.0..=72.0).contains(&v), "v={v} out of bounds at ts={ts}");
        }
    }
}
