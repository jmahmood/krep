use libadwaita as adw;
use adw::prelude::*;
use adw::Application;
use cardio_core::{
    get_default_catalog, increase_intensity, load_external_strength, load_recent_sessions, BandSpec,
    Config, ExternalStrengthSignal, JsonlSink, MicrodoseCategory, MicrodoseSession, MovementStyle,
    PrescribedMicrodose, ProgressionState, SessionKind, SessionSink, UserContext,
    UserMicrodoseState,
};
use chrono::{DateTime, Utc};
use dirs;
use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use gtk4 as gtk;
use glib::{self, ControlFlow};
use ksni;
use serde_json;
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
use tracing::Level;
use uuid::Uuid;

struct LoadedData {
    config: Config,
    data_dir: PathBuf,
    wal_path: PathBuf,
    csv_path: PathBuf,
    state_path: PathBuf,
    strength_path: PathBuf,
    // Use reference to cached catalog for performance
    catalog: &'static cardio_core::Catalog,
    user_state: UserMicrodoseState,
    recent_sessions: Vec<SessionKind>,
    warnings: Vec<String>,
    strength_signal: Option<ExternalStrengthSignal>,
}

struct UiState {
    loaded: LoadedData,
    skipped_ids: HashSet<String>,
    prescription: PrescribedMicrodose,
    ctx_now: DateTime<Utc>,
}

#[derive(Debug)]
enum TrayEvent {
    Activate,
    WatcherOnline,
    WatcherOffline,
}

struct KrepTray {
    tx: Sender<TrayEvent>,
}

impl ksni::Tray for KrepTray {
    fn icon_name(&self) -> String {
        // Fallback to a well-known icon so the indicator is always visible
        "applications-system".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![solid_icon(24, 0xFF2ECC71)]
    }

    fn id(&self) -> String {
        "krep-tray".into()
    }

    fn title(&self) -> String {
        "Krep".into()
    }

    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: self.icon_name(),
            icon_pixmap: self.icon_pixmap(),
            title: "Krep".into(),
            description: "Microdose cardio assistant".into(),
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![ksni::MenuItem::Standard(
            ksni::menu::StandardItem {
                label: "Microdose Now".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayEvent::Activate);
                }),
                ..Default::default()
            },
        )]
    }

    fn watcher_online(&self) {
        let _ = self.tx.send(TrayEvent::WatcherOnline);
    }

    fn watcher_offine(&self) -> bool {
        let _ = self.tx.send(TrayEvent::WatcherOffline);
        true
    }
}

fn solid_icon(size: i32, argb: u32) -> ksni::Icon {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..(size * size) {
        data.extend_from_slice(&argb.to_be_bytes());
    }
    ksni::Icon {
        width: size,
        height: size,
        data,
    }
}

fn init_logging() {
    let log_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("krep")
        .join("krep_tray.log");

    let _ = std::fs::create_dir_all(
        log_path
            .parent()
            .unwrap_or_else(|| Path::new(".")),
    );

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(move || -> Box<dyn Write + Send> {
            match std::fs::File::options()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                Ok(f) => Box::new(f),
                Err(_) => Box::new(std::io::stdout()),
            }
        })
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}

fn main() {
    init_logging();

    let app = Application::builder()
        .application_id("com.krep.tray")
        .build();

    app.connect_activate(|app| {
        setup_tray(app);
    });

    app.run();
}

fn setup_tray(app: &Application) {
    // Prevent the app from quitting when the window is closed.
    Box::leak(Box::new(app.hold()));

    let (tx, rx) = channel::<TrayEvent>();

    let _svc = ksni::TrayService::new(KrepTray { tx }).spawn();

    let app_weak = app.downgrade();
    let mut watcher_seen = false;
    let mut warned_no_watcher = false;
    eprintln!("[krep-tray] Tray service started. Waiting for watcher/events...");
    let app_clone_for_loop = app_weak.clone();
    glib::timeout_add_local(Duration::from_millis(300), move || {
        while let Ok(event) = rx.try_recv() {
            match event {
                TrayEvent::Activate => {
                    if let Some(app) = app_clone_for_loop.upgrade() {
                        show_prescription_window(&app);
                    }
                }
                TrayEvent::WatcherOnline => {
                    watcher_seen = true;
                    warned_no_watcher = false;
                    eprintln!("[krep-tray] StatusNotifier watcher detected.");
                }
                TrayEvent::WatcherOffline => {
                    watcher_seen = false;
                    eprintln!("[krep-tray] StatusNotifier watcher went offline.");
                }
            }
        }

        if !watcher_seen && !warned_no_watcher {
            warned_no_watcher = true;
            eprintln!(
                "[krep-tray] No StatusNotifier watcher detected. Ensure the AppIndicator/SNI extension is enabled in GNOME. Falling back to showing the popup window once."
            );
            if let Some(app) = app_clone_for_loop.upgrade() {
                show_prescription_window(&app);
            }
        }
        ControlFlow::Continue
    });

    // Immediately show the popup at startup as a fallback
    if let Some(app) = app_weak.upgrade() {
        eprintln!("[krep-tray] Showing popup once as startup fallback.");
        show_prescription_window(&app);
    }
}

