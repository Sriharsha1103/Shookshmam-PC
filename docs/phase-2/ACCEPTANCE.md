# Phase 2 — Interim Acceptance Report

Interim status. Phase 2 integrates the Lian Li TL controller. The device-layer
software (model, capability gating, port mapping, ownership policy, gated
transports) is implemented and unit-tested; everything that touches the real USB
protocol is **blocked on hardware-in-the-loop (HIL) validation and the P2-01
protocol/licence review**, and refuses writes until then.

## Work item status

| Item | Description | Status |
|---|---|---|
| P2-01 | Protocol/library and licence review | Open — no validated protocol; `VALIDATED_REVISIONS` is empty, keeping all writes gated |
| P2-02 | Controller identification + capability probing | Done (logic) — `LianLiController::probe`; unknown revisions stay read-only |
| P2-03 | Stable device/port mapping | Done — `logical_device_id(controller, port)`, order-independent |
| P2-04 | TL fan telemetry provider | Gated — `LianLiFanProvider` refuses (`NotImplemented`); no RPM is inferred from duty |
| P2-05 | LCD transport + static frame | Gated — `LianLiLcdTransport` via shared `DisplayTransport`; refuses until resolution/protocol confirmed |
| P2-06 | Multi-screen layout/rendering | Partial — reuses the Phase 1 scalable renderer; per-screen layouts stored separately (TODO wiring) |
| P2-07 | Optional validated RGB capability | Gated — `RgbEffectWrite` capability modeled but `Unvalidated`; deferred to Phase 4 |
| P2-08 | Windows/Bazzite HW validation + diagnostics | Blocked (hardware) |

## Acceptance criteria (spec §9)

| # | Criterion | Status |
|---|---|---|
| 1 | TL controller detected or unsupported state diagnosed | Logic ready; detection is HIL |
| 2 | Each TL LCD mapped to correct logical screen after reboot/reconnect | Stable-ID mapping done; HIL verification pending |
| 3 | Static test image on every validated screen | Blocked (hardware) |
| 4 | Live telemetry layout updates without blocking polling | Covered by Phase 1 thread isolation |
| 5 | Fan RPM reported only from a real tachometer | Enforced — provider refuses rather than estimate |
| 6 | Disconnect/reconnect does not crash or corrupt state | Transport reports health; registry isolation covers crashes |
| 7 | Competing-process ownership detected/handled | Done — `check_ownership` (L-Connect/OpenRGB) |
| 8 | No unsupported firmware / destructive commands | Enforced — no such command path exists |
| 9 | Validated capabilities have tests/procedures | Unit tests present; HIL procedures in TEST-PLAN scope |

## Remaining before sign-off
1. P2-01 protocol + licence review (evaluate the Lian Li Linux project / documented protocol).
2. Confirm TL LCD pixel format/resolution and implement the real transport.
3. Confirm the fan tachometer source and implement `LianLiFanProvider` reads.
4. HIL validation on Windows and Bazzite; populate `VALIDATED_REVISIONS`.
