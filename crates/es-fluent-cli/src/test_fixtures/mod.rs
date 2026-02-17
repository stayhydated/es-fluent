//! Shared test fixtures for es-fluent-cli tests.
//!
//! This module provides common file-based fixtures that can be reused
//! across different test modules.
#![allow(dead_code)]

pub const CARGO_TOML: &str = include_str!("../../tests/fixtures/base/Cargo.toml");
pub const I18N_TOML: &str = include_str!("../../tests/fixtures/base/i18n.toml");
pub const LIB_RS: &str = include_str!("../../tests/fixtures/base/lib.rs");
pub const HELLO_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello.ftl");
pub const HELLO_ES_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello_es.ftl");
pub const HELLO_FR_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello_fr.ftl");
pub const HELLO_WORLD_FTL: &str = include_str!("../../tests/fixtures/base/ftl/hello_world.ftl");
pub const RUNNER_SCRIPT: &str = include_str!("../../tests/fixtures/runner/runner.sh");
pub const RUNNER_OUTPUT_SCRIPT: &str = include_str!("../../tests/fixtures/runner/runner_output.sh");
pub const RUNNER_FAILING_SCRIPT: &str =
    include_str!("../../tests/fixtures/runner/runner_failing.sh");
pub const INVALID_FTL: &str = include_str!("../../tests/fixtures/runner/invalid.ftl");

// Check command specific fixtures
pub const INVENTORY_WITH_HELLO: &str =
    include_str!("../../tests/fixtures/check/inventory_with_hello.json");
pub const INVENTORY_WITH_MISSING_KEY: &str =
    include_str!("../../tests/fixtures/check/inventory_with_missing_key.json");

// Format command specific fixtures
pub const UI_UNSORTED_FTL: &str = include_str!("../../tests/fixtures/format/ui_unsorted.ftl");

// Utils specific fixtures
pub const WORKSPACE_CARGO_TOML: &str = include_str!("../../tests/fixtures/workspace/Cargo.toml");
