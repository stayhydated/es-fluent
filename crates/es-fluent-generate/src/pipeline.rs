use crate::FluentParseMode;
use crate::formatting;
use crate::merge::MergeBehavior;
use es_fluent_shared::EsFluentResult;
use es_fluent_shared::registry::FtlTypeInfo;
use fluent_syntax::{ast, serializer};
use indexmap::IndexMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub(crate) struct PlannedOutput<'a> {
    pub(crate) dir_path: PathBuf,
    pub(crate) file_path: PathBuf,
    pub(crate) items: Vec<&'a FtlTypeInfo>,
}

pub(crate) enum OutputOperation {
    Generate(FluentParseMode),
    Clean,
}

impl OutputOperation {
    fn render_resource(
        &self,
        existing_resource: ast::Resource<String>,
        items: &[&FtlTypeInfo],
    ) -> ast::Resource<String> {
        match self {
            Self::Generate(FluentParseMode::Aggressive) => {
                crate::ast_build::build_target_resource(items)
            },
            Self::Generate(FluentParseMode::Conservative) => {
                crate::merge::smart_merge(existing_resource, items, MergeBehavior::Append)
            },
            Self::Clean => {
                crate::merge::smart_merge(existing_resource, items, MergeBehavior::Clean)
            },
        }
    }

    fn formatter(&self) -> fn(&ast::Resource<String>) -> String {
        match self {
            Self::Generate(_) => formatting::sort_ftl_resource,
            Self::Clean => serializer::serialize,
        }
    }
}

pub(crate) fn plan_outputs<'a, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: &Path,
    manifest_dir: &Path,
    items: &'a [I],
) -> EsFluentResult<Vec<PlannedOutput<'a>>> {
    let items_ref: Vec<&'a FtlTypeInfo> = items.iter().map(|item| item.as_ref()).collect();

    let mut namespaced: IndexMap<Option<String>, Vec<&'a FtlTypeInfo>> = IndexMap::new();
    for item in &items_ref {
        let namespace = item.resolved_namespace(manifest_dir);
        if let Some(ref namespace) = namespace {
            es_fluent_shared::namespace::validate_namespace_path(namespace).map_err(|reason| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Invalid namespace '{namespace}' for type '{}': {reason}",
                        item.type_name
                    ),
                )
            })?;
        }
        namespaced.entry(namespace).or_default().push(item);
    }

    Ok(namespaced
        .into_iter()
        .map(|(namespace, items)| {
            let (dir_path, file_path) = match namespace {
                Some(namespace) => {
                    let dir_path = i18n_path.join(crate_name);
                    let file_path = dir_path.join(format!("{}.ftl", namespace));
                    (dir_path, file_path)
                },
                None => (
                    i18n_path.to_path_buf(),
                    i18n_path.join(format!("{}.ftl", crate_name)),
                ),
            };

            PlannedOutput {
                dir_path,
                file_path,
                items,
            }
        })
        .collect())
}

pub(crate) fn apply_output_operation(
    output: PlannedOutput<'_>,
    operation: &OutputOperation,
    dry_run: bool,
) -> EsFluentResult<bool> {
    if !dry_run {
        fs::create_dir_all(&output.dir_path)?;
    }

    let existing_resource = crate::io::read_existing_resource(&output.file_path)?;
    let final_resource = operation.render_resource(existing_resource, &output.items);

    crate::io::write_updated_resource(
        &output.file_path,
        &final_resource,
        dry_run,
        operation.formatter(),
    )
}
