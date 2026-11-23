# Packaging and Release (Phase 5, P5-08)

Planning document for release-quality distribution on Windows and Bazzite. Not
yet implemented; captured here so the build is reproducible and the signing/
checksum strategy is decided before the first release.

## Targets

- **Windows 10/11 x86_64** — Tauri bundle (MSI/NSIS) via `cargo tauri build`.
- **Bazzite Linux x86_64** — AppImage or Flatpak. Bazzite is an immutable
  (rpm-ostree) host, so the app and any helper must install into the user space
  (`~/.local`, Flatpak) and must not modify the host image. USB device access is
  granted through a narrowly scoped udev rule shipped as documentation, not by
  running the app as root.

## Reproducible build

- Rust toolchain pinned in `rust-toolchain.toml` (1.82.0).
- Node pinned in CI (20). `npm ci` from the committed `package-lock.json`.
- `cargo tauri build` after `npm run build`.
- Dependency + licence notices generated with `cargo about` (to be added) and
  bundled with the release.

## Signing and integrity

- **Windows:** Authenticode signing of the installer and the main binary.
  Strategy and certificate custody to be decided; CI signs only on tagged
  releases, never on PR builds.
- **Linux:** detached signature for the AppImage / Flatpak repo signing.
- **Checksums:** publish SHA-256 for every artifact alongside the release.

## Versioning and migration

- Semantic version in `Cargo.toml` (`[workspace.package] version`) and
  `tauri.conf.json`, kept in sync at release time.
- Profile bundles carry `schema_version`; `profiles::migrate` upgrades older
  bundles on import.
- SQLite history schema changes are versioned migrations (P5-07); corruption
  recovery preserves the ability to start monitoring without history.

## Release tests (pre-tag checklist)

1. Clean install on a fresh Windows and Bazzite system.
2. Upgrade from the previous version (profile + history migration).
3. Downgrade / rollback path documented and exercised.
4. Uninstall leaves no controlling state; BIOS/firmware remains the cooling
   fallback.
5. 24-hour soak with monitoring active (P5-09).

## Release notes

Each release ships versioned notes and a known-issues list, with an explicit
statement of which hardware capabilities are validated on which platform
(parity is claimed only for validated features — ROADMAP release gate 6).
