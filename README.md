# Shookshmam PC

Cross-platform PC monitoring and control platform for Windows and Bazzite Linux.

## Frozen product baseline

- Platforms: Windows 10/11 and Bazzite Linux (x86_64)
- UI: React + TypeScript
- Desktop shell: Tauri 2
- Hardware core: Rust
- Monitoring: live telemetry + 30-day history
- LCDs: independently configurable layouts for Thermalright Trofeo Vision 6.86-inch and Lian Li TL LCD devices
- Cooling: automatic curves + manual override
- Lighting: motherboard ARGB and supported Lian Li RGB
- Phase 1: read-only monitoring foundation and Thermalright LCD output; no fan/RGB writes

## Repository status

This is an architecture and implementation scaffold, not a finished or validated application. Hardware adapters are interfaces/placeholders until tested against the actual devices. Phase 1 contains no hardware-control command.

## Repository layout

- `apps/desktop`: React/Tauri desktop application
- `crates/`: platform-neutral Rust domain and service crates
- `adapters/`: OS and device-specific integrations
- `config/`: example hardware profile and application defaults
- `docs/`: architecture, ADRs, roadmap and phase specifications
- `tests/fixtures`: sample telemetry for deterministic tests

## Roadmap

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the complete phase map and links to all specifications.

1. Phase 1 — Monitoring Foundation
2. Phase 2 — Lian Li Device Integration
3. Phase 3 — Fan and Cooling Control
4. Phase 4 — ARGB and Lighting Control
5. Phase 5 — Profiles, Automation and Release Hardening

## Safety boundary

Phase 1 is read-only. Do not add PWM, pump, GPU fan, USB controller, or ARGB writes in Phase 1. Motherboard BIOS fan curves remain the fallback for all future control phases. Later phases must pass their safety gates before enabling writes.

Start with `docs/phase-1/PHASE-1-SPEC.md`. The remaining phase specifications define intended future scope and do not imply those features are already implemented.
