use crate::core::{BuildOutcome, CrateInfo};
use crossterm::event::KeyEvent;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Debug)]
pub enum AppEvent {
    /// An input event from the user.
    Input(KeyEvent),
    /// A file change event.
    FileChange(CrateInfo),
    /// A tick event that occurs at a regular interval.
    Tick,
}

pub struct AppState {
    /// The crates that have been discovered.
    pub crates: Vec<CrateInfo>,
    /// The build statuses of the crates.
    pub build_statuses: Arc<Mutex<HashMap<String, BuildOutcome>>>,
    /// A debouncer for pending builds.
    pub pending_builds_debouncer: HashMap<String, (CrateInfo, Instant)>,
    /// The crates that are currently being built.
    pub active_builds: Arc<Mutex<HashSet<String>>>,
    /// Whether the application should quit.
    pub should_quit: bool,
}

impl AppState {
    /// Creates a new `AppState`.
    pub fn new(
        discovered_crates: Vec<CrateInfo>,
        initial_statuses: HashMap<String, BuildOutcome>,
    ) -> Self {
        Self {
            crates: discovered_crates,
            build_statuses: Arc::new(Mutex::new(initial_statuses)),
            pending_builds_debouncer: HashMap::new(),
            active_builds: Arc::new(Mutex::new(HashSet::new())),
            should_quit: false,
        }
    }
}
