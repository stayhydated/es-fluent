#![doc = include_str!("../README.md")]

pub use es_fluent_runner::FileTransaction;
use es_fluent_shared::EsFluentResult;
pub use es_fluent_shared::FluentParseMode;
use es_fluent_shared::registry::FtlTypeInfo;
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

use pipeline::OutputOperation;

#[cfg(test)]
pub(crate) use ast_build::{create_group_comment_entry, create_message_entry};
#[cfg(test)]
pub(crate) use io::{print_diff, write_or_preview};
#[cfg(test)]
pub(crate) use io::{read_existing_resource, write_updated_resource};
#[cfg(test)]
pub(crate) use merge::{
    MergeBehavior, collect_existing_keys, group_comment_name, insert_late_relocated,
    remove_empty_group_comments, smart_merge,
};
#[cfg(test)]
pub(crate) use model::{OwnedTypeInfo, OwnedVariant};

/// Generates a Fluent translation file from a list of `FtlTypeInfo` objects.
pub fn generate<P: AsRef<Path>, M: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    manifest_dir: M,
    items: &[I],
    mode: FluentParseMode,
    dry_run: bool,
) -> EsFluentResult<bool> {
    let transaction = plan_generate(crate_name, i18n_path, manifest_dir, items, mode)?;
    io::apply_transaction(&transaction, dry_run)
}

/// Plans all fallback FTL mutations without writing them.
pub fn plan_generate<P: AsRef<Path>, M: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    manifest_dir: M,
    items: &[I],
    mode: FluentParseMode,
) -> EsFluentResult<FileTransaction> {
    let i18n_path = i18n_path.as_ref();
    let manifest_dir = manifest_dir.as_ref();
    let mut transaction = FileTransaction::default();

    let operation = OutputOperation::Generate(mode);
    for output in pipeline::plan_outputs(crate_name, i18n_path, manifest_dir, items)? {
        pipeline::plan_output_operation(output, &operation, &mut transaction)?;
    }

    Ok(transaction)
}

#[cfg(test)]
mod tests;

/// Applies or previews a previously planned FTL transaction.
pub fn apply_transaction(transaction: &FileTransaction, dry_run: bool) -> EsFluentResult<bool> {
    io::apply_transaction(transaction, dry_run)
}
