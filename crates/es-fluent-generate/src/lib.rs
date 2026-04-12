#![doc = include_str!("../README.md")]

use clap::ValueEnum;
use es_fluent_derive_core::EsFluentResult;
use es_fluent_derive_core::registry::FtlTypeInfo;
use std::path::Path;

mod ast_build;
pub mod ftl;
mod io;
mod merge;
mod model;
mod pipeline;

pub mod clean;
pub mod error;
pub mod formatting;
pub mod value;

use pipeline::{OutputOperation, apply_output_operation, plan_outputs};

#[cfg(test)]
pub(crate) use ast_build::{create_group_comment_entry, create_message_entry};
#[cfg(test)]
pub(crate) use io::{read_existing_resource, write_updated_resource};
#[cfg(test)]
pub(crate) use io::{print_diff, write_or_preview};
#[cfg(test)]
pub(crate) use merge::{
    MergeBehavior, collect_existing_keys, group_comment_name, insert_late_relocated,
    remove_empty_group_comments, smart_merge,
};
#[cfg(test)]
pub(crate) use model::{OwnedTypeInfo, OwnedVariant};

/// The mode to use when parsing Fluent files.
#[derive(Clone, Debug, Default, strum::Display, PartialEq, ValueEnum)]
#[strum(serialize_all = "snake_case")]
pub enum FluentParseMode {
    /// Overwrite existing translations.
    Aggressive,
    /// Preserve existing translations.
    #[default]
    Conservative,
}

/// Generates a Fluent translation file from a list of `FtlTypeInfo` objects.
pub fn generate<P: AsRef<Path>, M: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    manifest_dir: M,
    items: &[I],
    mode: FluentParseMode,
    dry_run: bool,
) -> EsFluentResult<bool> {
    let i18n_path = i18n_path.as_ref();
    let manifest_dir = manifest_dir.as_ref();
    let mut any_changed = false;

    let operation = OutputOperation::Generate(mode);
    for output in plan_outputs(crate_name, i18n_path, manifest_dir, items) {
        if apply_output_operation(output, &operation, dry_run)? {
            any_changed = true;
        }
    }

    Ok(any_changed)
}

#[cfg(test)]
mod tests;
