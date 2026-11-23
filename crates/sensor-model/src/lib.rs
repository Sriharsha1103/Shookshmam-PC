//! Normalized, platform-neutral sensor model shared by every provider and the UI.
//!
//! Design rules (see docs/architecture/ARCHITECTURE.md §4 and
//! docs/phase-1/PHASE-1-SPEC.md §4):
//! - Every reading carries source, timestamp, unit, status and device identity.
//! - Missing / unsupported / permission-denied / stale readings are never
//!   converted to zero. Only `Available` and `Stale` readings may carry a value.

use serde::{Deserialize, Serialize};

/// Logical grouping of a sensor, used by the UI to lay out dashboards.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SensorCategory {
    Temperature,
    Utilization,
    Frequency,
    Voltage,
    Power,
    Memory,
    FanSpeed,
    StorageHealth,
}

impl SensorCategory {
    /// Canonical unit the registry normalizes this category to.
    pub fn canonical_unit(self) -> &'static str {
        match self {
            SensorCategory::Temperature => "C",
            SensorCategory::Utilization => "%",
            SensorCategory::Frequency => "MHz",
            SensorCategory::Voltage => "V",
            SensorCategory::Power => "W",
            SensorCategory::Memory => "bytes",
            SensorCategory::FanSpeed => "RPM",
            // Storage health is a mix of SMART fields; it has no single unit.
            SensorCategory::StorageHealth => "",
        }
    }
}

/// Lifecycle/quality status of a single reading.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorStatus {
    /// Fresh, valid reading with a finite value.
    Available,
    /// The hardware/driver does not expose this sensor on this system.
    Unsupported,
    /// Known sensor, but momentarily not readable (e.g. device gone).
    Unavailable,
    /// The sensor exists but the process lacks permission to read it.
    PermissionDenied,
    /// Last value is retained but the provider stopped updating it in time.
    Stale,
    /// The provider hit an error while reading this sensor.
    Error,
}

impl SensorStatus {
    /// Whether a value is meaningful. Stale keeps its last-known value, but it
    /// must never be treated as safe input for future control (Phase 3+).
    pub fn carries_value(self) -> bool {
        matches!(self, SensorStatus::Available | SensorStatus::Stale)
    }
}

/// Role a physical device plays in the system.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Cpu,
    Gpu,
    Motherboard,
    Memory,
    Storage,
    Cooler,
    FanController,
    Display,
    Other,
}

/// Stable description of a device a provider exposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceDescriptor {
    /// Provider-specific but stable-where-possible identity.
    pub id: String,
    /// Human-readable label.
    pub name: String,
    pub kind: DeviceKind,
    /// Provider/driver that owns this device.
    pub source: String,
}

/// A single normalized sensor reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    /// Stable logical identifier, e.g. `gpu.temperature.hotspot`.
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub category: SensorCategory,
    /// Numeric value, or `None` when the reading carries no value.
    pub value: Option<f64>,
    pub unit: String,
    /// Provider/driver identifier that produced this reading.
    pub source: String,
    pub status: SensorStatus,
    /// UTC sample time in Unix milliseconds.
    pub timestamp_unix_ms: i64,
    /// Optional confidence/validity note.
    pub quality_note: Option<String>,
}

impl SensorReading {
    /// Construct an available reading with the category's canonical unit.
    pub fn available(
        id: impl Into<String>,
        device_id: impl Into<String>,
        name: impl Into<String>,
        category: SensorCategory,
        value: f64,
        source: impl Into<String>,
        timestamp_unix_ms: i64,
    ) -> Self {
        SensorReading {
            id: id.into(),
            device_id: device_id.into(),
            name: name.into(),
            category,
            value: Some(value),
            unit: category.canonical_unit().to_string(),
            source: source.into(),
            status: SensorStatus::Available,
            timestamp_unix_ms,
            quality_note: None,
        }
    }

    /// Construct a reading that carries no value (unsupported/unavailable/etc.).
    /// This is how adapters report "no data" without fabricating a zero.
    pub fn without_value(
        id: impl Into<String>,
        device_id: impl Into<String>,
        name: impl Into<String>,
        category: SensorCategory,
        source: impl Into<String>,
        status: SensorStatus,
        timestamp_unix_ms: i64,
    ) -> Self {
        debug_assert!(!status.carries_value(), "use `available` for valued readings");
        SensorReading {
            id: id.into(),
            device_id: device_id.into(),
            name: name.into(),
            category,
            value: None,
            unit: category.canonical_unit().to_string(),
            source: source.into(),
            status,
            timestamp_unix_ms,
            quality_note: None,
        }
    }

    /// Attach a quality note (builder-style).
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.quality_note = Some(note.into());
        self
    }

    /// Validate structural invariants of a reading. Returns the first problem.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() || self.device_id.trim().is_empty() {
            return Err("sensor and device identifiers must not be empty");
        }
        if self.timestamp_unix_ms <= 0 {
            return Err("timestamp must be a positive Unix millisecond value");
        }
        if self.status.carries_value() {
            let value = self.value.ok_or("valued status must have a value")?;
            if !value.is_finite() {
                return Err("sensor value must be finite");
            }
        } else if self.value.is_some() {
            // Guard against silently converting "no data" into a number.
            return Err("non-valued status must not carry a value");
        }
        Ok(())
    }
}

