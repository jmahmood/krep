use cardio_core::*;
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "krep")]
#[command(about = "Cardio microdose prescription system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Override data directory
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Prescribe and perform the next microdose (default)
    Now {
        /// Target category (vo2, gtg, mobility)
        #[arg(long)]
        category: Option<String>,

        /// Dry run - show prescription without logging
        #[arg(long)]
        dry_run: bool,

        /// Auto-complete (for testing) - automatically mark as done
        #[arg(long)]
        auto_complete: bool,
    },

    /// Roll up WAL sessions to CSV
    Rollup {
        /// Clean up processed WAL files after rollup
        #[arg(long)]
        cleanup: bool,
    },
}

fn main() -> Result<()> {
    // Initialize logging
    cardio_core::logging::init();

    let cli = Cli::parse();

    // Determine data directory
    let config = Config::load()?;
    let data_dir = cli.data_dir.unwrap_or_else(|| config.data.data_dir.clone());

    match cli.command {
        Some(Commands::Now {
            category,
            dry_run,
            auto_complete,
        }) => cmd_now(data_dir, category, dry_run, auto_complete, &config),
        Some(Commands::Rollup { cleanup }) => cmd_rollup(data_dir, cleanup),
        None => {
            // Default to "now" command
            cmd_now(data_dir, None, false, false, &config)
        }
    }
}

fn cmd_now(
    data_dir: PathBuf,
    category: Option<String>,
    dry_run: bool,
    auto_complete: bool,
    config: &Config,
) -> Result<()> {
    // Ensure directories exist
    let wal_dir = data_dir.join("wal");
    std::fs::create_dir_all(&wal_dir)?;

    // Set up paths
    let state_path = wal_dir.join("state.json");
    let wal_path = wal_dir.join("microdose_sessions.wal");
    let csv_path = data_dir.join("sessions.csv");
    let strength_path = data_dir.join("strength").join("signal.json");

    // Load catalog and state
    let catalog = build_default_catalog();
    let errors = catalog.validate();
    if !errors.is_empty() {
        eprintln!("Catalog validation errors:");
        for error in errors {
            eprintln!("  - {}", error);
        }
        return Err(Error::CatalogValidation("Invalid catalog".into()));
    }

    let mut user_state = UserMicrodoseState::load(&state_path)?;
    let strength_signal = load_external_strength(&strength_path)?;

    // Load recent sessions (7 days)
    let recent_sessions = load_recent_sessions(&wal_path, &csv_path, 7)?;

    // Parse category if provided
    let target_category = category.as_ref().and_then(|c| match c.to_lowercase().as_str() {
        "vo2" => Some(MicrodoseCategory::Vo2),
        "gtg" => Some(MicrodoseCategory::Gtg),
        "mobility" => Some(MicrodoseCategory::Mobility),
        _ => {
            eprintln!("Unknown category: {}. Using default selection.", c);
            None
        }
    });

    // Build user context
    let ctx = UserContext {
        now: chrono::Utc::now(),
        user_state: user_state.clone(),
        recent_sessions,
        external_strength: strength_signal,
        equipment_available: config.equipment.available.clone(),
    };

    // Prescribe next microdose
    let prescription = prescribe_next(&catalog, &ctx, target_category)?;

    // Display prescription
    display_prescription(&prescription);

    if dry_run {
        println!("\n[Dry run - not logging session]");
        return Ok(());
    }

    // Wait for user action (unless auto-complete)
    let action = if auto_complete {
        UserAction::Done
    } else {
        prompt_user_action()?
    };

    match action {
        UserAction::Done => {
            // Create session
            let session = MicrodoseSession {
                id: uuid::Uuid::new_v4(),
                definition_id: prescription.definition.id.clone(),
                performed_at: ctx.now,
                started_at: Some(ctx.now),
                completed_at: Some(ctx.now),
                actual_duration_seconds: Some(prescription.definition.suggested_duration_seconds),
                metrics_realized: vec![], // Could capture actual reps here
                perceived_rpe: None,
                avg_hr: None,
                max_hr: None,
            };

            // Append to WAL
            let mut sink = JsonlSink::new(&wal_path);
            sink.append(&session)?;

            // Update mobility round-robin if applicable
            if prescription.definition.category == MicrodoseCategory::Mobility {
                user_state.last_mobility_def_id = Some(prescription.definition.id.clone());
                user_state.save(&state_path)?;
            }

            println!("\n✓ Session logged!");
        }

        UserAction::Skip => {
            println!("\nSession skipped.");
        }

        UserAction::Harder => {
            // Increase intensity
            increase_intensity(&prescription.definition.id, &mut user_state, config);
            user_state.save(&state_path)?;

            println!("\n✓ Intensity increased for next time!");
            println!("  Level: {}", user_state.progressions[&prescription.definition.id].level);
            println!("  Reps: {}", user_state.progressions[&prescription.definition.id].reps);
        }
    }

    Ok(())
}

fn cmd_rollup(data_dir: PathBuf, cleanup: bool) -> Result<()> {
    let wal_dir = data_dir.join("wal");
    let wal_path = wal_dir.join("microdose_sessions.wal");
    let csv_path = data_dir.join("sessions.csv");

    if !wal_path.exists() {
        println!("No WAL file found - nothing to roll up.");
        return Ok(());
    }

    let count = cardio_core::csv_rollup::wal_to_csv_and_archive(&wal_path, &csv_path)?;

    println!("✓ Rolled up {} sessions to CSV", count);
    println!("  CSV: {}", csv_path.display());

    if cleanup {
        let cleaned = cardio_core::csv_rollup::cleanup_processed_wals(&wal_dir)?;
        if cleaned > 0 {
            println!("✓ Cleaned up {} processed WAL files", cleaned);
        }
    }

    Ok(())
}

fn display_prescription(prescription: &PrescribedMicrodose) {
    println!("\n╭─────────────────────────────────────────╮");
    println!("│  {:?} MICRODOSE", prescription.definition.category);
    println!("╰─────────────────────────────────────────╯");
    println!();
    println!("  {}", prescription.definition.name);
    println!("  Duration: ~{} seconds ({} min)",
        prescription.definition.suggested_duration_seconds,
        prescription.definition.suggested_duration_seconds / 60);
    println!();

    for block in &prescription.definition.blocks {
        let movement = prescription.definition.name.clone(); // Simplified

        if let Some(reps) = prescription.reps {
            println!("  → {} reps", reps);
        }

        if let Some(ref style) = prescription.style {
            match style {
                MovementStyle::Burpee(s) => {
                    println!("  → Style: {:?}", s);
                }
                MovementStyle::Band(BandSpec::NamedColour(c)) => {
                    println!("  → Band: {}", c);
                }
                _ => {}
            }
        }
    }

    if let Some(ref url) = prescription.definition.reference_url {
        println!();
        println!("  ℹ Reference: {}", url);
    }

    println!();
}

enum UserAction {
    Done,
    Skip,
    Harder,
}

fn prompt_user_action() -> Result<UserAction> {
    println!("─────────────────────────────────────────");
    println!("Press Enter when done");
    println!("  's' + Enter to skip");
    println!("  'h' + Enter to mark 'harder next time'");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let action = match input.trim().to_lowercase().as_str() {
        "s" => UserAction::Skip,
        "h" => UserAction::Harder,
        _ => UserAction::Done,
    };

    Ok(action)
}
