# Developer Guide

## Workspace Layout

- `cardio_core`: engine, catalog, persistence (WAL/CSV/state), progression, strength signal loader, tracing setup.
- `cardio_cli`: CLI binary `krep`.
- `cardio_tray`: Ayatana tray/GTK4 popup binary `krep-tray`.

## Running Tests

```bash
cargo test --all            # requires GTK/libadwaita dev packages
cargo test -p cardio_core
cargo test -p cardio_cli
```

Integration tests live under `cardio_cli/tests` and `cardio_cli/tests/integration/`.

## Engine/Session Model

- `SessionKind::Real(MicrodoseSession)` is the only variant that reaches WAL/CSV.
- `SessionKind::ShownButSkipped` is injected in-memory to influence the round-robin cycle after a skip.
- Categories: `Vo2`, `Gtg`, `Mobility`.
- Strength override: recent lower-body strength (<24h) biases to GTG.

## Persistence Invariants

- WAL: JSONL at `$DATA_DIR/wal/microdose_sessions.wal`, append-only with fs2 locks. Corrupted lines are skipped with WARN.
- State: `$DATA_DIR/wal/state.json`, locked reads/writes, atomic saves. Corruption falls back to defaults with WARN.
- CSV rollup: `cargo_core::csv_rollup::wal_to_csv_and_archive` syncs CSV then renames WAL to `.processed`, deduplicated across WAL/CSV.

## Catalog/Progression

- Default catalog built in `cardio_core::catalog`.
- Progression rules in `progression.rs` (`increase_intensity`): burpees (reps → style), KB swings (linear reps), GTG pull-ups (rep ceiling).

## Tray App Notes

- Tray menu via `ksni` StatusNotifier (DBus); GTK4/adw window rebuilt per prescription.
- Uses the same core engine/state/WAL as the CLI.
- Warning banner shown if state/strength files are present but invalid.
- Logging: `~/.local/share/krep/krep_tray.log` (file appended if writable, else stdout).

## CI & Packaging

- GitHub Actions workflow installs GTK/adwaita dev libs, then runs fmt/clippy/tests.
- `cargo deb` produces a `.deb` containing both binaries and the icon (see `install.sh` for manual installs).

## Modifying the Catalog

Catalog definitions live in `cardio_core/src/catalog.rs`. Keep IDs stable; progression keys and round-robin history rely on `definition.id`.

## Adding Session Types

- Extend `MicrodoseCategory` and catalog entries.
- Implement selection rules in `engine.rs`.
- Update tests where category ordering/round-robin expectations are asserted.
