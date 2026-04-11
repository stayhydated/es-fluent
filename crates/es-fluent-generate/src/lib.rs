#![doc = include_str!("../README.md")]

use clap::ValueEnum;
use es_fluent_derive_core::EsFluentResult;
use es_fluent_derive_core::registry::FtlTypeInfo;
use indexmap::IndexMap;
use std::{fs, path::Path};

mod ast_build;
mod io;
mod merge;
mod model;

pub mod clean;
pub mod error;
pub mod formatting;
pub mod value;

use ast_build::build_target_resource;
pub(crate) use io::{read_existing_resource, write_updated_resource};
use merge::{MergeBehavior, smart_merge};

#[cfg(test)]
pub(crate) use ast_build::{create_group_comment_entry, create_message_entry};
#[cfg(test)]
pub(crate) use io::{print_diff, write_or_preview};
#[cfg(test)]
pub(crate) use merge::{
    collect_existing_keys, group_comment_name, insert_late_relocated, remove_empty_group_comments,
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
    let items_ref: Vec<&FtlTypeInfo> = items.iter().map(|i| i.as_ref()).collect();

    let mut namespaced: IndexMap<Option<String>, Vec<&FtlTypeInfo>> = IndexMap::new();
    for item in &items_ref {
        let namespace = item.resolved_namespace(manifest_dir);
        namespaced.entry(namespace).or_default().push(item);
    }

    let mut any_changed = false;

    for (namespace, ns_items) in namespaced {
        let (dir_path, file_path) = match namespace {
            Some(ns) => {
                let dir = i18n_path.join(crate_name);
                let file = dir.join(format!("{}.ftl", ns));
                (dir, file)
            },
            None => (
                i18n_path.to_path_buf(),
                i18n_path.join(format!("{}.ftl", crate_name)),
            ),
        };

        if !dry_run {
            fs::create_dir_all(&dir_path)?;
        }

        let existing_resource = read_existing_resource(&file_path)?;
        let final_resource = if matches!(mode, FluentParseMode::Aggressive) {
            build_target_resource(&ns_items)
        } else {
            smart_merge(existing_resource, &ns_items, MergeBehavior::Append)
        };

        if write_updated_resource(
            &file_path,
            &final_resource,
            dry_run,
            formatting::sort_ftl_resource,
        )? {
            any_changed = true;
        }
    }

    Ok(any_changed)
}

#[cfg(test)]
mod tests;