fn load_data() -> cardio_core::Result<LoadedData> {
    let config = Config::load()?;
    let data_dir = config.data.data_dir.clone();
    let wal_dir = data_dir.join("wal");
    std::fs::create_dir_all(&wal_dir)?;

    let state_path = wal_dir.join("state.json");
    let wal_path = wal_dir.join("microdose_sessions.wal");
    let csv_path = data_dir.join("sessions.csv");
    let strength_path = data_dir.join("strength").join("signal.json");

    let mut warnings = Vec::new();

    // Use cached catalog for performance (eliminates 50+ allocations)
    let catalog = get_default_catalog();

    // Load state - error handling is built into load() function
    let user_state = match UserMicrodoseState::load(&state_path) {
        Ok(state) => state,
        Err(e) => {
            warnings.push(format!("State load failed: {}; using defaults.", e));
            UserMicrodoseState::default()
        }
    };

    // Load strength signal - error handling is built into load_external_strength()
    let strength_signal = match load_external_strength(&strength_path) {
        Ok(sig) => sig,
        Err(e) => {
            warnings.push(format!("Strength signal load failed: {}; ignoring.", e));
            None
        }
    };

    // Load history
    let recent_sessions = load_recent_sessions(&wal_path, &csv_path, 7)?;

    Ok(LoadedData {
        config,
        data_dir,
        wal_path,
        csv_path,
        state_path,
        strength_path,
        catalog,
        user_state,
        recent_sessions,
        warnings,
        strength_signal,
    })
}

fn compute_prescription(
    loaded: &LoadedData,
    ctx_now: DateTime<Utc>,
    recent: &[SessionKind],
) -> cardio_core::Result<PrescribedMicrodose> {
    let mut ctx = UserContext {
        now: ctx_now,
        user_state: loaded.user_state.clone(),
        recent_sessions: recent.to_vec(),
        external_strength: loaded.strength_signal.clone(),
        equipment_available: loaded.config.equipment.available.clone(),
    };

    cardio_core::prescribe_next(&loaded.catalog, &mut ctx, None)
}

fn show_prescription_window(app: &Application) {
    let loaded = match load_data() {
        Ok(data) => data,
        Err(err) => {
            tracing::error!("Failed to load data: {}", err);
            return;
        }
    };

    let ctx_now = Utc::now();
    let prescription = match compute_prescription(&loaded, ctx_now, &loaded.recent_sessions) {
        Ok(p) => p,
        Err(err) => {
            tracing::error!("Failed to prescribe: {}", err);
            return;
        }
    };

    let ui_state = Rc::new(RefCell::new(UiState {
        loaded,
        skipped_ids: HashSet::new(),
        prescription,
        ctx_now,
    }));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(320)
        .default_height(420)
        .title("Krep")
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    window.set_content(Some(&content));

    build_prescription_ui(&content, ui_state.clone(), &window);

    window.present();
}

