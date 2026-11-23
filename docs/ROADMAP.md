# Shookshmam PC — Product Roadmap and Specification Index

## Frozen product decisions

- Windows + Bazzite Linux, x86_64.
- React + TypeScript UI, Tauri 2 shell, Rust hardware core.
- Live monitoring and historical charts.
- 30-day default telemetry retention.
- Independent layouts for Thermalright Trofeo Vision and Lian Li TL LCD screens.
- Automatic fan curves plus manual override.
- Monitoring, LCDs, fans and ARGB are in product scope.
- Hardware writes are capability-gated and safety-checked.

## Phase map

| Phase | Name | Main deliverable | Control writes |
|---|---|---|---|
| 1 | Monitoring Foundation | Cross-platform telemetry, history, dashboard, Thermalright LCD | None; read-only |
| 2 | Lian Li Integration | TL controller/fan discovery, TL LCD streaming and validated capabilities | Only explicitly validated display/RGB operations; fan control deferred |
| 3 | Fan and Cooling Control | Automatic curves, manual override, watchdog and safe release | Validated fan channels only; pump/GPU fans deferred |
| 4 | ARGB and Lighting | Motherboard and Lian Li lighting zones, effects and profiles | Validated lighting zones only |
| 5 | Profiles and Release Hardening | Unified profiles, startup, portability, packaging and reliability | Through Phase 3/4 safety policies only |

## Specifications

- [System architecture](architecture/ARCHITECTURE.md)
- [Phase 1 — Monitoring Foundation](phase-1/PHASE-1-SPEC.md)
- [Phase 1 — Test Plan](phase-1/TEST-PLAN.md)
- [Phase 2 — Lian Li Integration](phase-2/PHASE-2-SPEC.md)
- [Phase 3 — Fan and Cooling Control](phase-3/PHASE-3-SPEC.md)
- [Phase 4 — ARGB and Lighting](phase-4/PHASE-4-SPEC.md)
- [Phase 5 — Profiles and Release Hardening](phase-5/PHASE-5-SPEC.md)
- [Architecture Decision Records](adr/)

## Cross-phase release gates

1. A phase is not complete until its acceptance criteria and tests are recorded.
2. Hardware support is per device revision and capability, not assumed from product family name.
3. Unsupported sensors/devices remain explicitly unsupported; no fabricated readings.
4. No hardware write may bypass the safety policy and adapter validation.
5. Third-party code and protocol reuse require licence review and attribution.
6. Windows and Bazzite are tested independently; parity is claimed only for validated features.
7. BIOS/firmware remains the fallback for cooling safety.
