# Test fixtures

Deterministic sample telemetry used by the integration tests in
`tests/integration`. These are **synthetic** records, not captured from any real
machine, and deliberately include every sensor status so the normalized contract
and the UI's non-available rendering paths are exercised:

- `sample-snapshot.json` — one `Snapshot` covering `available`, `stale`,
  `unsupported`, and `permission_denied` readings across CPU/GPU/board/SSD.
