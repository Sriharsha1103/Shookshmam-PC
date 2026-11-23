# Phase 5 — Interim Acceptance Report

Interim status. The profile/automation **software core is implemented and
tested**: declarative schema with migration, transactional apply/rollback,
presets and a sanitized support bundle. Startup/service lifecycle, packaging and
the long-running soak/fault tests depend on the earlier phases being validated on
hardware and on the desktop shell wiring.

## Work item status

| Item | Description | Status |
|---|---|---|
| P5-01 | Profile schema + versioning | Done — `Profile` (schema_version), `migrate` |
| P5-02 | Profile editor + preview | Partial — Profiles tab previews each profile's plan + generates a support bundle; editor/apply UI pending |
| P5-03 | Transactional apply/rollback | Done — `apply()` with per-change ack + rollback; partial ≠ success |
| P5-04 | Cross-platform import/export + migration | Done — JSON `to_json`/`from_json`, platform compatibility, migration hook |
| P5-05 | Startup/service lifecycle | Documented (schema flags) — OS integration is app/packaging work |
| P5-06 | Diagnostics + support bundle | Done — `SupportBundle` with home-path/email sanitization |
| P5-07 | SQLite backup/migration/recovery | Partial — Phase 1 store + in-memory fallback on open failure; migration tests pending |
| P5-08 | Packaging, signing, release automation | Documented — see [PACKAGING.md](PACKAGING.md); not yet built |
| P5-09 | Performance, soak, fault-injection | Blocked — needs validated hardware + running shell |
| P5-10 | Release candidate + acceptance report | Pending Phases 1–4 HIL sign-off |

## Acceptance criteria (spec §9)

| # | Criterion | Status |
|---|---|---|
| 1 | Create/edit/export/import/apply a profile | Done (core) — editor UI pending |
| 2 | Apply previews changes + capability mismatches | Done — `ApplyPlan` lists unsupported + missing caps |
| 3 | Partial failures reported accurately; safe rollback | Done — integration-tested (`profiles.rs`) |
| 4 | Windows + Bazzite share one portable format | Done — JSON + platform compatibility |
| 5 | OS-specific settings isolated + migrated | Done — capability/stable-ID matching + migration hook |
| 6 | Background service lifecycle documented + tested | Documented; test is HIL/app work |
| 7 | Stable during a 24h soak | Blocked (hardware/app) |
| 8 | Reconnect/disconnect/provider-failure/restart tests pass | Partial — logic covered; full test is HIL |
| 9 | No control bypasses Phase 3/4 safety policies | By design — apply routes through the same gates |
| 10 | Packages install/upgrade/uninstall cleanly | Blocked — packaging not yet built |

## Remaining before sign-off
1. Profile editor + preview UI and desktop wiring of `plan`/`apply`.
2. Startup/service lifecycle + Bazzite-safe packaging (see PACKAGING.md).
3. SQLite migration + corruption-recovery tests.
4. Performance/soak/fault-injection once Phases 1–4 are hardware-validated.
