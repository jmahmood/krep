# CLAUDE.md - AI Assistant Guide for Krep

This document provides comprehensive guidance for AI assistants (like Claude) working with the Krep codebase. It explains the architecture, conventions, workflows, and critical context needed to make effective contributions.

## Project Overview

**Krep** is a Rust-based cardio microdose prescription system that intelligently prescribes 30-second to 5-minute high-intensity cardio sessions throughout the day. It tracks progression, integrates with external strength training signals, and uses a robust WAL-based persistence system.

**Current Status**: v0.1 (stable core, tray app WIP)

## Repository Structure

```
krep/
├── cardio_core/          # Core business logic library
│   ├── src/
│   │   ├── lib.rs        # Public API exports
│   │   ├── types.rs      # Domain types (movements, sessions, metrics)
│   │   ├── engine.rs     # Prescription logic (v1.1 rules)
│   │   ├── progression.rs # Intensity upgrade algorithms
│   │   ├── catalog.rs    # Movement/workout definitions
│   │   ├── wal.rs        # Write-ahead logging
│   │   ├── csv_rollup.rs # CSV archival and deduplication
│   │   ├── state.rs      # User progression state management
│   │   ├── history.rs    # Session history loading
│   │   ├── strength.rs   # External strength signal integration
│   │   ├── config.rs     # Configuration management (TOML)
│   │   ├── error.rs      # Error types (thiserror)
│   │   └── logging.rs    # Logging setup (tracing)
│   └── Cargo.toml
├── cardio_cli/           # Command-line interface binary
│   ├── src/
│   │   └── main.rs       # CLI entry point (clap)
│   ├── tests/            # Integration tests
│   │   ├── integration_tests.rs
│   │   ├── concurrency_tests.rs
│   │   └── corruption_recovery_tests.rs
│   └── Cargo.toml
├── cardio_tray/          # GNOME tray app (WIP)
│   ├── src/
│   │   └── main.rs       # GTK4/libadwaita UI
│   └── Cargo.toml
├── Cargo.toml            # Workspace configuration
├── Cargo.lock            # Locked dependencies
└── README.md             # User-facing documentation
```

## Architecture Principles

### 1. Type Safety First

The codebase uses Rust's type system to enforce correctness at compile time:

- **Enum-based metrics**: `MetricSpec` uses tagged enums (`Reps`, `Band`) instead of stringly-typed maps
- **Session type safety**: `SessionKind` enum distinguishes between `Real` sessions (persisted) and `ShownButSkipped` (in-memory only) to prevent accidental persistence of skipped prescriptions
- **Movement styles**: `MovementStyle` enum wraps `BurpeeStyle` and `BandSpec` for type-safe progression
- **No unsafe code**: `#![forbid(unsafe_code)]` in `cardio_core`

### 2. WAL-First Persistence

Data flow: `WAL → CSV → Analysis`

- **Write-Ahead Log (WAL)**: Append-only JSONL file (`microdose_sessions.wal`) with fs2 file locking
- **Atomic operations**: All file writes use atomic operations (write-to-temp + rename)
- **CSV rollup**: Background archival process with deduplication via UUID tracking
- **Zero data loss**: WAL never deleted until successfully rolled up and archived to `.processed`

File locations (XDG compliant):
- WAL: `~/.local/share/krep/wal/microdose_sessions.wal`
- State: `~/.local/share/krep/wal/state.json`
- CSV: `~/.local/share/krep/sessions.csv`
- Strength signal: `~/.local/share/krep/strength/signal.json`

### 3. Prescription Engine (v1.1)

The engine implements intelligent workout selection (see `cardio_core/src/engine.rs`):

**Decision flow**:
1. **Strength-based override**: If lower-body strength session within 24h → prefer GTG or Mobility
2. **VO2 timing**: If last VO2 session > 4h ago → prioritize VO2
3. **Round-robin**: Cycle through categories and definitions to avoid repetition
4. **User-forced category**: `--category vo2|gtg|mobility` overrides all rules

**Key functions**:
- `prescribe_next()`: Main entry point
- `determine_category()`: Category selection logic
- `select_definition_from_category()`: Round-robin within category
- `compute_intensity()`: Apply progression state to definition

