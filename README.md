# Krep - Cardio Microdose Prescription System

A Rust-based system for prescribing and tracking short, high-intensity cardiovascular "microdose" workouts throughout the day.

## Overview

Krep intelligently prescribes 30-second to 5-minute cardio sessions based on:
- Your recent workout history
- External strength training signals
- Automatic progression tracking
- Time-of-day and recovery considerations

## Features

### ✅ Implemented (v0.1)

- **Smart Prescription Engine** (v1.1 spec)
  - VO2 EMOM workouts (burpees, KB swings)
  - GTG (Grease the Groove) pull-ups with band assistance
  - Mobility drills (hip CARs, shoulder CARs)
  - Strength-signal integration (24h lower-body override)
  - Time-based VO2 prioritization (>4h since last session)

- **Automatic Progression**
  - Burpees: Reps → Style transitions (4-count → 6-count → seal)
  - KB Swings: Linear rep progression
  - Pull-ups: Rep-based GTG progression

- **Robust Persistence**
  - Write-Ahead Log (WAL) with fs2 file locking
  - Atomic CSV rollup for analytics
  - Deduplication across WAL and CSV
  - 7-day session history window

- **Configuration**
  - TOML-based config (`~/.config/krep/config.toml`)
  - XDG Base Directory compliance
  - Configurable progression parameters
  - User-extendable mobility drills

- **Command-Line Interface**
  - `krep` - Prescribe and log sessions
  - `krep now --category vo2` - Force category
  - `krep now --dry-run` - Preview without logging
  - `krep rollup` - Archive WAL to CSV

### 🚧 In Progress

- GNOME system tray integration (Milestone 5)
- Integration tests (Milestone 4.5)
- CI/CD pipeline (Milestone 6)

## Installation

### Prerequisites

- Rust 1.70+ (`rustup install stable`)
- For tray app (optional): GTK4, libadwaita

### Build

```bash
# Clone repository
git clone <repo-url>
cd krep

# Build all binaries
cargo build --release

# Or build just the CLI (no GTK dependencies)
cargo build --release -p cardio_cli
```

Binaries will be in `target/release/`:
- `cardio_cli` - Command-line interface
- `cardio_tray` - GNOME tray app (requires GTK)

## Usage

### Basic Usage

```bash
# Get your next microdose prescription
krep

# Or explicitly
krep now
```

**Example output:**
```
╭─────────────────────────────────────────╮
│  Vo2 MICRODOSE
╰─────────────────────────────────────────╯

  5-Min EMOM: Burpees
  Duration: ~300 seconds (5 min)

  → 3 reps
  → Style: FourCount

─────────────────────────────────────────
Press Enter when done
  's' + Enter to skip
  'h' + Enter to mark 'harder next time'
>
```

### Force a Category

```bash
# Mobility work
krep now --category mobility

# VO2 session
krep now --category vo2

# GTG pull-ups
krep now --category gtg
```

### Preview Without Logging

```bash
krep now --dry-run
```

### Archive Sessions

```bash
# Roll up WAL to CSV
krep rollup

# Also clean up processed WAL files
krep rollup --cleanup
```

### External Strength Signal

Krep can read strength training data from `$DATA_DIR/strength/signal.json`:

```json
{
  "last_session_at": "2025-11-17T10:30:00Z",
  "session_type": "lower"
}
```

If a lower-body session was within 24h, Krep will prefer GTG or mobility over VO2.

## Configuration

Optional config file: `~/.config/krep/config.toml`

```toml
[data]
data_dir = "~/.local/share/krep"

[equipment]
available = ["kettlebell", "pullup_bar", "bands"]

[progression]
burpee_rep_ceiling = 10
kb_swing_max_reps = 15

[mobility]
custom = [
  { id = "90_90", name = "90/90 Hip Stretch", url = "https://..." },
]
```

## Data Storage

- **State**: `$DATA_DIR/wal/state.json` - Progression levels
- **WAL**: `$DATA_DIR/wal/microdose_sessions.wal` - Append-only session log
- **CSV**: `$DATA_DIR/sessions.csv` - Archived sessions for analysis
- **Strength**: `$DATA_DIR/strength/signal.json` - External strength training data

Default `DATA_DIR`: `~/.local/share/krep`

## Architecture

### Workspace Structure

- **cardio_core**: Core business logic (library)
  - Domain types and catalog
  - Prescription engine
  - Progression algorithms
  - Persistence layer (WAL, CSV, state)
  - Configuration management

- **cardio_cli**: Command-line interface (binary)
  - Argument parsing with clap
  - Session logging workflow
  - CSV rollup commands

