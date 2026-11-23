# Phase 5 Technical Specification — Profiles, Automation and Release Hardening

**Status:** Planned  
**Depends on:** Phases 1–4 individually accepted

## 1. Objective

Unify monitoring, LCD layouts, fan curves and ARGB settings into reliable user profiles; provide startup behavior, cross-platform configuration portability, diagnostics and release-quality packaging for Windows and Bazzite.

## 2. Scope

### Included
- Named profiles: Quiet, Balanced, Performance and Custom (editable templates, not fixed hardware assumptions).
- Profile bundles for fan curves, LCD layout assignments, RGB zones/effects and dashboard preferences.
- Per-platform capability-aware profile application.
- Import/export of portable configuration bundles.
- Startup integration and service lifecycle management.
- Profile switching with preview, validation and rollback.
- Device reconnect automation and safe restoration.
- Health/diagnostic page and support bundle export.
- Release packaging, signing strategy, versioning, migration and update documentation.
- Performance, soak, regression and recovery testing.

### Excluded
- Cloud account, telemetry upload or remote device control.
- Automatic overclocking, undervolting or GPU power tuning.
- Applying a profile to hardware that has not passed capability validation.

## 3. Profile model

A profile includes:
- Schema version and profile ID.
- Human-readable name and description.
- Platform compatibility metadata.
- Hardware selectors/capability requirements.
- Dashboard and per-display layout assignments.
- Fan curve assignments and control mode preferences.
- Lighting zone/effect assignments.
- Startup behavior preferences.
- Creation/update timestamps.

Profiles must be declarative. They must not contain executable scripts or arbitrary shell commands.

## 4. Profile application transaction

1. Load and validate the profile schema.
2. Discover current hardware and capabilities.
3. Compare profile requirements with available devices.
4. Show a preview of changes and unsupported/missing components.
5. Require confirmation for hardware writes.
6. Apply changes in a deterministic order with per-device acknowledgements.
7. If a step fails, stop and execute supported rollback/release actions.
8. Record outcome and diagnostics.

A partial application must never be presented as fully successful.

## 5. Cross-platform portability

- Store portable settings separately from OS-specific device identifiers and paths.
- Match devices using stable identifiers and capability selectors; prompt when matching is ambiguous.
- Never assume Windows and Linux expose identical sensors or control features.
- Unsupported profile sections are skipped only with explicit user-visible explanation.
- Import/export must include schema version and migration support.

## 6. Startup and service behavior

- Optional launch at user login.
- Hardware service starts only with required permissions and device access.
- UI may close while the service remains active only when the user explicitly enables background operation.
- Provide a visible service status and stop/release control.
- On shutdown/restart, execute tested controller release behavior; BIOS/firmware remains the cooling fallback.
- Bazzite packaging must respect its immutable OS model and avoid modifying the host in unsupported ways.

## 7. Reliability and diagnostics

- Structured logs with rotation and bounded storage.
- Support bundle includes app version, OS version, adapter versions, device inventory, capability states and sanitized logs.
- No telemetry or diagnostic upload by default.
- SQLite migrations are versioned and tested.
- Database corruption recovery must preserve the ability to start monitoring without history.
- Update rollback and configuration backup/restore procedures documented.

## 8. Release requirements

- Windows installer/package and Bazzite-compatible distribution.
- Reproducible build instructions and dependency/licence notices.
- Versioned release notes and known-issues list.
- Code signing strategy for Windows release.
- Checksums for release artifacts.
- Clean install, upgrade, downgrade/rollback and uninstall tests.

## 9. Acceptance criteria

1. A profile can be created, edited, exported, imported and applied.
2. Profile application previews all changes and capability mismatches.
3. Partial failures are reported accurately and safe rollback/release is attempted.
4. Windows and Bazzite use the same portable profile format.
5. OS-specific settings are isolated and migrated safely.
6. Background service lifecycle and user permissions are documented and tested.
7. Monitoring remains stable during a 24-hour soak test.
8. LCD reconnect, controller disconnect, sensor provider failure and service restart tests pass.
9. No control operation bypasses the Phase 3/4 safety policies.
10. Release packages install, upgrade and uninstall cleanly on both target platforms.

## 10. Work breakdown

- P5-01: profile schema and versioning
- P5-02: profile editor and preview
- P5-03: transactional apply/rollback orchestration
- P5-04: cross-platform import/export and migration
- P5-05: startup/service lifecycle
- P5-06: diagnostics and support bundle
- P5-07: SQLite backup/migration/recovery tests
- P5-08: packaging, signing and release automation
- P5-09: performance, soak and fault-injection tests
- P5-10: release candidate and acceptance report