### 4. Progression System

Each microdose definition has independent progression state (see `cardio_core/src/progression.rs`):

**Burpees** (`upgrade_burpee`):
- Increment reps to ceiling (default: 10)
- Then upgrade style: 4-count → 6-count → 6-count-2-pump → seal
- Reset reps after style upgrade

**KB Swings** (`upgrade_kb_swing`):
- Linear: `base_reps + level`, capped at `max_reps` (default: 15)

**Pullups** (`upgrade_pullup`):
- Rep progression (band selection is manual via state file)

State persisted in `state.json` as:
```json
{
  "progressions": {
    "burpee_emom_5": {
      "reps": 5,
      "style": { "burpee": "four_count" },
      "level": 3,
      "last_upgraded": "2025-11-17T10:00:00Z"
    }
  },
  "last_mobility_def_id": "hip_car"
}
```

### 5. Catalog System

The catalog is the source of truth for workouts (see `cardio_core/src/catalog.rs`):

- `build_default_catalog()`: Returns hardcoded catalog (no database)
- `Catalog::validate()`: Runs integrity checks (movement references, metric specs)
- Extensible via config TOML for custom mobility drills

**Adding new movements**: Edit `catalog.rs` directly (no runtime registration yet)

## Development Conventions

### Code Style

- **Modules**: One concept per file (e.g., `engine.rs` only contains prescription logic)
- **Comments**: Doc comments (`///`) on all public items, module-level (`//!`) docs
- **Imports**: Grouped (stdlib → external crates → internal modules)
- **Naming**:
  - Functions: `verb_noun` (e.g., `load_recent_sessions`)
  - Types: `PascalCase` (e.g., `MicrodoseDefinition`)
  - Modules: `snake_case` (e.g., `csv_rollup`)

### Error Handling

All errors use `thiserror` (see `cardio_core/src/error.rs`):

```rust
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Prescription error: {0}")]
    Prescription(String),
    // ...
}
```

**Usage**:
- Propagate with `?` operator
- Library functions return `Result<T>`
- CLI `main()` returns `Result<()>` for clean error display

### Logging

Uses `tracing` crate (see `cardio_core/src/logging.rs`):

```rust
tracing::info!("Prescribing microdose from category: {:?}", category);
tracing::warn!("Category not found, using fallback");
tracing::debug!("Burpee progression: increased reps to {}", state.reps);
```

**Log levels**:
- `info`: User-relevant actions (prescription, session logging)
- `warn`: Non-fatal issues (fallback logic, missing files)
- `debug`: Internal state changes (progression upgrades)
- `error`: Failures (file I/O errors, validation failures)

**Environment variable**: `RUST_LOG=debug cargo run`

### Testing Strategy

**Unit tests** (in `cardio_core/src/*.rs`):
- Test pure functions (progression logic, catalog validation)
- Use `#[cfg(test)]` modules at file bottom
- Current coverage: 42 tests passing

**Integration tests** (in `cardio_cli/tests/*.rs`):
- Test CLI end-to-end (session logging, rollup, concurrency)
- Use `assert_cmd` and `tempfile` crates
- Key tests:
  - `test_session_logged_to_wal`: Verify WAL append
  - `test_dry_run_does_not_log`: Ensure dry-run flag works
  - `test_concurrent_writes`: fs2 locking correctness
  - `test_corruption_recovery`: WAL line-skipping resilience

**Running tests**:
```bash
# All tests
cargo test

# Core library only
cargo test -p cardio_core

# With output
cargo test -- --nocapture

# Specific test
cargo test test_burpee_progression
```

## Common Tasks for AI Assistants

### Adding a New Movement

1. **Edit `cardio_core/src/catalog.rs`**:
   ```rust
   movements.insert(
       "jump_rope".into(),
       Movement {
           id: "jump_rope".into(),
           name: "Jump Rope".into(),
           kind: MovementKind::Cardio,
           default_style: MovementStyle::None,
           tags: vec!["vo2".into()],
           reference_url: Some("https://...".into()),
       },
   );
   ```

