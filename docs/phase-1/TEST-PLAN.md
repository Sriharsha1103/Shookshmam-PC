# Phase 1 Test Plan

## Unit tests
- Sensor ID uniqueness and descriptor validation.
- Unit conversion and canonical unit handling.
- Status transitions: available → stale → unavailable → available.
- Timestamp ordering and future timestamp rejection.
- Ring buffer bounds and sampling interval enforcement.
- SQLite insert/query/retention pruning.
- Display layout scaling and frame bounds.

## Adapter contract tests
Every adapter must pass the same contract suite:
- Discover zero, one and multiple devices.
- Return unsupported sensors explicitly.
- Handle permission denied and transient I/O errors.
- Never emit NaN/infinite values as valid readings.
- Mark samples stale when provider data stops updating.

## Integration tests
- Windows provider discovery and snapshot collection.
- Linux hwmon/AMDGPU discovery and snapshot collection.
- History survives application restart.
- 30-day retention removes only expired samples.
- LCD unplug/replug recovery.
- LCD write failure does not interrupt telemetry.

## Hardware-in-the-loop tests
- Compare CPU/GPU readings against a trusted independent utility.
- Check GPU edge/hotspot/VRAM values during idle and a repeatable game workload.
- Check SSD temperatures under idle and load without excessive SMART polling.
- Verify each reported fan RPM against firmware/vendor utility.
- Validate LCD static test pattern, orientation, color and update stability.

## Safety regression
- Assert Phase 1 contains no PWM/pump/GPU-fan/ARGB write path.
- Assert missing/stale temperatures cannot be interpreted as 0°C.
- Assert malformed LCD frame dimensions are rejected.
