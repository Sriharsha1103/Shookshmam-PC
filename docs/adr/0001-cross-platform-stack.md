# ADR-0001: Cross-platform application stack

- Status: Accepted as project baseline
- Decision: React + TypeScript UI, Tauri 2 shell, Rust hardware core.
- Context: One application must run on Windows and Bazzite while accessing low-level OS/device interfaces.
- Consequences: Hardware access stays in Rust adapters; UI communicates through typed IPC. Platform-specific dependencies remain isolated.
