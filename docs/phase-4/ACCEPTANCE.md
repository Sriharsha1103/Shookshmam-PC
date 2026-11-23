# Phase 4 — Interim Acceptance Report

Interim status. The lighting **software core is implemented and tested**; concrete
ARGB/RGB write backends and the zone-editor UI remain, and **no zone is
validated**, so every lighting write is refused. The most safety-relevant rule —
an unknown electrical type or topology disables writes — is enforced and tested.

## Work item status

| Item | Description | Status |
|---|---|---|
| P4-01 | Physical wiring + electrical-type inventory | Blocked (hardware) — model ready (`ElectricalType`, per-zone) |
| P4-02 | Lighting zone/capability model | Done — `LightingZone`, `writable()` safety gate |
| P4-03 | Controller/library licence + protocol review | Open (OpenRGB reuse) — no backend linked |
| P4-04 | ASUS ARGB discovery + read-only tests | Gated — modeled; discovery is HIL |
| P4-05 | Lian Li RGB adapter integration | Gated — depends on Phase 2 controller validation |
| P4-06 | Effect schema + validation | Done — `Effect` + `validate`, deterministic `render` engine |
| P4-07 | Zone editor + preview UI | Not started |
| P4-08 | Profile persistence + ownership coordination | Partial — ownership detection done; persistence is Phase 5 |
| P4-09 | Staged HW validation + regression tests | Blocked (hardware) |

## Acceptance criteria (spec §8)

| # | Criterion | Status |
|---|---|---|
| 1 | 5V ARGB vs 12V RGB paths correctly distinguished | Modeled — requires physical verification to set |
| 2 | Unknown/unverified zones cannot receive writes | Done — enforced + integration-tested (`lighting.rs`) |
| 3 | Static color + Off on each validated zone | Blocked (hardware) — engine ready |
| 4 | Each supported effect behaves within controller limits | Effect engine unit-tested; HIL behavior pending |
| 5 | No competing process writes to an owned controller | Done — `check_lighting_ownership` |
| 6 | Disconnect/reconnect/restart cause no unintended changes | By design — no startup default effect; writes gated |
| 7 | Win/Bazzite pass the same capability/safety contract | Blocked (hardware) |
| 8 | Arctic AIO + Lian Li lighting independent only if wiring permits | Enforced — per-zone connector identity; no assumption |

## Remaining before sign-off
1. Physically inventory headers (5V 3-pin vs 12V 4-pin), LED counts and daisy-chains; set `electrical_type`/`led_count` only from verified wiring.
2. P4-03 licence/protocol review and a real backend (e.g. OpenRGB) behind the `LightingController` trait; mark zones `Validated` only after HIL.
3. Zone editor + preview UI; profile save/restore (Phase 5).
4. Staged HW validation + regression on Windows and Bazzite.
