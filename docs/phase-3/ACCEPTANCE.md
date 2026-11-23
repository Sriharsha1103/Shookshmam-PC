# Phase 3 — Interim Acceptance Report

Interim status. Phase 3 is a hardware-control phase with a gated rollout. The
**safety-critical control logic is implemented and unit/integration-tested**; the
concrete motherboard/Lian Li fan write backends and the curve-editor UI remain,
and **no channel is validated**, so every duty write is refused (`NotValidated`).

## Work item status

| Item | Description | Status |
|---|---|---|
| P3-01 | Header/channel inventory + capability schema | Done (model) — `ChannelInfo`, `ChannelBounds` (validated flag, allow_stop) |
| P3-02 | Control policy engine + typed command model | Done — `SafetyPolicy`, `ControlCommand`, `Decision` |
| P3-03 | Curve model, validation, persistence | Done — `FanCurve` (+serde), strict validation, presets |
| P3-04 | ASUS motherboard fan adapter (read-only first) | Gated — modeled via `GatedFanController`; real header mapping is HIL |
| P3-05 | Lian Li fan control adapter (if validated) | Gated — refused until Phase 2 controller protocol validated |
| P3-06 | Auto curve scheduler + smoothing | Done — `AutoController` (hysteresis + ramp-up/down limiting) |
| P3-07 | Manual override + expiry | Done — `ManualOverride` (30-min default, persistent requires confirm) |
| P3-08 | Watchdog, release, fault-injection | Done (logic) — `Watchdog`, safe-release paths, audit log; HIL fault tests pending |
| P3-09 | UI curve editor + channel view | Partial — read-only Cooling tab (channels + curve previews) in the dashboard; draggable editor pending |
| P3-10 | Staged HW validation (Win/Bazzite) | Blocked (hardware) |

## Acceptance criteria (spec §9)

| # | Criterion | Status |
|---|---|---|
| 1 | Every control channel physically identified + documented | HIL — model ready |
| 2 | Control disabled by default + after unclean shutdown | Done — channels start disabled; must be explicitly enabled |
| 3 | Valid curves apply without oscillation | Done — hysteresis + ramp limiting (unit-tested) |
| 4 | Invalid/stale/missing sensor never yields a low-duty command | Done — enforced + integration-tested (`cooling.rs`) |
| 5 | Manual override respects hardware bounds + expiry | Done |
| 6 | Disconnect/crash demonstrates safe fallback | Logic done (fallback/release/watchdog); HIL demo pending |
| 7 | BIOS fan control remains enabled | By design — we never disable it; release returns ownership |
| 8 | No pump/GPU fan control without separate approval | Enforced — no such channel/command path exists |
| 9 | Win/Bazzite validated independently | Blocked (hardware) |

## Remaining before sign-off
1. Physically map ASUS B550M-A headers (PWM vs DC) and the Arctic AIO pump/fan wiring before exposing any channel.
2. Implement the real fan write backend(s) and mark channels `validated` only after per-channel first-use calibration.
3. Curve-editor UI with live sensor overlay; one-click release-to-firmware.
4. HIL fault-injection (disconnect/crash/sensor-loss) on Windows and Bazzite.