2. **Create microdose definition** (in same file):
   ```rust
   microdoses.insert(
       "jump_rope_3min".into(),
       MicrodoseDefinition {
           id: "jump_rope_3min".into(),
           name: "3-Min Jump Rope".into(),
           category: MicrodoseCategory::Vo2,
           suggested_duration_seconds: 180,
           gtg_friendly: false,
           blocks: vec![MicrodoseBlock {
               movement_id: "jump_rope".into(),
               movement_style: MovementStyle::None,
               duration_hint_seconds: 180,
               metrics: vec![],
           }],
           reference_url: None,
       },
   );
   ```

3. **Add validation test**:
   ```rust
   #[test]
   fn test_jump_rope_in_catalog() {
       let catalog = build_default_catalog();
       assert!(catalog.movements.contains_key("jump_rope"));
       assert!(catalog.microdoses.contains_key("jump_rope_3min"));
   }
   ```

4. **Run tests**: `cargo test -p cardio_core`

### Modifying Prescription Logic

1. **Locate rule in `cardio_core/src/engine.rs`**:
   - Category selection: `determine_category()`
   - Definition selection: `select_definition_from_category()`
   - Intensity: `compute_intensity()`

2. **Update logic** (example: add time-of-day rule):
   ```rust
   fn determine_category(ctx: &UserContext) -> Result<MicrodoseCategory> {
       // New rule: No VO2 after 8pm
       if ctx.now.hour() >= 20 {
           return Ok(MicrodoseCategory::Mobility);
       }

       // Existing rules...
   }
   ```

3. **Add test**:
   ```rust
   #[test]
   fn test_late_evening_prescribes_mobility() {
       let ctx = UserContext {
           now: Utc.ymd(2025, 11, 17).and_hms(21, 0, 0),
           // ...
       };
       let category = determine_category(&ctx).unwrap();
       assert_eq!(category, MicrodoseCategory::Mobility);
   }
   ```

4. **Document in docstring** (v1.2 prescription logic)

### Adding Configuration Options

1. **Update `Config` struct in `cardio_core/src/config.rs`**:
   ```rust
   #[derive(Deserialize)]
   pub struct ProgressionConfig {
       pub burpee_rep_ceiling: i32,
       pub kb_swing_max_reps: i32,
       pub pullup_gtg_increment: i32, // NEW
   }
   ```

2. **Update default values** (in `impl Default`):
   ```rust
   impl Default for ProgressionConfig {
       fn default() -> Self {
           Self {
               // ...
               pullup_gtg_increment: 1,
           }
       }
   }
   ```

3. **Use in progression logic** (`cardio_core/src/progression.rs`):
   ```rust
   pub fn upgrade_pullup(state: &mut ProgressionState, increment: i32) {
       state.reps += increment;
       // ...
   }
   ```

4. **Document in README** (Configuration section)

### Fixing Persistence Bugs

**Critical invariants** (violations = data loss):

1. **WAL must be append-only**: Never truncate or overwrite
2. **Atomic writes**: Always write to `.tmp` + rename
3. **File locking**: Use `fs2::FileExt::try_lock_exclusive()` before WAL append
4. **UUID uniqueness**: Generate with `uuid::Uuid::new_v4()`
5. **CSV deduplication**: Track seen UUIDs in rollup to prevent duplicates

**Common bugs**:
- Missing `.flush()` after WAL write → partial records
- Forgot to unlock file → deadlock on next write
- Parsing error → entire WAL unreadable (use line-by-line parsing with `serde_json::from_str`)

**Debugging**: Check `~/.local/share/krep/wal/` for lock files, tmp files

### Adding CLI Commands

1. **Update `Commands` enum in `cardio_cli/src/main.rs`**:
   ```rust
   #[derive(Subcommand)]
   enum Commands {
       // ...
       /// Show workout history
       History {
           #[arg(long, default_value = "7")]
           days: u32,
       },
   }
   ```

2. **Implement handler**:
   ```rust
   fn cmd_history(data_dir: PathBuf, days: u32) -> Result<()> {
       let sessions = load_recent_sessions(&wal_path, &csv_path, days)?;
       for session in sessions {
           println!("{:?}", session);
       }
       Ok(())
   }
   ```

