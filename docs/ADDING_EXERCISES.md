# Adding Exercises, Stretches, and Movements to Krep

This guide provides comprehensive instructions for adding new exercises, stretches, mobility drills, and workouts to the Krep cardio microdose system.

## Table of Contents

1. [Understanding the System Architecture](#understanding-the-system-architecture)
2. [Quick Start: Adding a Simple Movement](#quick-start-adding-a-simple-movement)
3. [Movement Types and Categories](#movement-types-and-categories)
4. [Adding Movements with Progression](#adding-movements-with-progression)
5. [Creating Microdose Definitions](#creating-microdose-definitions)
6. [Working with Metrics](#working-with-metrics)
7. [Movement Styles and Variations](#movement-styles-and-variations)
8. [Advanced: Custom Progression Logic](#advanced-custom-progression-logic)
9. [Testing Your Additions](#testing-your-additions)
10. [Complete Examples](#complete-examples)

---

## Understanding the System Architecture

The Krep system has three main components for exercises:

1. **Movements**: Individual exercises (e.g., "Burpee", "Hip CARs")
2. **Microdose Definitions**: Complete workout prescriptions that use movements (e.g., "5-Min EMOM: Burpees")
3. **Progression State**: Per-definition tracking of intensity increases over time

```
Movement (catalog.rs)
    ↓ referenced by
MicrodoseDefinition (catalog.rs)
    ↓ tracked in
ProgressionState (state.json)
    ↓ upgraded by
Progression Logic (progression.rs)
```

**Key Files:**
- `cardio_core/src/catalog.rs` - Movement and workout definitions
- `cardio_core/src/types.rs` - Type definitions (enums, structs)
- `cardio_core/src/progression.rs` - Intensity upgrade algorithms
- `cardio_core/src/engine.rs` - Prescription selection logic

---

## Quick Start: Adding a Simple Movement

The simplest addition is a **mobility drill** or **bodyweight exercise** without complex progression.

### Step 1: Add the Movement

Edit `cardio_core/src/catalog.rs` in the `build_default_catalog()` function:

```rust
movements.insert(
    "ankle_cars".into(),
    Movement {
        id: "ankle_cars".into(),
        name: "Ankle Controlled Articular Rotations (CARs)".into(),
        kind: MovementKind::MobilityDrill,
        default_style: MovementStyle::None,
        tags: vec!["mobility".into(), "ankle".into(), "gtg_ok".into()],
        reference_url: Some("https://www.youtube.com/watch?v=example".into()),
    },
);
```

**Field Explanations:**

- **id**: Unique identifier (snake_case, no spaces). Used throughout the system.
- **name**: Human-readable display name.
- **kind**: Movement type enum (see [Movement Types](#movement-types-and-categories)).
- **default_style**: Starting style variation (most movements use `MovementStyle::None`).
- **tags**: Searchable labels (convention: lowercase, underscore-separated).
- **reference_url**: Optional YouTube link or form guide.

### Step 2: Create a Microdose Definition

In the same file, add a workout that uses your movement:

```rust
microdoses.insert(
    "mobility_ankle_cars".into(),
    MicrodoseDefinition {
        id: "mobility_ankle_cars".into(),
        name: "Ankle CARs (5 reps each side)".into(),
        category: MicrodoseCategory::Mobility,
        suggested_duration_seconds: 180,
        gtg_friendly: true,
        reference_url: None,
        blocks: vec![MicrodoseBlock {
            movement_id: "ankle_cars".into(),
            movement_style: MovementStyle::None,
            duration_hint_seconds: 180,
            metrics: vec![MetricSpec::Reps {
                key: "reps_per_side".into(),
                default: 5,
                min: 3,
                max: 8,
                step: 1,
                progressable: false,
            }],
        }],
    },
);
```

**Field Explanations:**

- **id**: Unique identifier for this workout (convention: `category_movement_duration`).
- **category**: `Vo2`, `Gtg`, or `Mobility` (affects prescription logic).
- **suggested_duration_seconds**: Expected time to complete.
- **gtg_friendly**: Can this be done frequently without fatigue? (true for GTG/Mobility).
- **blocks**: Array of movement blocks (most microdoses have one block).

### Step 3: Update Test Expectations

The catalog has tests that count movements and definitions. Update `cardio_core/src/catalog.rs`:

```rust
#[test]
fn test_catalog_loads() {
    let catalog = build_default_catalog();
    assert_eq!(catalog.movements.len(), 6);  // Was 5, now 6
    assert_eq!(catalog.microdoses.len(), 6); // Was 5, now 6
}
```

### Step 4: Run Tests

```bash
cargo test -p cardio_core
```

If validation passes, your movement is ready!

---

## Movement Types and Categories

### MovementKind Enum

Defined in `cardio_core/src/types.rs`:

```rust
pub enum MovementKind {
    KettlebellSwing,
    Burpee,
    Pullup,
    MobilityDrill,
}
```

**Adding a New Kind:**

If your movement doesn't fit existing kinds (e.g., you want to add rowing):

1. Edit `cardio_core/src/types.rs`:
   ```rust
   pub enum MovementKind {
       KettlebellSwing,
       Burpee,
       Pullup,
       MobilityDrill,
       Rowing,  // NEW
   }
   ```

2. Use it in `catalog.rs`:
   ```rust
   kind: MovementKind::Rowing,
   ```

3. Add progression logic in `progression.rs` if needed (see [Custom Progression](#advanced-custom-progression-logic)).

### MicrodoseCategory Enum

Workouts are categorized for prescription logic:

- **Vo2**: High-intensity cardio (5-min EMOM burpees, KB swings)
- **Gtg**: Grease-the-Groove strength (short, submaximal, frequent)
- **Mobility**: Stretching and joint health (CARs, yoga flows)

**Prescription Behavior:**
- Vo2 workouts are prioritized if last VO2 session > 4 hours ago
- GTG/Mobility are preferred after recent lower-body strength training
- Round-robin selection within categories prevents repetition

---

## Adding Movements with Progression

Some movements benefit from automatic intensity increases (e.g., burpees, KB swings). This requires progression logic.

### Example: Jump Squats

Let's add jump squats with rep progression similar to KB swings.

#### Step 1: Add Movement Kind (if needed)

```rust
// cardio_core/src/types.rs
pub enum MovementKind {
    // ... existing kinds
    JumpSquat,  // NEW
}
```

#### Step 2: Add Movement to Catalog

```rust
// cardio_core/src/catalog.rs
movements.insert(
    "jump_squat".into(),
    Movement {
        id: "jump_squat".into(),
        name: "Jump Squat".into(),
        kind: MovementKind::JumpSquat,
        default_style: MovementStyle::None,
        tags: vec!["vo2".into(), "plyometric".into(), "legs".into()],
        reference_url: Some("https://www.youtube.com/watch?v=example".into()),
    },
);
```

#### Step 3: Create Microdose Definition

```rust
microdoses.insert(
    "emom_jump_squat_5m".into(),
    MicrodoseDefinition {
        id: "emom_jump_squat_5m".into(),
        name: "5-Min EMOM: Jump Squats".into(),
        category: MicrodoseCategory::Vo2,
        suggested_duration_seconds: 300,
        gtg_friendly: false,
        reference_url: None,
        blocks: vec![MicrodoseBlock {
            movement_id: "jump_squat".into(),
            movement_style: MovementStyle::None,
            duration_hint_seconds: 60,
            metrics: vec![MetricSpec::Reps {
                key: "reps".into(),
                default: 5,
                min: 3,
                max: 20,  // Higher ceiling for jump squats
                step: 1,
                progressable: true,  // IMPORTANT: Enable progression
            }],
        }],
    },
);
```

#### Step 4: Add Progression Logic

Edit `cardio_core/src/progression.rs`:

```rust
/// Upgrade jump squat intensity (linear progression)
pub fn upgrade_jump_squat(state: &mut ProgressionState, base_reps: i32, max_reps: i32) {
    if state.reps < max_reps {
        state.reps = (base_reps + state.level as i32 + 1).min(max_reps);
        state.level += 1;
        state.last_upgraded = Some(Utc::now());
        tracing::debug!("Jump squat progression: increased to {} reps", state.reps);
    } else {
        tracing::debug!("Jump squat progression: already at max ({} reps)", max_reps);
    }
}
```

#### Step 5: Wire Up Progression in `increase_intensity()`

In the same file, update the match statement:

```rust
pub fn increase_intensity(def_id: &str, user_state: &mut UserMicrodoseState, config: &Config) {
    let state = user_state
        .progressions
        .entry(def_id.to_string())
        .or_insert_with(|| {
            let (reps, style) = match def_id {
                "emom_burpee_5m" => (3, MovementStyle::Burpee(BurpeeStyle::FourCount)),
                "emom_kb_swing_5m" => (5, MovementStyle::None),
                "gtg_pullup_band" => (3, MovementStyle::None),
                "emom_jump_squat_5m" => (5, MovementStyle::None),  // NEW
                _ => (3, MovementStyle::None),
            };

            ProgressionState {
                reps,
                style,
                level: 0,
                last_upgraded: None,
            }
        });

    match def_id {
        "emom_burpee_5m" => {
            upgrade_burpee(state, config.progression.burpee_rep_ceiling);
        }
        "emom_kb_swing_5m" => {
            upgrade_kb_swing(state, 5, config.progression.kb_swing_max_reps);
        }
        "gtg_pullup_band" => {
            upgrade_pullup(state, 8);
        }
        "emom_jump_squat_5m" => {  // NEW
            upgrade_jump_squat(state, 5, 20);
        }
        _ => {
            tracing::warn!("Unknown definition ID for progression: {}", def_id);
        }
    }

    // ... rest of function
}
```

#### Step 6: Add Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_squat_progression() {
        let mut state = ProgressionState {
            reps: 5,
            style: MovementStyle::None,
            level: 0,
            last_upgraded: None,
        };

        upgrade_jump_squat(&mut state, 5, 20);
        assert_eq!(state.reps, 6);
        assert_eq!(state.level, 1);

        upgrade_jump_squat(&mut state, 5, 20);
        assert_eq!(state.reps, 7);
    }

    #[test]
    fn test_jump_squat_respects_max() {
        let mut state = ProgressionState {
            reps: 19,
            style: MovementStyle::None,
            level: 14,
            last_upgraded: None,
        };

        upgrade_jump_squat(&mut state, 5, 20);
        assert_eq!(state.reps, 20);

        // Should not exceed max
        upgrade_jump_squat(&mut state, 5, 20);
        assert_eq!(state.reps, 20);
    }
}
```

---

## Creating Microdose Definitions

A **MicrodoseDefinition** is a complete workout prescription. Most definitions have a single block, but you can create multi-block workouts (e.g., circuit training).

### Anatomy of a Definition

```rust
MicrodoseDefinition {
    id: "unique_id".into(),              // Snake_case identifier
    name: "Display Name".into(),          // Human-readable
    category: MicrodoseCategory::Vo2,     // Vo2, Gtg, or Mobility
    suggested_duration_seconds: 300,      // Expected time (used for scheduling)
    gtg_friendly: false,                  // Can this be done frequently?
    reference_url: None,                  // Optional external link
    blocks: vec![/* MicrodoseBlocks */], // Work intervals
}
```

### Single-Block Workout (Most Common)

**Example: 3-Minute Plank Hold**

```rust
microdoses.insert(
    "plank_hold_3m".into(),
    MicrodoseDefinition {
        id: "plank_hold_3m".into(),
        name: "3-Min Plank Hold".into(),
        category: MicrodoseCategory::Gtg,
        suggested_duration_seconds: 180,
        gtg_friendly: true,
        reference_url: None,
        blocks: vec![MicrodoseBlock {
            movement_id: "plank".into(),
            movement_style: MovementStyle::None,
            duration_hint_seconds: 180,
            metrics: vec![],  // No reps, just time-based
        }],
    },
);
```

### Multi-Block Workout (Circuit)

**Example: 4-Minute Tabata (20s work / 10s rest × 8 rounds)**

```rust
microdoses.insert(
    "tabata_burpee_4m".into(),
    MicrodoseDefinition {
        id: "tabata_burpee_4m".into(),
        name: "4-Min Tabata: Burpees".into(),
        category: MicrodoseCategory::Vo2,
        suggested_duration_seconds: 240,
        gtg_friendly: false,
        reference_url: None,
        blocks: vec![
            MicrodoseBlock {
                movement_id: "burpee".into(),
                movement_style: MovementStyle::Burpee(BurpeeStyle::FourCount),
                duration_hint_seconds: 20,
                metrics: vec![],  // Max reps in 20s
            },
            // Note: Rest periods aren't explicitly modeled yet
            // This is a limitation of the current v0.1 schema
        ],
    },
);
```

**Current Limitation**: The v0.1 schema doesn't explicitly model rest periods. For EMOM workouts, rest is implicit (work + rest = 60s). For Tabata, you'd need to document rest in the name or reference_url.

---

## Working with Metrics

Metrics define **how** a workout is measured. The system uses type-safe enums (not stringly-typed maps).

### MetricSpec::Reps

For counted repetitions (burpees, swings, squats, etc.):

```rust
MetricSpec::Reps {
    key: "reps".into(),          // Identifier (used in logs/CSV)
    default: 5,                   // Starting value
    min: 3,                       // Floor (for validation)
    max: 15,                      // Ceiling (for progression)
    step: 1,                      // Increment size (usually 1)
    progressable: true,           // Can this auto-increase?
}
```

**Common Keys:**
- `"reps"` - Total repetitions
- `"reps_per_side"` - Unilateral movements (lunges, single-leg RDL)
- `"rounds"` - Circuit workouts

**progressable Flag:**
- `true`: Intensity automatically increases over time (burpees, KB swings)
- `false`: Static prescription (mobility drills, warmups)

### MetricSpec::Band

For resistance band assistance (primarily pullups):

```rust
MetricSpec::Band {
    key: "band".into(),
    default: "red".into(),         // Band color (user-specific)
    progressable: false,           // Manual band changes only
}
```

**Why `progressable: false`?**

Band selection is **manual** because:
1. Users have different band sets (colors vary by manufacturer)
2. Band progression is non-linear (switching from red → orange is a big jump)
3. Users should decide when they're ready for less assistance

**User Workflow:**
1. User edits `~/.local/share/krep/wal/state.json` to change band color
2. Future workouts use the new band spec
3. Reps can still progress independently

### Adding New Metric Types

If you need a new metric (e.g., weight, time, distance), you must:

1. **Extend the enum** in `cardio_core/src/types.rs`:
   ```rust
   #[derive(Clone, Debug, Serialize, Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   pub enum MetricSpec {
       Reps { /* ... */ },
       Band { /* ... */ },
       Weight {  // NEW
           key: String,
           default: f32,
           unit: String,  // "kg" or "lbs"
           progressable: bool,
       },
   }
   ```

2. **Update validation** in `catalog.rs`:
   ```rust
   MetricSpec::Weight { default, .. } => {
       if *default <= 0.0 {
           errors.push(format!(
               "Microdose '{}': weight metric has invalid default",
               id
           ));
       }
   }
   ```

3. **Handle in progression logic** (if `progressable: true`).

**Breaking Change Warning**: Adding metric types changes the serialization schema. Existing user data (WAL, state.json) will need migration if they contain the old format.

---

## Movement Styles and Variations

Some movements have **style variations** that affect difficulty (burpees) or assistance level (banded pullups).

### MovementStyle Enum

Defined in `cardio_core/src/types.rs`:

```rust
pub enum MovementStyle {
    None,                    // No variation
    Burpee(BurpeeStyle),     // 4-count, 6-count, etc.
    Band(BandSpec),          // Resistance band assistance
}
```

### BurpeeStyle Progression

```rust
pub enum BurpeeStyle {
    FourCount,         // Standard: squat, plank, push-up, jump
    SixCount,          // Add mountain climbers (2 extra counts)
    SixCountTwoPump,   // 6-count + 2 push-ups
    Seal,              // Hardest: hands come off ground in push-up
}
```

**Progression Path** (see `progression.rs`):
```
FourCount @ 3 reps
    ↓ (increase reps to 10)
FourCount @ 10 reps
    ↓ (upgrade style, reset reps)
SixCount @ 6 reps
    ↓ (increase reps to 10)
SixCount @ 10 reps
    ↓ (upgrade style)
SixCountTwoPump @ 5 reps
    ↓ (increase reps to 10)
SixCountTwoPump @ 10 reps
    ↓ (upgrade style)
Seal @ 4 reps
    ↓ (increase reps to ceiling)
Seal @ 10 reps (max level)
```

**Why Reset Reps After Style Upgrade?**

Each style is significantly harder. Starting at a lower rep count after upgrading maintains appropriate RPE (Rate of Perceived Exertion).

### Adding New Style Variations

**Example: Push-Up Styles**

1. **Add enum in `types.rs`**:
   ```rust
   #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
   #[serde(rename_all = "snake_case")]
   pub enum PushUpStyle {
       Standard,
       Diamond,
       Archer,
       OneArm,
   }
   ```

2. **Extend MovementStyle**:
   ```rust
   pub enum MovementStyle {
       None,
       Burpee(BurpeeStyle),
       Band(BandSpec),
       PushUp(PushUpStyle),  // NEW
   }
   ```

3. **Use in catalog**:
   ```rust
   movements.insert(
       "push_up".into(),
       Movement {
           id: "push_up".into(),
           name: "Push-Up".into(),
           kind: MovementKind::PushUp,
           default_style: MovementStyle::PushUp(PushUpStyle::Standard),
           tags: vec!["gtg".into(), "push".into()],
           reference_url: None,
       },
   );
   ```

4. **Implement progression logic** (similar to `upgrade_burpee`):
   ```rust
   pub fn upgrade_pushup(state: &mut ProgressionState, rep_ceiling: i32) {
       if state.reps < rep_ceiling {
           state.reps += 1;
           state.level += 1;
           state.last_upgraded = Some(Utc::now());
           return;
       }

       // Upgrade style at ceiling
       let (new_style, new_reps) = match &state.style {
           MovementStyle::PushUp(PushUpStyle::Standard) => {
               (MovementStyle::PushUp(PushUpStyle::Diamond), 8)
           }
           MovementStyle::PushUp(PushUpStyle::Diamond) => {
               (MovementStyle::PushUp(PushUpStyle::Archer), 6)
           }
           MovementStyle::PushUp(PushUpStyle::Archer) => {
               (MovementStyle::PushUp(PushUpStyle::OneArm), 3)
           }
           MovementStyle::PushUp(PushUpStyle::OneArm) => {
               state.reps = rep_ceiling;
               state.level += 1;
               state.last_upgraded = Some(Utc::now());
               return;  // Max level
           }
           _ => (MovementStyle::PushUp(PushUpStyle::Standard), 10),
       };

       state.style = new_style;
       state.reps = new_reps;
       state.level += 1;
       state.last_upgraded = Some(Utc::now());
   }
   ```

---

## Advanced: Custom Progression Logic

For complex progression patterns (e.g., periodization, wave loading), you can implement custom upgrade functions.

### Example: Deadlift with Wave Loading

**Goal**: Alternate between rep ranges (5 reps, 8 reps, 3 reps) to prevent adaptation.

```rust
pub fn upgrade_deadlift(state: &mut ProgressionState, config: &Config) {
    // Wave pattern: 5 → 8 → 3 → 5 (repeat)
    let wave_pattern = [5, 8, 3];
    let current_index = (state.level as usize) % wave_pattern.len();
    let next_index = (current_index + 1) % wave_pattern.len();

    state.reps = wave_pattern[next_index];
    state.level += 1;
    state.last_upgraded = Some(Utc::now());

    tracing::debug!(
        "Deadlift progression: wave to {} reps (level {})",
        state.reps,
        state.level
    );
}
```

### Example: Time-Based Progression (Planks)

**Goal**: Increase duration instead of reps.

```rust
// Note: This requires a new metric type (Duration) - see "Adding New Metric Types"
pub fn upgrade_plank_hold(state: &mut ProgressionState, max_seconds: u32) {
    // Store duration in 'reps' field (hacky but works with current schema)
    let current_seconds = state.reps as u32;

    if current_seconds < max_seconds {
        state.reps = (current_seconds + 15).min(max_seconds) as i32;  // Add 15s
        state.level += 1;
        state.last_upgraded = Some(Utc::now());
        tracing::debug!("Plank progression: increased to {} seconds", state.reps);
    }
}
```

**Better Approach**: Add a `Duration` metric type to avoid overloading the `reps` field.

---

## Testing Your Additions

Comprehensive testing prevents bugs and validates your additions work correctly.

### Unit Tests (Core Library)

Add tests to `cardio_core/src/catalog.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_squat_exists() {
        let catalog = build_default_catalog();
        assert!(catalog.movements.contains_key("jump_squat"));
        assert!(catalog.microdoses.contains_key("emom_jump_squat_5m"));
    }

    #[test]
    fn test_jump_squat_metrics_valid() {
        let catalog = build_default_catalog();
        let def = catalog.microdoses.get("emom_jump_squat_5m").unwrap();

        assert_eq!(def.category, MicrodoseCategory::Vo2);
        assert!(!def.gtg_friendly);
        assert_eq!(def.blocks.len(), 1);

        let block = &def.blocks[0];
        assert_eq!(block.movement_id, "jump_squat");
        assert_eq!(block.metrics.len(), 1);

        match &block.metrics[0] {
            MetricSpec::Reps { default, min, max, progressable, .. } => {
                assert_eq!(*default, 5);
                assert_eq!(*min, 3);
                assert_eq!(*max, 20);
                assert!(*progressable);
            }
            _ => panic!("Expected Reps metric"),
        }
    }

    #[test]
    fn test_catalog_validates_after_additions() {
        let catalog = build_default_catalog();
        let errors = catalog.validate();
        assert!(
            errors.is_empty(),
            "Catalog validation failed: {:?}",
            errors
        );
    }
}
```

### Progression Tests

Add to `cardio_core/src/progression.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_squat_linear_progression() {
        let mut state = ProgressionState {
            reps: 5,
            style: MovementStyle::None,
            level: 0,
            last_upgraded: None,
        };

        for expected in 6..=20 {
            upgrade_jump_squat(&mut state, 5, 20);
            assert_eq!(state.reps, expected);
        }
    }

    #[test]
    fn test_increase_intensity_initializes_jump_squat() {
        let mut user_state = UserMicrodoseState::default();
        let config = Config::default();

        increase_intensity("emom_jump_squat_5m", &mut user_state, &config);

        assert!(user_state.progressions.contains_key("emom_jump_squat_5m"));
        let state = &user_state.progressions["emom_jump_squat_5m"];
        assert_eq!(state.reps, 6);  // Started at 5, increased to 6
        assert_eq!(state.level, 1);
    }
}
```

### Integration Tests (CLI)

Add to `cardio_cli/tests/integration_tests.rs`:

```rust
#[test]
fn test_jump_squat_prescription() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Prescribe jump squat workout
    Command::cargo_bin("cardio_cli")
        .unwrap()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("prescribe")
        .arg("--category")
        .arg("vo2")
        .assert()
        .success();

    // Should include jump_squat as one of the VO2 options
    // (exact prescription depends on round-robin state)
}
```

### Running Tests

```bash
# All tests
cargo test

# Core library only
cargo test -p cardio_core

# Specific test
cargo test test_jump_squat_progression

# With verbose output
cargo test -- --nocapture

# With debug logging
RUST_LOG=debug cargo test test_increase_intensity_initializes_jump_squat -- --nocapture
```

---

## Complete Examples

### Example 1: Kettlebell Clean (New Movement + Progression)

**Goal**: Add KB cleans with rep progression (3-15 reps).

```rust
// ======================================
// Step 1: Add MovementKind (types.rs)
// ======================================
pub enum MovementKind {
    KettlebellSwing,
    KettlebellClean,  // NEW
    Burpee,
    // ...
}

// ======================================
// Step 2: Add Movement (catalog.rs)
// ======================================
movements.insert(
    "kb_clean".into(),
    Movement {
        id: "kb_clean".into(),
        name: "Kettlebell Clean (2-hand)".into(),
        kind: MovementKind::KettlebellClean,
        default_style: MovementStyle::None,
        tags: vec!["vo2".into(), "hinge".into(), "power".into()],
        reference_url: Some("https://www.youtube.com/watch?v=example".into()),
    },
);

// ======================================
// Step 3: Add Microdose (catalog.rs)
// ======================================
microdoses.insert(
    "emom_kb_clean_5m".into(),
    MicrodoseDefinition {
        id: "emom_kb_clean_5m".into(),
        name: "5-Min EMOM: KB Cleans (2-hand)".into(),
        category: MicrodoseCategory::Vo2,
        suggested_duration_seconds: 300,
        gtg_friendly: false,
        reference_url: None,
        blocks: vec![MicrodoseBlock {
            movement_id: "kb_clean".into(),
            movement_style: MovementStyle::None,
            duration_hint_seconds: 60,
            metrics: vec![MetricSpec::Reps {
                key: "reps".into(),
                default: 3,
                min: 2,
                max: 15,
                step: 1,
                progressable: true,
            }],
        }],
    },
);

// ======================================
// Step 4: Add Progression (progression.rs)
// ======================================
pub fn upgrade_kb_clean(state: &mut ProgressionState, base_reps: i32, max_reps: i32) {
    if state.reps < max_reps {
        state.reps = (base_reps + state.level as i32 + 1).min(max_reps);
        state.level += 1;
        state.last_upgraded = Some(Utc::now());
        tracing::debug!("KB clean progression: increased to {} reps", state.reps);
    } else {
        tracing::debug!("KB clean progression: already at max ({} reps)", max_reps);
    }
}

// ======================================
// Step 5: Wire Up (progression.rs)
// ======================================
pub fn increase_intensity(def_id: &str, user_state: &mut UserMicrodoseState, config: &Config) {
    let state = user_state
        .progressions
        .entry(def_id.to_string())
        .or_insert_with(|| {
            let (reps, style) = match def_id {
                "emom_burpee_5m" => (3, MovementStyle::Burpee(BurpeeStyle::FourCount)),
                "emom_kb_swing_5m" => (5, MovementStyle::None),
                "emom_kb_clean_5m" => (3, MovementStyle::None),  // NEW
                "gtg_pullup_band" => (3, MovementStyle::None),
                _ => (3, MovementStyle::None),
            };
            // ...
        });

    match def_id {
        "emom_burpee_5m" => upgrade_burpee(state, config.progression.burpee_rep_ceiling),
        "emom_kb_swing_5m" => upgrade_kb_swing(state, 5, config.progression.kb_swing_max_reps),
        "emom_kb_clean_5m" => upgrade_kb_clean(state, 3, 15),  // NEW
        "gtg_pullup_band" => upgrade_pullup(state, 8),
        _ => tracing::warn!("Unknown definition ID for progression: {}", def_id),
    }
    // ...
}

// ======================================
// Step 6: Add Tests (progression.rs)
// ======================================
#[test]
fn test_kb_clean_progression() {
    let mut state = ProgressionState {
        reps: 3,
        style: MovementStyle::None,
        level: 0,
        last_upgraded: None,
    };

    upgrade_kb_clean(&mut state, 3, 15);
    assert_eq!(state.reps, 4);
    assert_eq!(state.level, 1);
}

#[test]
fn test_kb_clean_respects_max() {
    let mut state = ProgressionState {
        reps: 14,
        style: MovementStyle::None,
        level: 11,
        last_upgraded: None,
    };

    upgrade_kb_clean(&mut state, 3, 15);
    assert_eq!(state.reps, 15);

    upgrade_kb_clean(&mut state, 3, 15);
    assert_eq!(state.reps, 15);  // Should not exceed
}
```

**Run Tests:**
```bash
cargo test test_kb_clean
```

---

### Example 2: Yoga Flow (Mobility, No Progression)

**Goal**: Add a 3-minute sun salutation flow (static, no progression).

```rust
// ======================================
// Step 1: Add Movement (catalog.rs)
// ======================================
movements.insert(
    "sun_salutation".into(),
    Movement {
        id: "sun_salutation".into(),
        name: "Sun Salutation Flow (Surya Namaskar)".into(),
        kind: MovementKind::MobilityDrill,
        default_style: MovementStyle::None,
        tags: vec!["mobility".into(), "yoga".into(), "full_body".into(), "gtg_ok".into()],
        reference_url: Some("https://www.youtube.com/watch?v=example".into()),
    },
);

// ======================================
// Step 2: Add Microdose (catalog.rs)
// ======================================
microdoses.insert(
    "mobility_sun_salutation".into(),
    MicrodoseDefinition {
        id: "mobility_sun_salutation".into(),
        name: "Sun Salutation Flow (3 rounds)".into(),
        category: MicrodoseCategory::Mobility,
        suggested_duration_seconds: 180,
        gtg_friendly: true,
        reference_url: None,
        blocks: vec![MicrodoseBlock {
            movement_id: "sun_salutation".into(),
            movement_style: MovementStyle::None,
            duration_hint_seconds: 180,
            metrics: vec![MetricSpec::Reps {
                key: "rounds".into(),
                default: 3,
                min: 1,
                max: 5,
                step: 1,
                progressable: false,  // Static prescription
            }],
        }],
    },
);

// ======================================
// Step 3: Update Test Counts (catalog.rs)
// ======================================
#[test]
fn test_catalog_loads() {
    let catalog = build_default_catalog();
    assert_eq!(catalog.movements.len(), 7);  // Update count
    assert_eq!(catalog.microdoses.len(), 7); // Update count
}

// ======================================
// Step 4: Add Validation Test (catalog.rs)
// ======================================
#[test]
fn test_sun_salutation_exists() {
    let catalog = build_default_catalog();
    assert!(catalog.movements.contains_key("sun_salutation"));
    assert!(catalog.microdoses.contains_key("mobility_sun_salutation"));

    let def = &catalog.microdoses["mobility_sun_salutation"];
    assert_eq!(def.category, MicrodoseCategory::Mobility);
    assert!(def.gtg_friendly);
}
```

**No Progression Logic Needed**: Because `progressable: false`, this workout will always prescribe 3 rounds.

---

### Example 3: Dumbbell Complex (Multi-Block Workout)

**Goal**: Create a 4-minute complex with squats, presses, and rows.

```rust
// ======================================
// Step 1: Add Movements (catalog.rs)
// ======================================
movements.insert(
    "db_goblet_squat".into(),
    Movement {
        id: "db_goblet_squat".into(),
        name: "Dumbbell Goblet Squat".into(),
        kind: MovementKind::Squat,
        default_style: MovementStyle::None,
        tags: vec!["legs".into(), "squat".into()],
        reference_url: None,
    },
);

movements.insert(
    "db_overhead_press".into(),
    Movement {
        id: "db_overhead_press".into(),
        name: "Dumbbell Overhead Press".into(),
        kind: MovementKind::Press,
        default_style: MovementStyle::None,
        tags: vec!["shoulders".into(), "press".into()],
        reference_url: None,
    },
);

movements.insert(
    "db_row".into(),
    Movement {
        id: "db_row".into(),
        name: "Dumbbell Row".into(),
        kind: MovementKind::Row,
        default_style: MovementStyle::None,
        tags: vec!["back".into(), "pull".into()],
        reference_url: None,
    },
);

// ======================================
// Step 2: Add Complex Definition (catalog.rs)
// ======================================
microdoses.insert(
    "vo2_db_complex_4m".into(),
    MicrodoseDefinition {
        id: "vo2_db_complex_4m".into(),
        name: "4-Min Dumbbell Complex (Squat-Press-Row)".into(),
        category: MicrodoseCategory::Vo2,
        suggested_duration_seconds: 240,
        gtg_friendly: false,
        reference_url: None,
        blocks: vec![
            // Block 1: Goblet Squats
            MicrodoseBlock {
                movement_id: "db_goblet_squat".into(),
                movement_style: MovementStyle::None,
                duration_hint_seconds: 60,
                metrics: vec![MetricSpec::Reps {
                    key: "reps".into(),
                    default: 5,
                    min: 3,
                    max: 10,
                    step: 1,
                    progressable: true,
                }],
            },
            // Block 2: Overhead Press
            MicrodoseBlock {
                movement_id: "db_overhead_press".into(),
                movement_style: MovementStyle::None,
                duration_hint_seconds: 60,
                metrics: vec![MetricSpec::Reps {
                    key: "reps".into(),
                    default: 5,
                    min: 3,
                    max: 10,
                    step: 1,
                    progressable: true,
                }],
            },
            // Block 3: Rows
            MicrodoseBlock {
                movement_id: "db_row".into(),
                movement_style: MovementStyle::None,
                duration_hint_seconds: 60,
                metrics: vec![MetricSpec::Reps {
                    key: "reps_per_side".into(),
                    default: 5,
                    min: 3,
                    max: 10,
                    step: 1,
                    progressable: true,
                }],
            },
        ],
    },
);
```

**Note**: Multi-block progression is more complex because each block can progress independently. You may want to track per-block state or use a simpler "whole-workout" progression.

---

## Common Patterns and Conventions

### Naming Conventions

- **Movement IDs**: `snake_case` (e.g., `kb_swing_2h`, `hip_cars`)
- **Microdose IDs**: `category_movement_duration` (e.g., `emom_burpee_5m`, `mobility_hip_cars`)
- **Display Names**: Title case with full words (e.g., "5-Min EMOM: Burpees")

### Tag Conventions

**Functional Tags:**
- `vo2` - High-intensity cardio movements
- `gtg_ok` - Safe for frequent use (GTG/Mobility)
- `bodyweight` - No equipment needed
- `mobility` - Stretching/joint health

**Body Region Tags:**
- `full_body`, `upper_body`, `lower_body`
- `hip`, `shoulder`, `ankle`, `knee`, `spine`

**Movement Pattern Tags:**
- `hinge`, `squat`, `push`, `pull`
- `posterior_chain`, `anterior_chain`
- `plyometric`, `power`

### When to Add MovementKind vs. Reuse Existing

**Add New Kind If:**
- Progression logic is unique (e.g., BurpeeStyle upgrades)
- You want to query/filter by this type (e.g., "show all kettlebell movements")
- The movement is a primary category (e.g., Squats, Deadlifts)

**Reuse Existing If:**
- Movement is a variation of existing kind (e.g., single-leg RDL → MobilityDrill)
- Progression is generic (linear rep increase)
- You only need one or two definitions

### Category Selection Guidelines

| Category | When to Use | Duration | GTG-Friendly? |
|----------|-------------|----------|---------------|
| **Vo2** | Elevates heart rate, challenges cardio system | 3-5 min | No |
| **Gtg** | Builds strength via high-frequency, submaximal work | 30-60 sec | Yes |
| **Mobility** | Stretching, joint health, recovery | 2-4 min | Yes |

**Prescription Impact:**
- Vo2 workouts are prioritized if last VO2 > 4 hours ago
- GTG/Mobility preferred after recent lower-body strength training
- Round-robin within category prevents repetition

---

## Troubleshooting

### Validation Errors

**Error**: `"Microdose 'xyz' references non-existent movement 'abc'"`

**Fix**: Check that `movement_id` in your `MicrodoseBlock` matches an existing movement `id` in the catalog.

---

**Error**: `"Microdose 'xyz': default reps 15 > max 10"`

**Fix**: Ensure `default <= max` in your `MetricSpec::Reps`.

---

**Error**: `"Catalog has no VO2 microdoses"`

**Fix**: At least one microdose must have `category: MicrodoseCategory::Vo2`.

---

### Progression Not Working

**Symptom**: Workouts always prescribe the same intensity.

**Causes**:
1. `progressable: false` in metrics (change to `true`)
2. Missing case in `increase_intensity()` match statement
3. Definition ID mismatch (check logs for "Unknown definition ID")

**Debug**:
```bash
RUST_LOG=debug cargo run -- prescribe --dry-run
```

Look for:
- `"Burpee progression: increased reps to X"`
- `"Unknown definition ID for progression: xyz"`

---

### Tests Failing

**Error**: `"assertion failed: catalog.movements.len() == 5"`

**Fix**: Update test expectations after adding movements:
```rust
assert_eq!(catalog.movements.len(), 6);  // Was 5
```

---

**Error**: `"Catalog validation failed: [...]"`

**Fix**: Run validation manually to see all errors:
```rust
let catalog = build_default_catalog();
let errors = catalog.validate();
println!("Errors: {:?}", errors);
```

---

## FAQ

### Q: Can I add movements without editing `catalog.rs`?

**A**: Not in v0.1. Runtime catalog loading (from TOML/JSON) is planned for v0.3. For now, all movements must be in `build_default_catalog()`.

---

### Q: Can I have different users with different catalogs?

**A**: Not yet. v0.1 uses a global catalog. Multi-user support is planned for v0.3.

---

### Q: How do I add equipment requirements (e.g., "requires 24kg KB")?

**A**: The system doesn't track equipment metadata yet. For now, use:
1. Movement tags (e.g., `"kettlebell".into()`)
2. Movement name (e.g., "KB Swing (24kg)")
3. `user_state.equipment_available` (checked manually in prescription logic)

Future: Equipment spec in `Movement` struct.

---

### Q: Can I add custom metrics (e.g., heart rate zones)?

**A**: Yes, but it requires extending the `MetricSpec` enum (see [Adding New Metric Types](#adding-new-metric-types)). This is a breaking change to the schema.

---

### Q: How do I disable a movement without deleting it?

**A**: The system doesn't support disabled/archived movements yet. Workarounds:
1. Comment out the movement in `catalog.rs`
2. Remove all microdose definitions that use it
3. (Future) Add `enabled: bool` field to `Movement`

---

### Q: Can I have time-based progression (e.g., increase plank duration)?

**A**: Not cleanly in v0.1. You can hack it by storing seconds in the `reps` field, but adding a `Duration` metric type is better (requires schema change).

---

## Next Steps

- Read `CLAUDE.md` for full system architecture
- Explore `cardio_core/src/engine.rs` for prescription logic
- Study `cardio_cli/tests/integration_tests.rs` for testing patterns
- Join development: See `README.md` for contribution guidelines

---

**Happy coding! 💪🏃‍♂️**
