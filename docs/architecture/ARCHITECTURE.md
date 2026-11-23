# Shookshmam PC — System Architecture

## 1. Goals

Provide one Windows/Bazzite application for hardware telemetry, multiple LCD dashboards, cooling control and ARGB control. The system must isolate OS/device protocols from the UI and common domain logic.

## 2. Non-goals for Phase 1

- Writing fan PWM, pump speed, GPU fan, RGB or ARGB settings.
- Guaranteeing support for every sensor exposed by every motherboard/driver.
- Replacing BIOS/UEFI fan safety controls.
- Cloud accounts, remote control or telemetry upload.

## 3. Layered architecture

```text
React + TypeScript UI
        │ typed Tauri commands/events
Tauri 2 desktop shell
        │ narrow IPC boundary
Rust application services
  ├── Sensor registry and discovery
  ├── Telemetry scheduler and freshness monitor
  ├── History service (30-day retention)
  ├── LCD layout/rendering service
  ├── Device capability registry
  └── Safety/policy layer
        │ adapter traits
Platform/device adapters
  ├── Windows sensors
  ├── Linux hwmon / AMDGPU
  ├── Thermalright LCD transport
  ├── Lian Li controller (later phase)
  ├── Motherboard PWM (later phase)
  └── ARGB providers (later phase)
```

## 4. Architectural rules

1. UI code never accesses hardware directly.
2. Hardware writes require explicit capabilities, permissions and safety policy; none are enabled in Phase 1.
3. Every reading carries source, timestamp, unit, quality/status and device identity.
4. Missing/permission-denied/stale readings are not converted to zero.
5. Device adapters expose capabilities; UI features are enabled only when a capability is detected and validated.
6. A single process owns a device interface at a time; avoid competing with vendor tools or other daemons.
7. The same dashboard definition can be rendered at different resolutions, but each physical display has its own layout/profile assignment.
8. Hardware-specific code is isolated behind adapter traits and may be replaced without changing UI/domain models.

## 5. Rust domain interfaces (target)

- `SensorProvider`: discover sensors and read snapshots.
- `DeviceProvider`: enumerate devices and capabilities.
- `DisplayTransport`: enumerate displays, write frames, report transport health.
- `FanController`: later phase; read tachometer and request bounded duty only after validation.
- `LightingController`: later phase; enumerate zones and apply validated effects.

All interfaces must return typed errors. Device identifiers must be stable where possible but must not assume USB enumeration order is stable.

## 6. Data flow

1. Discovery enumerates providers and devices.
2. Providers publish sensor descriptors with units and supported ranges when known.
3. Scheduler polls according to per-provider limits (initial default target: 1 second for ordinary sensors).
4. Validation checks finite values, timestamps, unit consistency and plausible ranges.
5. Registry publishes snapshots to UI and history.
6. History stores normalized samples and applies 30-day retention.
7. LCD renderer consumes snapshots and a display-specific layout, then sends a frame through the display transport.

## 7. Storage

SQLite is the planned local history store. Keep current device inventory/configuration separate from time-series samples. Store UTC timestamps, sensor ID, numeric value, unit and quality. Retain 30 days by default; pruning runs in bounded batches. No telemetry leaves the machine.

## 8. Security and permissions

- Linux: prefer read access to kernel sensor interfaces; use narrowly scoped udev rules for supported USB devices. Do not run the entire UI as root.
- Windows: sensor backend may require elevated access for some providers; request elevation only for the helper/provider that needs it.
- Never execute shell commands assembled from UI-provided strings.
- Validate all device commands at the adapter boundary.
- Log device errors without recording personal paths or unrelated system data.

## 9. Failure handling

- Provider crash: mark its sensors unavailable and restart/reconnect with bounded backoff.
- Stale data: preserve last value but mark stale; never treat it as safe input for future fan control.
- LCD disconnect: stop frame writes, mark display disconnected, retry with backoff.
- Application crash: Phase 1 makes no hardware control changes; BIOS/firmware remains authoritative.
