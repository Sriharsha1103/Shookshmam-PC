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

The software foundations for all five phases are implemented and unit/integration tested on Windows + Linux in CI: the monitoring stack (Phase 1), the Lian Li device/capability layer (Phase 2), the fan-cooling safety policy and curves (Phase 3), the lighting zone/effect engine (Phase 4), and profiles with transactional apply and diagnostics (Phase 5).

It is **not yet validated against real hardware, and performs no hardware writes.** Every write path — fan duty, LCD frames, RGB effects — is gated behind per-capability validation and refuses (`NotValidated`/`NotImplemented`) until hardware-in-the-loop validation. Live monitoring values currently come from a clearly-labeled simulation provider. See each phase's `docs/phase-*/ACCEPTANCE.md` for per-item status and what remains hardware-gated.

## Building and running

Prerequisites: a Rust toolchain (pinned in `rust-toolchain.toml`), Node.js 20+, and the [Tauri 2 system dependencies](https://tauri.app/start/prerequisites/) for your OS.

```sh
# Rust core: lint and test everything except the GUI shell
cargo test --workspace --exclude shookshmam-pc

# Desktop app (installs frontend deps on first run)
cd apps/desktop
npm install
npm run tauri dev      # or: npm run build  to type-check + bundle the UI
```

The CI workflow in `.github/workflows/ci.yml` runs fmt, clippy, tests (Windows + Linux) and the frontend build on every push.

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
