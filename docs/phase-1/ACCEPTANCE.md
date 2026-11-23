# Phase 1 — Interim Acceptance Report

Status as of the current implementation pass. This records progress against the
acceptance criteria in `PHASE-1-SPEC.md §11`. It is **interim**: hardware-in-the-
loop (HIL) items cannot be signed off until the target machine and the ordered
Thermalright display are available for validation.

## Work item status

| Item | Description | Status |
|---|---|---|
| P1-01 | Workspace, lint, CI, app shell | Done — toolchain, rustfmt, clippy `-Dwarnings`, GitHub Actions (Win+Linux Rust matrix, frontend build) |
| P1-02 | Sensor model and provider traits | Done — `SensorReading`/`Snapshot`/`DeviceDescriptor`, strict validation, `SensorProvider` |
| P1-03 | Discovery registry and diagnostics | Done — `Registry` with panic/error isolation, `Diagnostics` summary |
| P1-04 | Windows read-only provider | Stub — honest `NotImplemented` pending LibreHardwareMonitor licensing review |
| P1-05 | Linux read-only provider | Partial — hwmon temp/fan reader + tested parsers; needs HIL coverage verification |
| P1-06 | Scheduler and freshness | Done — `Scheduler`, `TelemetryTracker` (available→stale→unavailable), `RingBuffer` |
| P1-07 | SQLite history + 30-day retention | Done — `HistoryStore` trait, memory + SQLite impls, batched inserts, bounded-batch prune |
| P1-08 | React dashboard | Done — live snapshot subscription, status chips, simulated-data banner |
| P1-09 | Thermalright identify + static frame | Blocked (hardware) — transport trait + frame validation ready; USB identity/protocol unknown |
| P1-10 | Thermalright live renderer | Partial — display-independent layout/render + test pattern implemented; send blocked on HIL |
| P1-11 | Integration tests + acceptance report | In progress — cross-crate contract/pipeline tests + this report |

## Acceptance criteria

| # | Criterion | Status | Notes |
|---|---|---|---|
| 1 | Starts on Windows and Bazzite x86_64 | Pending HIL | Builds target both; needs run verification on real OSes |
| 2 | UI responsive during polling | Done | Collection runs on a background thread; UI consumes events |
| 3 | Detected sensors show value/unit/source/timestamp | Done | Via dashboard cards (currently simulated source) |
| 4 | Unsupported/unavailable/permission-denied clearly represented | Done | Status chips + no-data rendering; enforced by model validation |
| 5 | Default 1s sampling where provider permits | Done | `effective_interval_ms` honors provider minimums |
| 6 | History persists + 30-day pruning tested | Done (logic) | SQLite store + retention tests; on-disk restart test is HIL |
| 7 | LCD static pattern + telemetry frame on real device | Blocked (hardware) | Marked blocked with diagnostics per spec §11.7 |
| 8 | USB disconnect/reconnect does not crash | Pending HIL | Transport reports health; reconnect loop is future work with device |
| 9 | No hardware-control writes in Phase 1 | Done | No write trait exists; transport refuses writes (`NotValidated`) |
| 10 | Win/Linux tests cover contract and error cases | Done (CI) | Registry isolation, freshness, history, fixtures |

## Remaining before Phase 1 sign-off

1. Validate hwmon sensor coverage on the actual Bazzite machine (which board/VRM
   and pump tachometers are exposed).
2. Complete and license-review the Windows sensor backend; replace the stub.
3. Confirm the Thermalright USB identity/protocol and implement the real
   `DisplayTransport`, static pattern and reconnect handling on the device.
4. Run the HIL test suite in `TEST-PLAN.md` against the target hardware.
