# Phase 3 Technical Specification — Fan and Cooling Control

**Status:** Planned  
**Depends on:** Phase 1 accepted; Phase 2 device capabilities validated where applicable  
**Safety classification:** Hardware-control phase; gated rollout

## 1. Objective

Provide safe automatic temperature-based fan curves and manual override for supported motherboard headers and validated Lian Li fan channels. The application must fail safely and must never become the only protection against overheating.

## 2. Scope

### Included
- Read current fan RPM and controllable channel capabilities.
- Discover motherboard PWM/DC headers and supported Lian Li fan channels.
- Create, edit, import/export and assign fan curves.
- Automatic curve mode with selectable validated temperature source(s).
- Manual override with bounded duty and explicit duration/persistence choice.
- Per-channel minimum/maximum duty constraints and slew/ramp limits.
- Fan stop only when hardware and cooling design explicitly support it; disabled by default.
- Control ownership, watchdog, failure handling and event audit log.
- Profiles: Quiet, Balanced, Performance and Custom as initial presets, with user-editable values.

### Excluded
- GPU fan control in the initial release unless a separately validated adapter is approved.
- AIO pump speed control in the initial release; pump remains on its existing safe motherboard/firmware control path.
- Automatic BIOS/UEFI configuration changes.
- Overclocking, voltage changes or power-limit changes.

## 3. Supported initial control targets

- ASUS Prime B550M-A WiFi II motherboard fan headers, only after exact header mapping and PWM/DC mode are verified.
- Lian Li TL fan channels, only if controller protocol supports reliable bounded control and RPM feedback.
- Arctic Liquid Freezer III 360 pump/fan wiring must be mapped physically before any control channel is exposed. Do not assume the pump and radiator fans are independently controllable through software.

## 4. Fan curve model

A curve consists of:
- Stable curve ID and name.
- Temperature source IDs and source-combination policy.
- Ordered points `(temperature_c, duty_percent)`.
- Minimum and maximum duty.
- Response smoothing / hysteresis.
- Ramp-up and ramp-down rate limits.
- Sensor-loss behavior.
- Target fan channel assignments.

Validation rules:
- Temperatures must be finite and strictly increasing.
- Duty values must be within the hardware-adapter's validated range.
- Curve must be monotonic non-decreasing unless a future explicit advanced mode is approved.
- Never apply a curve with missing, stale or invalid temperature input.
- Never silently clamp an unsafe user curve; show the corrected constraints and require confirmation.

## 5. Automatic mode

- Sample validated temperature sources using provider freshness metadata.
- Apply smoothing and hysteresis to prevent rapid oscillation.
- Ramp-up may be faster than ramp-down; limits are configurable within safe bounds.
- If a selected sensor becomes stale/unavailable, switch to the channel's configured safe fallback duty or relinquish control to firmware. The adapter must define which fallback is reliable.
- On app/service shutdown, release control and restore firmware/BIOS ownership where the hardware protocol supports it.

## 6. Manual override

- User selects a channel and duty within validated bounds.
- Default manual override expires after a configurable short period; initial default recommendation: 30 minutes.
- Persistent manual mode requires an explicit confirmation and visible warning.
- Returning to automatic mode must revalidate sensors before control resumes.
- UI must display requested duty and measured RPM separately.

## 7. Safety architecture

- All control commands pass through a central policy engine and adapter-side validation.
- Require explicit user enablement per controller/channel.
- First-use calibration/verification is performed one channel at a time.
- Maintain a watchdog heartbeat while control is active.
- On application crash, service crash, communication loss or sensor loss, execute the adapter's tested safe-release behavior.
- BIOS/UEFI fan curves remain the ultimate fallback; do not disable them.
- No pump or GPU fan control until separate design review and acceptance tests.
- Log every control transition, command, rejection and fallback without logging unrelated personal data.

## 8. UI requirements

- Fan topology view with physical/logical channel mapping.
- Curve editor with draggable points and live sensor overlay.
- Automatic/manual mode indicator per channel.
- Requested duty, actual RPM, selected sensor and sensor freshness.
- Clear warning for unsupported or unverified channels.
- One-click release-to-firmware action where supported.

## 9. Acceptance criteria

1. Every exposed control channel is physically identified and documented.
2. Control is disabled by default after installation and after unclean shutdown until safety checks pass.
3. Valid curves apply within defined update latency without oscillation.
4. Invalid, stale or missing sensor inputs never produce a low-duty command.
5. Manual override respects hardware bounds and expiry.
6. Disconnect/crash tests demonstrate the documented safe fallback.
7. BIOS fan control remains enabled and functional.
8. No pump or GPU fan control is exposed without separate approval.
9. Windows and Bazzite behavior is validated independently for each adapter.

## 10. Work breakdown

- P3-01: physical header/channel inventory and capability schema
- P3-02: control policy engine and typed command model
- P3-03: curve model, validation and persistence
- P3-04: ASUS motherboard fan adapter (read-only verification first)
- P3-05: Lian Li fan control adapter, if validated
- P3-06: automatic curve scheduler and smoothing
- P3-07: manual override and expiry
- P3-08: watchdog, release and fault-injection tests
- P3-09: UI curve editor and channel view
- P3-10: staged hardware validation on Windows and Bazzite
