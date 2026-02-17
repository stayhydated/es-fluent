//! Test fixtures for es-fluent-generate tests.
//!
//! This module provides file-based fixtures using `include_str!` macro
//! to include the actual file contents at compile time.
#![allow(dead_code)]

/// Fixture: Empty group for variant insertion test
pub const EMPTY_GROUP: &str = include_str!("ftl/empty_group.ftl");

/// Fixture: Two groups with key in wrong group
pub const RELOCATE_GROUPS: &str = include_str!("ftl/relocate_groups.ftl");

/// Fixture: Empty groups for similar name relocation
pub const EMPTY_GROUPS_SIMILAR: &str = include_str!("ftl/empty_groups_similar.ftl");

/// Fixture: Orphan group for clean mode test
pub const ORPHAN_GROUPS: &str = include_str!("ftl/orphan_groups.ftl");

/// Fixture: GroupB before GroupA with manual key inside GroupA
pub const GROUP_ORDERING: &str = include_str!("ftl/group_ordering.ftl");

/// Fixture: Complex FTL structure with selectors
pub const COMPLEX_STRUCTURE: &str = include_str!("ftl/complex_structure.ftl");

/// Fixture: Empty group A that should be removed, and group B that should be kept
pub const EMPTY_GROUP_A: &str = include_str!("ftl/empty_group_a.ftl");

/// Fixture: Country variants that should be preserved
pub const COUNTRY_VARIANTS: &str = include_str!("ftl/country_variants.ftl");

/// Fixture: Single group with one key
pub const SINGLE_GROUP_KEY: &str = include_str!("ftl/single_group_key.ftl");

/// Fixture: i18n.toml configuration
pub const I18N_TOML: &str = include_str!("config/i18n.toml");
