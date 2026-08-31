mod fixtures;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::io::{BufRead as _, Read as _};
use std::process::Stdio;

const SUBCOMMANDS: &[&str] = &[
    "generate",
    "watch",
    "clean",
    "fmt",
    "check",
    "status",
    "doctor",
    "sync",
    "add-locale",
    "tree",
];

#[path = "main_smoke/add_locale.rs"]
mod add_locale;
#[path = "main_smoke/check.rs"]
mod check;
#[path = "main_smoke/clean.rs"]
mod clean;
#[path = "main_smoke/doctor/mod.rs"]
mod doctor;
#[path = "main_smoke/fmt.rs"]
mod fmt;
#[path = "main_smoke/generate.rs"]
mod generate;
#[path = "main_smoke/help.rs"]
mod help;
#[path = "main_smoke/package_filters.rs"]
mod package_filters;
#[path = "main_smoke/status.rs"]
mod status;
#[path = "main_smoke/sync/mod.rs"]
mod sync;
#[path = "main_smoke/tree.rs"]
mod tree;
#[path = "main_smoke/watch.rs"]
mod watch;
