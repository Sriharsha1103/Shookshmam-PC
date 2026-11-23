# ADR-0002: Phase 1 is read-only

- Status: Accepted
- Decision: Phase 1 may read sensors and write LCD image frames only. It must not alter fan speed, pump speed, GPU fan behavior, RGB or ARGB settings.
- Context: Incorrect control behavior can cause overheating or device/controller faults. Hardware identity and capabilities are not yet validated.
- Consequences: Fan curves, manual overrides and lighting control are deferred until separate safety and hardware-validation milestones.
