use crate::FluentParseMode;
use crate::formatting;
use crate::merge::MergeBehavior;
use es_fluent_shared::EsFluentResult;
use es_fluent_shared::namespace::ResolvedNamespace;
use es_fluent_shared::registry::FtlTypeInfo;
use es_fluent_shared::resource::ResourceRoute;
use fluent_syntax::{ast, serializer};
use indexmap::IndexMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub(crate) struct PlannedOutput<'a> {
    pub(crate) route: ResourceRoute,
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
    ) -> EsFluentResult<ast::Resource<String>> {
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

    let mut namespaced: IndexMap<Option<ResolvedNamespace>, Vec<&'a FtlTypeInfo>> = IndexMap::new();
    for item in &items_ref {
        let namespace = item
            .try_resolved_namespace(manifest_dir)
            .map_err(|reason| {
                let namespace = item
                    .resolved_namespace(manifest_dir)
                    .unwrap_or_else(|| "<none>".to_string());
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Invalid namespace '{namespace}' for type '{}': {reason}",
                        item.type_name()
                    ),
                )
            })?;
        namespaced.entry(namespace).or_default().push(item);
    }

    Ok(namespaced
        .into_iter()
        .map(|(namespace, items)| {
            let route = ResourceRoute::from_namespace(namespace);
            let resource = route.resource_spec(crate_name, true);
            let file_path = i18n_path.join(resource.locale_relative_path.as_str());

            PlannedOutput {
                route,
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
    crate::model::validate_no_duplicate_ftl_keys(&output.items)?;

    if !dry_run && let Some(parent) = output.file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing_resource = crate::io::read_existing_resource(&output.file_path)?;
    let final_resource = operation.render_resource(existing_resource, &output.items)?;

    crate::io::write_updated_resource(
        &output.file_path,
        &final_resource,
        dry_run,
        operation.formatter(),
    )
}
