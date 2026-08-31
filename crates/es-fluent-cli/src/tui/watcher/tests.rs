use super::*;
use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
use crate::test_fixtures as tempfile;
use crate::test_fixtures::FakeRunnerBehavior;
use fs_err as fs;
use notify::{
    Event, RecursiveMode,
    event::{EventKind, ModifyKind},
};
use notify_debouncer_full::DebouncedEvent;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use toml::Value;

mod build_graph;
mod cargo_config;
#[path = "tests/events.rs"]
mod event_scenarios;
#[path = "tests/generation.rs"]
mod generation_scenarios;
mod loop_failure;
mod refresh;
mod support;

use support::*;