3. **Wire up in `main()`**:
   ```rust
   match cli.command {
       // ...
       Some(Commands::History { days }) => cmd_history(data_dir, days),
   }
   ```

4. **Add integration test**:
   ```rust
   #[test]
   fn test_history_command() {
       cli().arg("history").arg("--days").arg("3")
           .assert().success();
   }
   ```

## Critical Context for AI Assistants

### What NOT to Change

1. **WAL format**: Changing schema breaks existing user data (needs migration)
2. **State.json structure**: Same issue (add new fields as `Option<T>`)
3. **Catalog IDs**: Changing `definition_id` breaks progression history
4. **File locking strategy**: fs2 is required for correctness (don't replace)
5. **Type safety patterns**: `SessionKind` enum prevents bugs (don't flatten to single type)

### When to Ask User

1. **Breaking changes**: Schema migrations, catalog ID changes
2. **New dependencies**: Especially platform-specific (GTK, etc.)
3. **Prescription logic changes**: May affect user experience
4. **Data directory location**: XDG compliance is deliberate

### Common Pitfalls

1. **Forgot to run tests**: `cargo test` before committing
2. **Hardcoded paths**: Use `dirs::data_dir()` for XDG compliance
3. **Unwrapping Results**: Use `?` operator (see error.rs)
4. **Missing validation**: Run `catalog.validate()` after catalog changes
5. **Platform assumptions**: Code runs on Linux primarily (GTK4 tray app)

## Build and Release

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# CLI only (no GTK dependencies)
cargo build --release -p cardio_cli

# Check without building
cargo check
```

Binaries output to `target/release/`:
- `cardio_cli` (or `krep` symlink)
- `cardio_tray` (requires GTK4)

### Testing

```bash
# All tests
cargo test

# Core library only (unit tests)
cargo test -p cardio_core

# Integration tests only
cargo test -p cardio_cli --test integration_tests

# With logging
RUST_LOG=debug cargo test -- --nocapture
```

### Dependencies

**Core** (always required):
- `serde`, `serde_json`: Serialization
- `chrono`: Date/time handling
- `uuid`: Session IDs
- `thiserror`: Error types
- `toml`: Config parsing
- `tracing`: Logging
- `fs2`: File locking
- `csv`: CSV export
- `dirs`: XDG directories

**CLI** (for binary):
- `clap`: Argument parsing

**Tray app** (optional, Linux only):
- `gtk4`, `libadwaita`: UI
- `gio`, `glib`: GObject bindings
- `libayatana-appindicator`: System tray

**Testing**:
- `tempfile`: Temp directories
- `assert_cmd`: CLI testing
- `predicates`: Assertions

## Roadmap and TODOs

### v0.1 (Current - Stable)
- [x] Core prescription engine
- [x] WAL persistence
- [x] CSV rollup
- [x] Progression algorithms
- [x] CLI interface
- [x] Integration tests

### v0.2 (Next - In Progress)
- [ ] GNOME tray application (Milestone 5)
- [ ] GitHub Actions CI (Milestone 6)
- [ ] Installation script

### v0.3 (Future)
- [ ] Time-based prescriptions (cron integration)
- [ ] HR zone tracking
- [ ] Multi-user support
- [ ] Web dashboard for analytics

## Additional Resources

- **User documentation**: See `README.md`
- **Domain spec**: v1.1 prescription logic (documented in `engine.rs`)
- **API docs**: `cargo doc --open` (generates from `///` comments)
- **Tracing**: `RUST_LOG=debug` for verbose logs

## Questions?

If you're an AI assistant uncertain about:
- **Architecture decisions**: Check this doc's principles section
- **Code patterns**: Look at existing modules (e.g., `progression.rs` for upgrade logic)
- **Tests**: See `cardio_cli/tests/integration_tests.rs` for patterns
- **Breaking changes**: Ask the user before modifying WAL/state format

**Remember**: This codebase prioritizes type safety, data integrity, and zero data loss. When in doubt, prefer compile-time checks over runtime validation, and WAL-first persistence over in-memory state.
