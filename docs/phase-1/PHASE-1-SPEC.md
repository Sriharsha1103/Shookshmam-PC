# Phase 1 Technical Specification — Monitoring Foundation

## 1. Objective

Deliver a read-only, cross-platform monitoring foundation for Windows and Bazzite Linux, a React dashboard, local 30-day history, and Thermalright Trofeo Vision LCD output once the physical unit is available for validation.

## 2. Scope

### Included
- Application shell and Rust core.
- Sensor/device discovery and normalized sensor model.
- Windows and Linux read-only monitoring adapters.
- CPU, GPU, memory, motherboard, storage and exposed fan telemetry.
- Live dashboard and sensor status/freshness.
- SQLite history with 30-day default retention.
- Per-display dashboard layout configuration.
- Thermalright LCD transport investigation and integration after device arrival.
- Diagnostics and exportable support bundle.

### Excluded
- Fan PWM writes, pump writes, GPU fan writes.
- ARGB/RGB writes.
- Lian Li TL LCD integration beyond architecture/interface preparation.
- Automatic fan curves and manual fan override implementation.
- Cloud sync or remote access.

## 3. Target hardware profile

- CPU: AMD Ryzen 5 5600G
- GPU: ASRock Radeon RX 9070 16GB
- Motherboard: ASUS Prime B550M-A WiFi II
- RAM: 32GB Corsair Vengeance DDR4
- Storage: Crucial P5 Plus 1TB and Crucial P3 1TB
- Cooling: Arctic Liquid Freezer III 360 ARGB
- Case fans/controller: Lian Li Uni Fan TL LCD triple pack and Lian Li controller
- Additional display: Thermalright Trofeo Vision 6.86-inch, 1280×480 (ordered; validate actual USB identity when received)

Hardware profile is a test target, not a guarantee that every sensor is exposed on both operating systems.

## 4. Sensor contract

Each sensor record must include:
- `id`: stable logical identifier, e.g. `gpu.temperature.hotspot`
- `device_id`: provider-specific device identity
- `name`: human-readable label
- `category`: temperature, utilization, frequency, voltage, power, memory, fan_speed, storage_health
- `value`: numeric value or null
- `unit`: canonical unit (C, %, MHz, W, V, RPM, bytes, etc.)
- `source`: provider/driver identifier
- `status`: available, unsupported, unavailable, permission_denied, stale, error
- `timestamp`: UTC sample time
- `quality`: optional confidence/validity notes

Do not silently convert unavailable sensors to zero. Preserve provider-native readings and normalize units at the adapter boundary.

## 5. Initial polling policy

- Default target interval: 1000 ms for ordinary telemetry.
- Each provider can impose a slower minimum interval.
- USB display frame updates are separately rate-limited and must not drive sensor polling.
- Avoid high-frequency SSD SMART polling.
- UI rendering must not block sensor collection.
- Use bounded in-memory buffers; history writes are batched.

## 6. Initial sensor groups

- CPU: package/Tctl temperature where exposed, utilization, frequency.
- GPU: edge temperature, hotspot, utilization, core/memory clocks, VRAM use, fan RPM where exposed.
- Memory: used/available/total system memory; DIMM temperature only if exposed.
- Motherboard: available board/VRM temperatures and fan tachometers.
- Storage: temperature and health/SMART fields where available.
- Cooling: exposed pump and fan tachometers only; no control writes.

## 7. History

- SQLite local database.
- Default retention: 30 days.
- UTC timestamps.
- Store samples with sensor ID, device ID, value, unit, status and source.
- Batch inserts; indexes on timestamp and sensor ID.
- Prune in bounded batches; expose last successful prune time.
- No network transmission.

## 8. LCD requirements

- Independent layout/profile per display.
- Thermalright target: 1280×480; confirm actual USB VID/PID, endpoints and protocol against the delivered unit.
- Renderer produces a display-sized frame from normalized telemetry.
- Static test pattern before live dashboard.
- Reconnect handling with bounded backoff.
- LCD transport errors must not affect sensor polling or application UI.
- No claim of Linux/Windows support until both are tested on the actual display.

## 9. Platform implementation

### Windows
- Sensor backend behind `SensorProvider` interface.
- Evaluate LibreHardwareMonitor integration and its licensing/redistribution constraints before bundling.
- Keep any privileged/helper process narrow and separately permissioned.

### Bazzite Linux
- Read kernel `hwmon` and AMDGPU sysfs interfaces where available.
- Use standard system telemetry interfaces; avoid requiring root for the whole application.
- Document narrowly scoped permissions/udev rules for USB LCD access.
- Do not assume immutable-host package installation; package the app and helper appropriately for Bazzite.

## 10. Quality and safety requirements

- Phase 1 is strictly read-only with respect to hardware controls.
- No PWM, pump, GPU fan or RGB/ARGB writes.
- Stale or missing sensor data must be explicit.
- Provider errors must be isolated; one failed provider cannot crash the core.
- Diagnostics must include provider/device status and app logs, not sensitive user files.
- All parsing and USB frame operations must be bounded and validated.

## 11. Acceptance criteria

1. Application starts on Windows and Bazzite x86_64.
2. UI remains responsive during sensor polling.
3. Detected sensors show value, unit, source and timestamp.
4. Unsupported/unavailable/permission-denied sensors are clearly represented.
5. Default sampling target is 1 second where provider permits.
6. History persists locally and 30-day retention pruning is tested.
7. LCD static pattern and telemetry frame work on the actual Thermalright device, or integration is explicitly marked blocked with diagnostics if protocol/hardware differs.
8. USB disconnect/reconnect does not crash the app.
9. No hardware-control writes occur in Phase 1.
10. Windows/Linux tests cover normalized sensor contract and error cases.

## 12. Work breakdown

- P1-01: workspace, linting, CI and app shell
- P1-02: sensor model and provider traits
- P1-03: discovery registry and diagnostics
- P1-04: Windows read-only provider
- P1-05: Linux read-only provider
- P1-06: telemetry scheduler and freshness handling
- P1-07: SQLite history and 30-day retention
- P1-08: React monitoring dashboard
- P1-09: Thermalright device identification and static frame
- P1-10: Thermalright live dashboard renderer
- P1-11: Windows/Bazzite integration tests and acceptance report

## 13. Open validation items

- Exact Thermalright USB identity and protocol on delivered device.
- Which motherboard sensors are exposed by the current ASUS firmware/kernel/Windows provider.
- Whether pump tachometer is visible and which header reports it.
- RX 9070 sensor availability parity between Windows and Linux.
- Licensing and packaging approach for each third-party sensor/LCD dependency.
