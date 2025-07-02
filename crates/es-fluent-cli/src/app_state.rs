use crate::core::{BuildOutcome, CrateInfo};
use crossterm::event::KeyEvent;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Debug)]
pub enum AppEvent {
    Input(KeyEvent),
    FileChange(CrateInfo),
    Tick,
}

pub struct AppState {
    pub crates: Vec<CrateInfo>,
    pub build_statuses: Arc<Mutex<HashMap<String, BuildOutcome>>>,
    pub pending_builds_debouncer: HashMap<String, (CrateInfo, Instant)>,
    pub active_builds: Arc<Mutex<HashSet<String>>>,
    pub should_quit: bool,
}

impl AppState {
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
