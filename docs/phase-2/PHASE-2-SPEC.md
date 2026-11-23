# Phase 2 Technical Specification — Lian Li Device Integration

**Status:** Planned  
**Depends on:** Phase 1 accepted  
**Platforms:** Windows 10/11 x86_64 and Bazzite Linux x86_64

## 1. Objective

Integrate the Lian Li Uni Fan TL LCD controller and attached TL LCD fans into Shookshmam PC. Deliver device discovery, read-only fan telemetry where exposed, per-screen LCD layouts and content streaming, and supported RGB control only after the relevant controller protocol is validated. Phase 2 establishes the Lian Li device layer; general motherboard fan curves remain Phase 3.

## 2. Scope

### Included
- Detect the Lian Li TL controller and identify model/firmware/interface capabilities.
- Enumerate attached TL LCD fans/screens and map physical ports to stable logical device IDs.
- Read fan RPM and controller/device status where available.
- Render and stream independent content to each supported TL LCD.
- Support per-screen layout assignment, orientation, brightness where supported, and frame-rate limits.
- Integrate Lian Li RGB capabilities only if protocol and safety tests pass; otherwise expose a read-only capability status and defer writes to Phase 4.
- Windows and Bazzite adapters behind a common Rust trait.
- Device ownership/lock handling to avoid simultaneous access by L-Connect 3, Lian Li Linux or another process.

### Excluded
- Motherboard PWM curve control.
- Pump control and GPU fan control.
- General-purpose ARGB header control.
- Unsupported firmware flashing or controller updates.
- Guaranteeing support for devices/revisions not physically tested.

## 3. Device model

Each discovered device must expose:
- Stable logical ID (derived from controller identity + port/device identity, not enumeration order alone).
- Vendor, product, revision and firmware information when available.
- Device type: controller, fan, LCD, lighting zone.
- Connection state and last-seen timestamp.
- Capabilities: `fan_rpm_read`, `lcd_frame_write`, `lcd_brightness_write`, `rgb_effect_write`, etc.
- Provider name/version and typed diagnostic state.

Unknown revisions must default to safe, read-only behavior.

## 4. LCD requirements

- Each TL LCD is an independently addressable display target.
- Layouts are stored separately from the Thermalright layout.
- Renderer accepts normalized telemetry and emits the exact pixel format/resolution required by the detected device.
- Start with static test patterns, then a low-rate live dashboard.
- Frame updates must be bounded and rate-limited; sensor polling must not depend on display refresh.
- Handle disconnect/reconnect, partial device enumeration and write failures without crashing monitoring.
- Provide a per-display preview in the desktop UI.
- Never assume all three fans have identical orientation or port mapping; require detection or user confirmation.

## 5. Fan telemetry requirements

- Read tachometer values where the TL controller exposes them.
- Record source, timestamp, quality and status using the Phase 1 sensor contract.
- Do not infer fan RPM from requested duty or estimate a value when no tachometer reading exists.
- Mark unsupported or disconnected fans explicitly.

## 6. RGB requirements

- Enumerate lighting zones and supported effects before enabling writes.
- Preserve existing controller state when possible; do not apply a default effect on startup.
- Any RGB write must be explicit, validated and reversible to a saved known state where supported.
- If reliable state readback/restore is unavailable, disclose that limitation and require confirmation before first write.

## 7. Platform adapters

### Windows
- Implement the Lian Li provider using a documented or reviewed protocol/library.
- Detect conflicts with L-Connect 3 and provide actionable ownership diagnostics.
- Avoid installing unsigned kernel drivers.

### Bazzite Linux
- Evaluate reuse/integration of the existing Lian Li Linux project; review its licence and architecture before linking or embedding.
- Prefer a narrowly scoped user service/device permission model; do not run the UI as root.
- Document conflicts with other Lian Li daemons and provide a single-owner policy.

## 8. UI requirements

- Device inventory page showing controller, fans, ports, screens and capabilities.
- Individual LCD editor and live preview.
- Assign a named layout to each display.
- Connection health, firmware/revision and diagnostics.
- Clear distinction between supported, detected-but-unsupported and disconnected hardware.

## 9. Acceptance criteria

1. The actual TL controller is detected on Windows and Bazzite, or unsupported state is accurately diagnosed.
2. Each physically attached TL LCD is mapped to the correct logical screen after reboot and reconnect.
3. Static test image is rendered correctly on every validated screen.
4. Live telemetry layout updates without blocking sensor polling.
5. Fan RPM is reported only when read from a real tachometer source.
6. A controller disconnect/reconnect does not crash the app or corrupt other display state.
7. Competing-process ownership is detected or safely handled.
8. No unsupported firmware or undocumented destructive commands are issued.
9. All validated capabilities have automated tests or repeatable hardware test procedures.

## 10. Work breakdown

- P2-01: protocol/library and licence review
- P2-02: controller identification and capability probing
- P2-03: stable device/port mapping
- P2-04: TL fan telemetry provider
- P2-05: LCD transport and static frame tests
- P2-06: multi-screen layout/rendering integration
- P2-07: optional validated RGB capability
- P2-08: Windows/Bazzite hardware validation and diagnostics
