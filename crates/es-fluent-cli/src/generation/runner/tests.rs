use super::config::TempCrateConfig;
use super::exec::RunnerCrate;
use super::monolithic::MonolithicRunner;
use super::*;
use crate::core::{CrateInfo, WorkspaceInfo};
use crate::generation::cache::{MetadataCache, RunnerCache};
use crate::test_fixtures::FakeRunnerBehavior;
use es_fluent_runner::{
    FluentParseMode, I18nTomlPath, PackageName, RunnerMetadataStore, RunnerRequest,
};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use toml::Value;

mod cache;
mod configuration;
mod execution;
mod locking;
mod prepare;
mod staleness;
mod support;

use support::*;

#[test]
#[ignore = "subprocess helper for lock-owner exit coverage"]
fn monolithic_runner_lock_exit_without_drop_helper() {
    let Some(root) = std::env::var_os("ES_FLUENT_LOCK_EXIT_TEST_ROOT") else {
        return;
    };
    let _lock = acquire_monolithic_runner_lock(Path::new(&root)).expect("acquire child lock");
    std::process::exit(0);
}