- **cardio_tray**: GNOME tray app (binary, WIP)
  - GTK4/libadwaita UI
  - Ayatana AppIndicator integration
  - Popup prescription dialogs

### Design Principles

- **Type Safety**: Enum-based metrics, comprehensive validation
- **Atomic Operations**: fs2 locking, atomic file operations
- **Zero Data Loss**: WAL-first, staged rollup with .processed archiving
- **Testability**: 42 unit tests, deterministic prescription logic
- **No Unsafe Code**: `#![forbid(unsafe_code)]` in core library

## Apple Watch Implementation

A native watchOS version of Krep is available as a **pure Swift port** of the core business logic.

### Status: Phase 1 Complete ✅

- **Core Logic Ported** (~500 LOC)
  - Types, Catalog, Engine, Progression
  - 1:1 Swift equivalents of Rust types
  - Full v1.1 prescription algorithm

- **Unit Tests** (20+ tests)
  - Catalog validation
  - Prescription engine rules
  - Progression algorithms

### Location

```
cardio_watch/
├── Package.swift              # Swift Package Manager config
├── Sources/CardioCore/        # Business logic port
│   ├── Types.swift            # Domain types
│   ├── Catalog.swift          # Workout definitions
│   ├── Engine.swift           # Prescription algorithm
│   └── Progression.swift      # Intensity upgrades
└── Tests/CoreTests/           # XCTest unit tests
```

### Next Steps (Requires macOS + Xcode)

Phase 2-5 implementation requires Xcode on macOS:
- **watchOS UI** (SwiftUI views)
- **SwiftData storage** (replaces WAL)
- **HealthKit integration** (live HR monitoring)
- **iPhone companion app** (analytics, sync)

### Documentation

See **[docs/WATCH_IMPLEMENTATION.md](docs/WATCH_IMPLEMENTATION.md)** for:
- Architecture decisions (Why Swift over FFI?)
- Type mappings (Rust → Swift)
- Implementation guide (UI, storage, HealthKit)
- Build instructions (Xcode setup)
- Migration guide (Rust data → watchOS)

### Features (Planned)

- ⌚ **Native watchOS app** with live workouts
- ❤️ **HealthKit integration** for HR tracking
- 📊 **SwiftData persistence** with iCloud sync
- 🔔 **Complications** showing last workout
- 📱 **iPhone companion** for analytics

**Estimated completion**: 1-2 weeks for experienced iOS developer

## Testing

```bash
# Run all tests
cargo test -p cardio_core

# With output
cargo test -p cardio_core -- --nocapture
```

**Current test coverage:**
- ✅ 42 tests passing
- ✅ Catalog validation
- ✅ Progression algorithms (burpee, KB, pullup)
- ✅ Prescription engine rules
- ✅ WAL persistence & locking
- ✅ CSV rollup & deduplication
- ✅ State management
- ✅ History loading

## Development

### Adding Custom Movements

Krep supports adding new exercises, stretches, and mobility drills. For comprehensive instructions, see:

**📖 [Adding Exercises Guide](docs/ADDING_EXERCISES.md)**

This guide covers:
- Adding simple movements (mobility drills, stretches)
- Creating movements with automatic progression
- Working with metrics and movement styles
- Multi-block workouts (circuits, complexes)
- Custom progression algorithms
- Testing and validation

**Quick Example** - Add a mobility drill:

```rust
// In cardio_core/src/catalog.rs
movements.insert(
    "ankle_cars".into(),
    Movement {
        id: "ankle_cars".into(),
        name: "Ankle CARs".into(),
        kind: MovementKind::MobilityDrill,
        default_style: MovementStyle::None,
        tags: vec!["mobility".into(), "ankle".into()],
        reference_url: Some("https://...".into()),
    },
);
```

### Running with Logging

```bash
# Debug logs
RUST_LOG=debug krep now

# Info logs (default)
RUST_LOG=info krep now
```

## Roadmap

### v0.2 (Next)
- [ ] GNOME tray application
- [ ] Integration tests
- [ ] GitHub Actions CI
- [ ] Installation script

### v0.3 (Future)
- [ ] Time-based prescriptions (cron integration)
- [ ] HR zone tracking
- [ ] Multi-user support
- [ ] Web dashboard for analytics

## License

MIT OR Apache-2.0

## Credits

Built with:
- [Rust](https://www.rust-lang.org/)
- [clap](https://docs.rs/clap/) - CLI parsing
- [serde](https://serde.rs/) - Serialization
- [chrono](https://docs.rs/chrono/) - Date/time handling
- [tracing](https://docs.rs/tracing/) - Structured logging
- [GTK4](https://gtk.org/) / [libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/) - UI (tray app)
