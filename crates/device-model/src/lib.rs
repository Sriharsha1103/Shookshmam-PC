//! Rich device + capability model shared by Phase 2–5 device adapters.
//!
//! Core safety rule (PHASE-2-SPEC §3): an unknown device revision, or any
//! capability that has not been explicitly validated, defaults to **read-only**.
//! A write capability is usable only once its validation is `Validated`.

use serde::{Deserialize, Serialize};

/// Kind of device within a controller topology.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Controller,
    Fan,
    Lcd,
    LightingZone,
    Pump,
    Other,
}

/// A thing a device can do. Read capabilities are always safe; write
/// capabilities are gated by [`ValidationStatus`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    FanRpmRead,
    PumpRpmRead,
    FanDutyWrite,
    LcdFrameWrite,
    LcdBrightnessWrite,
    RgbEffectWrite,
}

impl Capability {
    /// Whether this capability writes to hardware (and therefore must be
    /// validated and policy-gated before use).
    pub fn is_write(self) -> bool {
        matches!(
            self,
            Capability::FanDutyWrite
                | Capability::LcdFrameWrite
                | Capability::LcdBrightnessWrite
                | Capability::RgbEffectWrite
        )
    }
}

/// Validation lifecycle of a capability on a specific device revision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// Not yet validated on this hardware; writes are refused (safe default).
    #[default]
    Unvalidated,
    /// Validated against the real device; writes are permitted (still policy-gated).
    Validated,
    /// Known not to work / not present on this revision.
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Unknown,
}

/// One capability and its current validation state on a device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityState {
    pub capability: Capability,
    pub validation: ValidationStatus,
}

impl CapabilityState {
    pub fn new(capability: Capability, validation: ValidationStatus) -> Self {
        CapabilityState { capability, validation }
    }

    /// A read capability (always safe) or a validated write capability.
    pub fn is_usable(&self) -> bool {
        if self.capability.is_write() {
            self.validation == ValidationStatus::Validated
        } else {
            self.validation != ValidationStatus::Unsupported
        }
    }
}

/// A discovered device with identity, connection state and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceRecord {
    /// Stable logical ID derived from controller + port identity (see
    /// [`logical_device_id`]), not raw enumeration order.
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub revision: Option<String>,
    pub firmware: Option<String>,
    pub connection: ConnectionState,
    pub last_seen_unix_ms: i64,
    pub provider: String,
    pub capabilities: Vec<CapabilityState>,
}

impl DeviceRecord {
    /// A device with all capabilities unvalidated (safe, read-only) default.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        device_type: DeviceType,
        provider: impl Into<String>,
        last_seen_unix_ms: i64,
    ) -> Self {
        DeviceRecord {
            id: id.into(),
            name: name.into(),
            device_type,
            vendor: None,
            product: None,
            revision: None,
            firmware: None,
            connection: ConnectionState::Unknown,
            last_seen_unix_ms,
            provider: provider.into(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_capability(mut self, capability: Capability, validation: ValidationStatus) -> Self {
        self.capabilities.push(CapabilityState::new(capability, validation));
        self
    }

    pub fn capability(&self, capability: Capability) -> Option<&CapabilityState> {
        self.capabilities.iter().find(|c| c.capability == capability)
    }

    /// Whether a hardware write for `capability` is currently allowed. False
    /// unless the device is connected, the capability is present and its
    /// validation is `Validated`.
    pub fn can_write(&self, capability: Capability) -> bool {
        if !capability.is_write() {
            return false;
        }
        self.connection == ConnectionState::Connected
            && self
                .capability(capability)
                .map(|c| c.validation == ValidationStatus::Validated)
                .unwrap_or(false)
    }

    /// Whether a read for `capability` is available.
    pub fn can_read(&self, capability: Capability) -> bool {
        if capability.is_write() {
            return false;
        }
        self.capability(capability).map(|c| c.is_usable()).unwrap_or(false)
    }
}

/// Build a stable logical device ID from a controller identity and a port.
/// Deterministic and independent of USB enumeration order (PHASE-2-SPEC §3).
pub fn logical_device_id(controller_id: &str, port: u8) -> String {
    format!("lianli:{}:port{}", controller_id.trim(), port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_capability_defaults_read_only() {
        // No capability entry at all -> no write, no read.
        let dev = DeviceRecord::new("d", "Dev", DeviceType::Fan, "p", 1);
        assert!(!dev.can_write(Capability::FanDutyWrite));
        assert!(!dev.can_read(Capability::FanRpmRead));
    }

    #[test]
    fn write_requires_validated_and_connected() {
        let dev = DeviceRecord::new("d", "Fan", DeviceType::Fan, "p", 1)
            .with_capability(Capability::FanDutyWrite, ValidationStatus::Validated);
        // Validated but Unknown connection -> still refused.
        assert!(!dev.can_write(Capability::FanDutyWrite));

        let mut connected = dev.clone();
        connected.connection = ConnectionState::Connected;
        assert!(connected.can_write(Capability::FanDutyWrite));
    }

    #[test]
    fn unvalidated_write_is_refused_even_when_connected() {
        let mut dev = DeviceRecord::new("d", "Fan", DeviceType::Fan, "p", 1)
            .with_capability(Capability::FanDutyWrite, ValidationStatus::Unvalidated);
        dev.connection = ConnectionState::Connected;
        assert!(!dev.can_write(Capability::FanDutyWrite));
    }

    #[test]
    fn read_capability_usable_without_validation() {
        let dev = DeviceRecord::new("d", "Fan", DeviceType::Fan, "p", 1)
            .with_capability(Capability::FanRpmRead, ValidationStatus::Unvalidated);
        assert!(dev.can_read(Capability::FanRpmRead));
    }

    #[test]
    fn logical_ids_are_stable_and_distinct() {
        assert_eq!(logical_device_id("ABC123", 2), "lianli:ABC123:port2");
        assert_ne!(logical_device_id("ABC123", 1), logical_device_id("ABC123", 2));
    }

    #[test]
    fn write_capability_classification() {
        assert!(Capability::FanDutyWrite.is_write());
        assert!(Capability::RgbEffectWrite.is_write());
        assert!(!Capability::FanRpmRead.is_write());
    }
}