/// A coherent set of readings collected at one moment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// When the snapshot was assembled (UTC Unix ms).
    pub timestamp_unix_ms: i64,
    pub readings: Vec<SensorReading>,
}

impl Snapshot {
    pub fn new(timestamp_unix_ms: i64, readings: Vec<SensorReading>) -> Self {
        Snapshot { timestamp_unix_ms, readings }
    }

    /// Find a reading by its logical id.
    pub fn get(&self, id: &str) -> Option<&SensorReading> {
        self.readings.iter().find(|r| r.id == id)
    }

    /// All readings in a category.
    pub fn by_category(&self, category: SensorCategory) -> impl Iterator<Item = &SensorReading> {
        self.readings.iter().filter(move |r| r.category == category)
    }

    /// Validate every reading and ensure logical ids are unique.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for r in &self.readings {
            r.validate().map_err(|e| format!("reading `{}`: {e}", r.id))?;
            if !seen.insert(r.id.as_str()) {
                return Err(format!("duplicate sensor id `{}`", r.id));
            }
        }
        Ok(())
    }
}

/// Normalize a provider-native unit string to a canonical form.
///
/// Adapters should normalize at their boundary, but this gives the registry a
/// consistent fallback for common aliases.
pub fn canonical_unit(raw: &str) -> String {
    let t = raw.trim();
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "c" | "°c" | "degc" | "celsius" => "C".to_string(),
        "%" | "percent" | "pct" => "%".to_string(),
        "mhz" => "MHz".to_string(),
        "ghz" => "GHz".to_string(),
        "v" | "volt" | "volts" => "V".to_string(),
        "w" | "watt" | "watts" => "W".to_string(),
        "rpm" => "RPM".to_string(),
        "b" | "byte" | "bytes" => "bytes".to_string(),
        _ => t.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(status: SensorStatus, value: Option<f64>) -> SensorReading {
        SensorReading {
            id: "gpu.temperature.hotspot".into(),
            device_id: "gpu0".into(),
            name: "GPU Hotspot".into(),
            category: SensorCategory::Temperature,
            value,
            unit: "C".into(),
            source: "test".into(),
            status,
            timestamp_unix_ms: 1,
            quality_note: None,
        }
    }

    #[test]
    fn available_sensor_requires_finite_value() {
        assert!(sample(SensorStatus::Available, Some(f64::NAN)).validate().is_err());
        assert!(sample(SensorStatus::Available, Some(f64::INFINITY)).validate().is_err());
        assert!(sample(SensorStatus::Available, None).validate().is_err());
        assert!(sample(SensorStatus::Available, Some(42.0)).validate().is_ok());
    }

    #[test]
    fn non_valued_status_must_not_carry_value() {
        // A zero sneaking in on an unavailable sensor is exactly what we forbid.
        assert!(sample(SensorStatus::Unavailable, Some(0.0)).validate().is_err());
        assert!(sample(SensorStatus::Unsupported, None).validate().is_ok());
        assert!(sample(SensorStatus::PermissionDenied, None).validate().is_ok());
    }

    #[test]
    fn stale_keeps_last_value() {
        assert!(sample(SensorStatus::Stale, Some(55.0)).validate().is_ok());
        assert!(SensorStatus::Stale.carries_value());
    }

    #[test]
    fn rejects_empty_ids_and_bad_timestamp() {
        let mut r = sample(SensorStatus::Available, Some(1.0));
        r.id = "  ".into();
        assert!(r.validate().is_err());
        let mut r = sample(SensorStatus::Available, Some(1.0));
        r.timestamp_unix_ms = 0;
        assert!(r.validate().is_err());
    }

    #[test]
    fn constructors_set_canonical_unit() {
        let r = SensorReading::available("cpu.freq", "cpu0", "CPU", SensorCategory::Frequency, 3600.0, "sim", 10);
        assert_eq!(r.unit, "MHz");
        assert_eq!(r.status, SensorStatus::Available);
        let r = SensorReading::without_value("cpu.temp", "cpu0", "CPU", SensorCategory::Temperature, "sim", SensorStatus::Unsupported, 10);
        assert_eq!(r.value, None);
        assert_eq!(r.unit, "C");
    }

    #[test]
    fn snapshot_detects_duplicate_ids() {
        let a = SensorReading::available("x", "d", "X", SensorCategory::Power, 1.0, "s", 5);
        let b = SensorReading::available("x", "d", "X2", SensorCategory::Power, 2.0, "s", 5);
        let snap = Snapshot::new(5, vec![a, b]);
        assert!(snap.validate().is_err());
    }

    #[test]
    fn snapshot_lookups() {
        let a = SensorReading::available("cpu.temp", "cpu0", "CPU", SensorCategory::Temperature, 50.0, "s", 5);
        let b = SensorReading::available("cpu.util", "cpu0", "CPU", SensorCategory::Utilization, 10.0, "s", 5);
        let snap = Snapshot::new(5, vec![a, b]);
        assert_eq!(snap.get("cpu.temp").unwrap().value, Some(50.0));
        assert_eq!(snap.by_category(SensorCategory::Temperature).count(), 1);
    }

    #[test]
    fn unit_canonicalization() {
        assert_eq!(canonical_unit("°C"), "C");
        assert_eq!(canonical_unit(" celsius "), "C");
        assert_eq!(canonical_unit("RPM"), "RPM");
        assert_eq!(canonical_unit("percent"), "%");
        assert_eq!(canonical_unit("unknownunit"), "unknownunit");
    }
}
