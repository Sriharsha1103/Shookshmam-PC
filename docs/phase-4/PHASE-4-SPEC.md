# Phase 4 Technical Specification — ARGB and Lighting Control

**Status:** Planned  
**Depends on:** Phase 1 device registry; Phase 2 Lian Li controller discovery; independent electrical/header validation

## 1. Objective

Provide unified, per-zone ARGB/RGB lighting configuration for validated motherboard headers and supported Lian Li devices, with reusable effects, profiles and safe ownership management.

## 2. Scope

### Included
- Enumerate supported lighting controllers and zones.
- Identify addressable 5V 3-pin ARGB versus 12V 4-pin RGB interfaces explicitly.
- Configure static colors, brightness, validated built-in effects and per-zone assignments.
- Support Lian Li controller lighting only where protocol and hardware revision are validated.
- Save and restore user lighting profiles where reliable state handling exists.
- Integrate with OpenRGB or reuse compatible protocol work only after licence, compatibility and device-ownership review.
- Windows and Bazzite provider implementations behind a common `LightingController` interface.

### Excluded
- Electrical power switching or voltage conversion.
- Sending 12V RGB commands to 5V ARGB headers or vice versa.
- Firmware flashing.
- Unsupported proprietary effects or undocumented controller writes.
- Per-LED editing unless the controller exposes a validated LED map.

## 3. Initial hardware targets

- ASUS Prime B550M-A WiFi II 5V 3-pin ARGB header and connected hub/chain, after confirming the physical wiring and LED count.
- Lian Li TL controller and TL fan lighting, if exposed by the validated controller adapter.
- Arctic Liquid Freezer III 360 ARGB lighting through its actual motherboard connection. Do not assume it is independently addressable if it shares a header or daisy-chain.

## 4. Lighting zone model

Each zone contains:
- Stable zone ID and friendly name.
- Controller and physical connector identity.
- Electrical type: `argb_5v_3pin`, `rgb_12v_4pin`, or `unknown`.
- LED count and topology where known.
- Supported effects and color formats.
- Current state/readback availability.
- Capability and validation status.

Unknown electrical type or topology must disable writes.

## 5. Effect model

Initial effects:
- Static color.
- Breathing.
- Color cycle.
- Gradient, only where supported.
- Off.

Each effect must have validated parameters: colors, speed, brightness, direction and zone scope. Effects must be adapted to controller capabilities rather than assuming a universal LED protocol.

## 6. Safety and ownership

- Read-only discovery precedes all writes.
- Require explicit user selection of a zone and confirmation before first write.
- Never infer header voltage from software device names; require physical/profile verification.
- Do not allow concurrent OpenRGB, vendor utility and Shookshmam writes to the same controller.
- Save previous state when readable; if state cannot be read or restored, disclose this before applying changes.
- On adapter failure, stop sending commands; do not loop rapid retries.

## 7. UI requirements

- Hardware/zone topology page.
- Electrical type and validation status visible per zone.
- Per-zone color/effect editor and preview.
- Global theme and per-zone override support.
- Profile save/apply and reset-to-known-state controls.
- Unsupported features clearly marked, not silently emulated.

## 8. Acceptance criteria

1. Physical 5V ARGB and 12V RGB paths are correctly distinguished.
2. Unknown or unverified zones cannot receive write commands.
3. Static color and Off work on each validated zone.
4. Each supported effect behaves within controller limits.
5. No competing process writes to an owned controller.
6. Disconnect/reconnect and app restart do not cause unintended lighting changes.
7. Windows and Bazzite implementations pass the same capability and safety contract tests.
8. Arctic AIO and Lian Li lighting are controlled independently only if wiring/controller topology genuinely permits it.

## 9. Work breakdown

- P4-01: physical wiring and electrical-type inventory
- P4-02: lighting zone/capability model
- P4-03: controller/library licence and protocol review
- P4-04: ASUS ARGB adapter discovery and read-only tests
- P4-05: Lian Li RGB adapter integration
- P4-06: effect schema and validation
- P4-07: zone editor and preview UI
- P4-08: profile persistence and ownership coordination
- P4-09: staged hardware validation and regression tests