fn build_prescription_ui(
    container: &gtk::Box,
    state: Rc<RefCell<UiState>>,
    window: &adw::ApplicationWindow,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let state_ref = state.borrow();
    let prescription = &state_ref.prescription;

    if !state_ref.loaded.warnings.is_empty() {
        let warning = gtk::Label::new(Some("⚠ Some data could not be loaded. Defaults used."));
        warning.set_wrap(true);
        warning.add_css_class("warning");
        container.append(&warning);
    }

    let title = gtk::Label::new(Some(&prescription.definition.name));
    title.set_margin_bottom(6);
    title.add_css_class("title-2");
    container.append(&title);

    let duration = gtk::Label::new(Some(&format!(
        "Duration: ~{} sec",
        prescription.definition.suggested_duration_seconds
    )));
    duration.set_margin_bottom(6);
    container.append(&duration);

    if let Some(reps) = prescription.reps {
        let reps_label = gtk::Label::new(Some(&format!("Reps: {}", reps)));
        reps_label.set_margin_bottom(4);
        container.append(&reps_label);
    }

    if let Some(style) = &prescription.style {
        let style_text = format!("Style: {}", format_style(style));
        let style_label = gtk::Label::new(Some(&style_text));
        style_label.set_margin_bottom(4);
        container.append(&style_label);
    }

    if let Some(url) = &prescription.definition.reference_url {
        let link = gtk::LinkButton::with_label(url, "Learn");
        container.append(&link);
    }

    let button_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    container.append(&button_row);

    let do_it = gtk::Button::with_label("Do It");
    let skip = gtk::Button::with_label("Skip");
    let harder = gtk::Button::with_label("Harder Next Time");
    let cancel = gtk::Button::with_label("Cancel");

    button_row.append(&do_it);
    button_row.append(&skip);
    button_row.append(&harder);
    button_row.append(&cancel);

    {
        let state = state.clone();
        let window = window.clone();
        do_it.connect_clicked(move |_| {
            let mut state = state.borrow_mut();
            if let Err(err) = log_session(&mut state) {
                tracing::error!("Failed to log session: {}", err);
            }
            window.close();
        });
    }

    {
        let state = state.clone();
        let container = container.clone();
        let window = window.clone();
        skip.connect_clicked(move |_| {
            if let Err(err) = handle_skip(&state) {
                tracing::error!("Failed to skip: {}", err);
                window.close();
                return;
            }
            build_prescription_ui(&container, state.clone(), &window);
        });
    }

    {
        let state = state.clone();
        let window = window.clone();
        harder.connect_clicked(move |_| {
            let mut state = state.borrow_mut();
            if let Err(err) = mark_harder(&mut state) {
                tracing::error!("Failed to apply harder: {}", err);
            }
            window.close();
        });
    }

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| {
            window.close();
        });
    }
}

fn log_session(state: &mut UiState) -> cardio_core::Result<()> {
    let prescription = state.prescription.clone();

    let session = MicrodoseSession {
        id: Uuid::new_v4(),
        definition_id: prescription.definition.id.clone(),
        performed_at: state.ctx_now,
        started_at: Some(state.ctx_now),
        completed_at: Some(state.ctx_now),
        actual_duration_seconds: Some(prescription.definition.suggested_duration_seconds),
        metrics_realized: vec![],
        perceived_rpe: None,
        avg_hr: None,
        max_hr: None,
    };

    let mut sink = JsonlSink::new(&state.loaded.wal_path);
    sink.append(&session)?;

    // Track mobility rotation and persist state
    if prescription.definition.category == MicrodoseCategory::Mobility {
        state.loaded.user_state.last_mobility_def_id =
            Some(prescription.definition.id.clone());
    }
    state
        .loaded
        .user_state
        .progressions
        .entry(prescription.definition.id.clone())
        .or_insert_with(|| ProgressionState {
            reps: prescription.reps.unwrap_or(0),
            style: prescription.style.unwrap_or(MovementStyle::None),
            level: 0,
            last_upgraded: None,
        });

    state.loaded.user_state.save(&state.loaded.state_path)?;
    Ok(())
}

fn handle_skip(state: &Rc<RefCell<UiState>>) -> cardio_core::Result<()> {
    let mut state = state.borrow_mut();
    let def_id = state.prescription.definition.id.clone();
    state.skipped_ids.insert(def_id.clone());

    // Insert a skipped entry into recent sessions
    let skipped = SessionKind::ShownButSkipped {
        definition_id: def_id,
        shown_at: state.ctx_now,
    };

    let mut recent = state.loaded.recent_sessions.clone();
    recent.insert(0, skipped);

    let next = compute_prescription(&state.loaded, state.ctx_now, &recent)?;
    state.prescription = next;
    state.loaded.recent_sessions = recent;
    Ok(())
}

fn mark_harder(state: &mut UiState) -> cardio_core::Result<()> {
    increase_intensity(
        &state.prescription.definition.id,
        &mut state.loaded.user_state,
        &state.loaded.config,
    );
    state.loaded.user_state.save(&state.loaded.state_path)?;
    Ok(())
}

fn format_style(style: &MovementStyle) -> String {
    match style {
        MovementStyle::None => "Default".to_string(),
        MovementStyle::Burpee(b) => format!("Burpee: {:?}", b),
        MovementStyle::Band(BandSpec::NamedColour(c)) => format!("Band: {}", c),
        other => format!("{:?}", other),
    }
}
