use super::check::{collect_check_run, count_issues};
use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo};
use crate::ftl::LocaleContext;
use clap::{Parser, Subcommand};
use es_fluent_runner::{FileTransaction, InventoryData, RunnerMetadataStore};
use path_slash::PathExt as _;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

const EXPORT_SCHEMA_VERSION: u32 = 1;
const EXPORT_STATE_FILE: &str = ".es-fluent-export.json";
const RUNTIME_IMPORT: &str = "@es-fluent/core";

/// Arguments for generated-language exports.
#[derive(Debug, Parser)]
pub struct ExportArgs {
    #[command(subcommand)]
    target: ExportTarget,
}

#[derive(Debug, Subcommand)]
enum ExportTarget {
    /// Export a framework-neutral TypeScript contract and package-owned FTL assets.
    Typescript(TypeScriptExportArgs),
}

/// Arguments for the TypeScript export target.
#[derive(Debug, Parser)]
struct TypeScriptExportArgs {
    #[command(flatten)]
    workspace: WorkspaceArgs,

    /// Output directory. Relative paths resolve from the selected Cargo workspace root.
    #[arg(long, value_name = "DIR")]
    out: PathBuf,

    /// Run the generated inventory runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    force_run: bool,

    /// Plan and report generated files without writing them.
    #[arg(long)]
    dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

#[derive(Clone, Debug)]
struct PackageExportInput {
    owner: String,
    fallback_locale: String,
    locales: Vec<String>,
    inventory: InventoryData,
    resources: Vec<ResourceInput>,
}

#[derive(Clone, Debug)]
struct ResourceInput {
    locale: String,
    domain: String,
    output_path: String,
    contents: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractView {
    schema_version: u32,
    revision: String,
    messages: Vec<MessageView>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageView {
    owner: String,
    domain: String,
    id: String,
    arguments: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rust_source: Option<RustSourceView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_line: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RustSourceView {
    type_kind: String,
    type_name: String,
    variant_name: String,
    module_path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestView {
    schema_version: u32,
    revision: String,
    packages: Vec<PackageView>,
    resources: Vec<ResourceView>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageView {
    owner: String,
    fallback_locale: String,
    locales: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceView {
    locale: String,
    owner: String,
    domain: String,
    path: String,
}

#[derive(Debug)]
struct ExportModel {
    contract: ContractView,
    manifest: ManifestView,
    resources: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportState {
    schema_version: u32,
    files: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportReport {
    target: &'static str,
    output_dir: String,
    package_count: usize,
    message_count: usize,
    resource_count: usize,
    warning_count: usize,
    changed_files: Vec<String>,
    removed_files: Vec<String>,
    dry_run: bool,
    applied: bool,
}

/// Run a generated-language export.
pub fn run_export(args: ExportArgs) -> Result<(), CliError> {
    match args.target {
        ExportTarget::Typescript(args) => run_typescript_export(args),
    }
}

fn run_typescript_export(args: TypeScriptExportArgs) -> Result<(), CliError> {
    if args.out.as_os_str().is_empty() {
        return Err(CliError::Other(
            "TypeScript export output directory must not be empty".to_string(),
        ));
    }

    let workspace = WorkspaceCrates::discover(args.workspace)?;
    workspace.require_non_empty_selection()?;
    workspace.require_all_crates_valid()?;

    let check = collect_check_run(&workspace, true, &[], args.force_run, true, false)?;
    let (error_count, warning_count) = count_issues(&check.issues);
    if error_count > 0 {
        let details = check
            .issues
            .iter()
            .filter(|issue| {
                !matches!(
                    issue,
                    crate::core::ValidationIssue::MissingVariable(_)
                        | crate::core::ValidationIssue::UntranslatedMessage(_)
                )
            })
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(CliError::Other(format!(
            "TypeScript export requires valid package-owned FTL resources ({error_count} error(s)): {details}"
        )));
    }

    let inputs = collect_package_inputs(&workspace)?;
    let model = build_export_model(inputs, Some(&workspace.workspace_info.root_dir))?;
    let output_dir = if args.out.is_absolute() {
        args.out
    } else {
        workspace.workspace_info.root_dir.join(args.out)
    };
    let (transaction, changed_files, removed_files) = plan_export(&output_dir, &model)?;
    let applied = if args.dry_run {
        false
    } else {
        transaction
            .commit()
            .map_err(|error| CliError::Other(error.to_string()))?
    };

    let report = ExportReport {
        target: "typescript",
        output_dir: crate::utils::paths::relative_slash_path(
            &output_dir,
            &workspace.workspace_info.root_dir,
        ),
        package_count: model.manifest.packages.len(),
        message_count: model.contract.messages.len(),
        resource_count: model.manifest.resources.len(),
        warning_count,
        changed_files,
        removed_files,
        dry_run: args.dry_run,
        applied,
    };

    if args.output.is_json() {
        args.output.print_json(&report)?;
    } else {
        println!(
            "Exported {} message(s) and {} resource(s) from {} package(s) to {}{}",
            report.message_count,
            report.resource_count,
            report.package_count,
            report.output_dir,
            if report.dry_run { " (dry run)" } else { "" }
        );
        if report.warning_count > 0 {
            println!("Validation warnings preserved: {}", report.warning_count);
        }
        println!("Changed files: {}", report.changed_files.len());
        println!("Removed stale files: {}", report.removed_files.len());
    }

    Ok(())
}

fn collect_package_inputs(
    workspace: &WorkspaceCrates,
) -> Result<Vec<PackageExportInput>, CliError> {
    let store = RunnerMetadataStore::temp_for_workspace(&workspace.workspace_info.root_dir);
    let mut inputs = Vec::new();

    for krate in &workspace.valid {
        let context = LocaleContext::from_crate(krate, true).map_err(CliError::from)?;
        let inventory = store
            .read_inventory(&krate.name)
            .map_err(|error| CliError::Other(error.to_string()))?;
        inputs.push(package_input_from_crate(krate, context, inventory)?);
    }

    inputs.sort_by(|left, right| left.owner.cmp(&right.owner));
    Ok(inputs)
}

fn package_input_from_crate(
    krate: &CrateInfo,
    context: LocaleContext,
    inventory: InventoryData,
) -> Result<PackageExportInput, CliError> {
    let mut resources = Vec::new();
    for locale in &context.locales {
        for file in context.discover_files(locale).map_err(CliError::from)? {
            let domain = domain_from_resource_path(&file.relative_path)?;
            let relative = file.relative_path.to_slash_lossy();
            let output_path = format!("locales/{}/{}/{}", locale, krate.name, relative);
            let contents = std::fs::read(&file.abs_path)?;
            resources.push(ResourceInput {
                locale: locale.clone(),
                domain,
                output_path,
                contents,
            });
        }
    }
    resources.sort_by(|left, right| left.output_path.cmp(&right.output_path));

    Ok(PackageExportInput {
        owner: krate.name.to_string(),
        fallback_locale: context.fallback,
        locales: context.locales,
        inventory,
        resources,
    })
}

fn domain_from_resource_path(path: &Path) -> Result<String, CliError> {
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return Err(CliError::Other(format!(
            "invalid package-owned FTL resource path: {}",
            path.display()
        )));
    };
    let first = first.to_str().ok_or_else(|| {
        CliError::Other(format!(
            "package-owned FTL resource path is not UTF-8: {}",
            path.display()
        ))
    })?;

    if components.next().is_some() {
        return Ok(first.to_string());
    }

    Path::new(first)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            CliError::Other(format!(
                "invalid package-owned FTL resource filename: {}",
                path.display()
            ))
        })
}

fn build_export_model(
    inputs: Vec<PackageExportInput>,
    source_root: Option<&Path>,
) -> Result<ExportModel, CliError> {
    let mut packages = Vec::new();
    let mut messages = Vec::new();
    let mut resources = Vec::new();
    let mut resource_views = Vec::new();
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"es-fluent-typescript-export-v1\0");

    for input in inputs {
        let mut locales = input.locales;
        locales.sort();
        locales.dedup();
        packages.push(PackageView {
            owner: input.owner.clone(),
            fallback_locale: input.fallback_locale,
            locales,
        });

        for key in input.inventory.expected_keys {
            let mut arguments = key
                .variables
                .into_iter()
                .map(|argument| argument.into_string())
                .collect::<Vec<_>>();
            arguments.sort();
            arguments.dedup();
            let (owner, domain, id) = key.key.into_parts();
            messages.push(MessageView {
                owner: owner.into_string(),
                domain: domain.into_string(),
                id: id.into_string(),
                arguments,
                rust_source: key.rust_source.map(|source| RustSourceView {
                    type_kind: source.type_kind.label().to_string(),
                    type_name: source.type_name,
                    variant_name: source.variant_name,
                    module_path: source.module_path,
                }),
                source_file: key
                    .source_file
                    .map(|file| export_source_path(file.as_str(), source_root)),
                source_line: key.source_line.map(|line| line.get()),
            });
        }

        for resource in input.resources {
            hasher.update(resource.output_path.as_bytes());
            hasher.update(b"\0");
            hasher.update(&resource.contents);
            hasher.update(b"\0");
            resource_views.push(ResourceView {
                locale: resource.locale,
                owner: input.owner.clone(),
                domain: resource.domain,
                path: resource.output_path.clone(),
            });
            resources.push((resource.output_path, resource.contents));
        }
    }

    packages.sort_by(|left, right| left.owner.cmp(&right.owner));
    messages.sort_by(|left, right| {
        (&left.owner, &left.domain, &left.id).cmp(&(&right.owner, &right.domain, &right.id))
    });
    resource_views.sort_by(|left, right| {
        (&left.owner, &left.locale, &left.path).cmp(&(&right.owner, &right.locale, &right.path))
    });
    resources.sort_by(|left, right| left.0.cmp(&right.0));

    hasher.update(
        &serde_json::to_vec(&packages).map_err(|error| CliError::Other(error.to_string()))?,
    );
    hasher.update(
        &serde_json::to_vec(&messages).map_err(|error| CliError::Other(error.to_string()))?,
    );
    let revision = hasher.finalize().to_hex().to_string();

    Ok(ExportModel {
        contract: ContractView {
            schema_version: EXPORT_SCHEMA_VERSION,
            revision: revision.clone(),
            messages,
        },
        manifest: ManifestView {
            schema_version: EXPORT_SCHEMA_VERSION,
            revision,
            packages,
            resources: resource_views,
        },
        resources,
    })
}

fn export_source_path(source: &str, source_root: Option<&Path>) -> String {
    let source = Path::new(source);
    source_root
        .and_then(|root| source.strip_prefix(root).ok())
        .unwrap_or(source)
        .to_slash_lossy()
        .into_owned()
}

fn plan_export(
    output_dir: &Path,
    model: &ExportModel,
) -> Result<(FileTransaction, Vec<String>, Vec<String>), CliError> {
    validate_export_output_ancestors(output_dir)?;
    if output_dir.exists() && !output_dir.is_dir() {
        return Err(CliError::Other(format!(
            "TypeScript export output is not a directory: {}",
            output_dir.display()
        )));
    }

    let contract_json = pretty_json(&model.contract)?;
    let manifest_json = pretty_json(&model.manifest)?;
    let messages_ts = render_messages_typescript(&model.contract)?;
    let manifest_ts = render_manifest_typescript(&model.manifest)?;
    let resources_ts = render_resources_typescript(&model.resources)?;
    let index_ts = concat!(
        "// Generated by `cargo es-fluent export typescript`.\n",
        "export { manifest } from \"./manifest.js\";\n",
        "export { messages } from \"./messages.js\";\n",
        "export { loadResource, resourceSources } from \"./resources.js\";\n",
    )
    .as_bytes()
    .to_vec();

    let mut files = BTreeMap::from([
        ("contract.json".to_string(), contract_json.into_bytes()),
        ("manifest.json".to_string(), manifest_json.into_bytes()),
        ("messages.ts".to_string(), messages_ts.into_bytes()),
        ("manifest.ts".to_string(), manifest_ts.into_bytes()),
        ("resources.ts".to_string(), resources_ts.into_bytes()),
        ("index.ts".to_string(), index_ts),
    ]);
    files.extend(model.resources.iter().cloned());

    let state_path = output_dir.join(EXPORT_STATE_FILE);
    validate_export_target_path(output_dir, &state_path)?;
    let previous = read_export_state(&state_path)?;
    let previously_owned = previous
        .as_ref()
        .map(|state| state.files.iter().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let current_paths = files.keys().cloned().collect::<BTreeSet<_>>();
    let mut stale_paths = previous
        .map(|state| state.files.into_iter().collect::<BTreeSet<_>>())
        .unwrap_or_default()
        .difference(&current_paths)
        .cloned()
        .collect::<Vec<_>>();
    stale_paths.sort();

    let state = ExportState {
        schema_version: EXPORT_SCHEMA_VERSION,
        files: current_paths.iter().cloned().collect(),
    };
    files.insert(
        EXPORT_STATE_FILE.to_string(),
        pretty_json(&state)?.into_bytes(),
    );

    let mut transaction = FileTransaction::default();
    let mut changed_files = Vec::new();
    let mut removed_files = Vec::new();
    for (relative, contents) in files {
        let path = checked_output_path(output_dir, &relative)?;
        validate_export_target_path(output_dir, &path)?;
        if relative != EXPORT_STATE_FILE
            && !previously_owned.contains(&relative)
            && std::fs::symlink_metadata(&path).is_ok()
        {
            return Err(CliError::Other(format!(
                "refusing to overwrite unowned TypeScript export path '{}'",
                path.display()
            )));
        }
        if transaction
            .plan_write(path, contents)
            .map_err(|error| CliError::Other(error.to_string()))?
        {
            changed_files.push(relative);
        }
    }
    for relative in stale_paths {
        let path = checked_output_path(output_dir, &relative)?;
        validate_export_target_path(output_dir, &path)?;
        if transaction
            .plan_remove(path)
            .map_err(|error| CliError::Other(error.to_string()))?
        {
            removed_files.push(relative);
        }
    }

    changed_files.sort();
    removed_files.sort();
    Ok((transaction, changed_files, removed_files))
}

fn validate_export_output_ancestors(output_dir: &Path) -> Result<(), CliError> {
    for path in output_dir.ancestors() {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CliError::Other(format!(
                    "TypeScript export output paths must not contain symlinks: {}",
                    path.display()
                )));
            },
            Ok(_) => {},
            Err(error)
                if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {},
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn validate_export_target_path(output_dir: &Path, target: &Path) -> Result<(), CliError> {
    let mut current = Some(target);
    while let Some(path) = current {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CliError::Other(format!(
                    "TypeScript export targets must not be symlinks: {}",
                    path.display()
                )));
            },
            Ok(metadata) if path != target && !metadata.is_dir() => {
                return Err(CliError::Other(format!(
                    "TypeScript export target parent is not a directory: {}",
                    path.display()
                )));
            },
            Ok(_) => {},
            Err(error)
                if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {},
            Err(error) => return Err(error.into()),
        }

        if path == output_dir {
            break;
        }
        current = path.parent();
    }
    Ok(())
}

fn pretty_json(value: &impl Serialize) -> Result<String, CliError> {
    let mut output =
        serde_json::to_string_pretty(value).map_err(|error| CliError::Other(error.to_string()))?;
    output.push('\n');
    Ok(output)
}

fn read_export_state(path: &Path) -> Result<Option<ExportState>, CliError> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let state: ExportState = serde_json::from_str(&source).map_err(|error| {
        CliError::Other(format!(
            "failed to parse TypeScript export state {}: {error}",
            path.display()
        ))
    })?;
    if state.schema_version != EXPORT_SCHEMA_VERSION {
        return Err(CliError::Other(format!(
            "unsupported TypeScript export state schema {} in {}",
            state.schema_version,
            path.display()
        )));
    }
    Ok(Some(state))
}

fn checked_output_path(output_dir: &Path, relative: &str) -> Result<PathBuf, CliError> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CliError::Other(format!(
            "invalid generated TypeScript export path '{relative}'"
        )));
    }
    Ok(output_dir.join(relative_path))
}

fn render_manifest_typescript(manifest: &ManifestView) -> Result<String, CliError> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|error| CliError::Other(error.to_string()))?;
    Ok(format!(
        "// Generated by `cargo es-fluent export typescript`.\n\
import type {{ EsFluentManifest }} from {runtime};\n\n\
export const manifest = {json} as const satisfies EsFluentManifest;\n",
        runtime = json_string(RUNTIME_IMPORT)?,
    ))
}

fn render_messages_typescript(contract: &ContractView) -> Result<String, CliError> {
    let mut grouped: BTreeMap<&str, BTreeMap<&str, Vec<&MessageView>>> = BTreeMap::new();
    for message in &contract.messages {
        grouped
            .entry(&message.owner)
            .or_default()
            .entry(&message.domain)
            .or_default()
            .push(message);
    }

    let mut output = String::new();
    writeln!(
        output,
        "// Generated by `cargo es-fluent export typescript`."
    )
    .map_err(fmt_error)?;
    writeln!(
        output,
        "import {{ defineMessage, type FluentVariable }} from {};",
        json_string(RUNTIME_IMPORT)?
    )
    .map_err(fmt_error)?;
    writeln!(output, "\nexport const messages = {{").map_err(fmt_error)?;

    for (owner, domains) in grouped {
        writeln!(output, "  {}: {{", json_string(owner)?).map_err(fmt_error)?;
        for (domain, messages) in domains {
            writeln!(output, "    {}: {{", json_string(domain)?).map_err(fmt_error)?;
            for message in messages {
                write!(output, "      {}: defineMessage", json_string(&message.id)?)
                    .map_err(fmt_error)?;
                if !message.arguments.is_empty() {
                    writeln!(output, "<{{").map_err(fmt_error)?;
                    for argument in &message.arguments {
                        writeln!(
                            output,
                            "        readonly {}: FluentVariable;",
                            json_string(argument)?
                        )
                        .map_err(fmt_error)?;
                    }
                    write!(output, "      }}>").map_err(fmt_error)?;
                }
                writeln!(output, "({{").map_err(fmt_error)?;
                writeln!(output, "        owner: {},", json_string(&message.owner)?)
                    .map_err(fmt_error)?;
                writeln!(output, "        domain: {},", json_string(&message.domain)?)
                    .map_err(fmt_error)?;
                writeln!(output, "        id: {},", json_string(&message.id)?)
                    .map_err(fmt_error)?;
                if let Some(source) = &message.rust_source {
                    writeln!(output, "        source: {{").map_err(fmt_error)?;
                    writeln!(
                        output,
                        "          typeKind: {},",
                        json_string(&source.type_kind)?
                    )
                    .map_err(fmt_error)?;
                    writeln!(
                        output,
                        "          typeName: {},",
                        json_string(&source.type_name)?
                    )
                    .map_err(fmt_error)?;
                    writeln!(
                        output,
                        "          variantName: {},",
                        json_string(&source.variant_name)?
                    )
                    .map_err(fmt_error)?;
                    writeln!(
                        output,
                        "          modulePath: {},",
                        json_string(&source.module_path)?
                    )
                    .map_err(fmt_error)?;
                    if let Some(file) = &message.source_file {
                        writeln!(output, "          file: {},", json_string(file)?)
                            .map_err(fmt_error)?;
                    }
                    if let Some(line) = message.source_line {
                        writeln!(output, "          line: {line},").map_err(fmt_error)?;
                    }
                    writeln!(output, "        }},").map_err(fmt_error)?;
                }
                writeln!(output, "      }}),").map_err(fmt_error)?;
            }
            writeln!(output, "    }},").map_err(fmt_error)?;
        }
        writeln!(output, "  }},").map_err(fmt_error)?;
    }
    writeln!(output, "}} as const;").map_err(fmt_error)?;
    Ok(output)
}

fn render_resources_typescript(resources: &[(String, Vec<u8>)]) -> Result<String, CliError> {
    let mut output = String::from(
        "// Generated by `cargo es-fluent export typescript`.\n\
import type { ResourceLoader } from \"@es-fluent/core\";\n\n\
export const resourceSources = {\n",
    );
    for (path, contents) in resources {
        let source = std::str::from_utf8(contents).map_err(|error| {
            CliError::Other(format!(
                "exported Fluent resource '{path}' is not UTF-8: {error}"
            ))
        })?;
        writeln!(
            output,
            "  {}: {},",
            json_string(path)?,
            json_string(source)?
        )
        .map_err(fmt_error)?;
    }
    output.push_str(
        "} as const;\n\n\
export const loadResource: ResourceLoader = (resource) => {\n\
  const sources: Readonly<Record<string, string>> = resourceSources;\n\
  const source = sources[resource.path];\n\
  if (source === undefined) {\n\
    throw new Error(`Missing exported Fluent resource: ${resource.path}`);\n\
  }\n\
  return source;\n\
};\n",
    );
    Ok(output)
}

fn json_string(value: &str) -> Result<String, CliError> {
    serde_json::to_string(value).map_err(|error| CliError::Other(error.to_string()))
}

fn fmt_error(error: std::fmt::Error) -> CliError {
    CliError::Other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_runner::{ExpectedKey, ExpectedKeyRustSource};
    use es_fluent_shared::fluent::{
        FluentArgumentName, FluentDomain, FluentEntryId, FluentMessageKey,
    };
    use es_fluent_shared::meta::TypeKind;
    use es_fluent_shared::source::{SourceFile, SourceLine};

    fn inventory() -> InventoryData {
        InventoryData {
            expected_keys: vec![ExpectedKey {
                key: FluentMessageKey::new(
                    FluentDomain::try_new("shop").expect("owner"),
                    FluentDomain::try_new("checkout").expect("domain"),
                    FluentEntryId::try_new("cart_message-Summary").expect("id"),
                ),
                variables: vec![
                    FluentArgumentName::try_new("state").expect("argument"),
                    FluentArgumentName::try_new("count").expect("argument"),
                ],
                resource: None,
                source_file: SourceFile::new("src/cart.rs"),
                source_line: Some(SourceLine::new(17)),
                rust_source: Some(ExpectedKeyRustSource {
                    type_kind: TypeKind::Enum,
                    type_name: "CartMessage".to_string(),
                    variant_name: "Summary".to_string(),
                    module_path: "shop::cart".to_string(),
                }),
            }],
        }
    }

    fn model_with_resource(contents: &[u8]) -> ExportModel {
        build_export_model(
            vec![PackageExportInput {
                owner: "shop".to_string(),
                fallback_locale: "en".to_string(),
                locales: vec!["fr".to_string(), "en".to_string()],
                inventory: inventory(),
                resources: vec![ResourceInput {
                    locale: "en".to_string(),
                    domain: "checkout".to_string(),
                    output_path: "locales/en/shop/checkout.ftl".to_string(),
                    contents: contents.to_vec(),
                }],
            }],
            None,
        )
        .expect("model")
    }

    #[test]
    fn resource_paths_preserve_package_local_domains() {
        assert_eq!(
            domain_from_resource_path(Path::new("checkout.ftl")).expect("domain"),
            "checkout"
        );
        assert_eq!(
            domain_from_resource_path(Path::new("checkout/forms.ftl")).expect("domain"),
            "checkout"
        );
    }

    #[test]
    fn generated_typescript_retains_scopes_arguments_and_rust_sources() {
        let model = model_with_resource(b"cart_message-Summary = { $count }\n");
        let output = render_messages_typescript(&model.contract).expect("render messages");

        assert!(output.contains("\"shop\": {"));
        assert!(output.contains("\"checkout\": {"));
        assert!(output.contains("\"cart_message-Summary\": defineMessage<{"));
        assert!(output.contains("readonly \"count\": FluentVariable;"));
        assert!(output.contains("typeName: \"CartMessage\""));
        assert!(output.contains("variantName: \"Summary\""));

        let resources = render_resources_typescript(&model.resources).expect("render resources");
        assert!(resources.contains("export const loadResource: ResourceLoader"));
        assert!(resources.contains("cart_message-Summary = { $count }\\n"));
    }

    #[test]
    fn export_transaction_writes_artifacts_and_removes_owned_stale_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let output_dir = temp.path().join("generated");
        std::fs::create_dir_all(output_dir.join("locales/en/shop")).expect("create output");
        std::fs::write(output_dir.join("stale.ftl"), "stale = Stale\n").expect("write stale");
        std::fs::write(
            output_dir.join(EXPORT_STATE_FILE),
            pretty_json(&ExportState {
                schema_version: EXPORT_SCHEMA_VERSION,
                files: vec!["stale.ftl".to_string()],
            })
            .expect("state"),
        )
        .expect("write state");

        let model = model_with_resource(b"cart_message-Summary = { $count }\n");
        let (transaction, changed, removed) =
            plan_export(&output_dir, &model).expect("plan export");

        assert!(changed.contains(&"messages.ts".to_string()));
        assert_eq!(removed, vec!["stale.ftl"]);
        assert!(transaction.commit().expect("commit"));
        assert!(!output_dir.join("stale.ftl").exists());
        assert!(output_dir.join("manifest.json").is_file());
        assert!(output_dir.join("locales/en/shop/checkout.ftl").is_file());
    }

    #[test]
    fn export_transaction_refuses_to_overwrite_unowned_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let output_dir = temp.path().join("generated");
        std::fs::create_dir_all(&output_dir).expect("create output");
        std::fs::write(
            output_dir.join("messages.ts"),
            "export const mine = true;\n",
        )
        .expect("write unowned file");

        let error = plan_export(&output_dir, &model_with_resource(b"hello = Hello\n"))
            .expect_err("unowned output collision should fail");

        assert!(error.to_string().contains("unowned TypeScript export path"));
        assert_eq!(
            std::fs::read_to_string(output_dir.join("messages.ts")).expect("read unowned file"),
            "export const mine = true;\n"
        );
        assert!(!output_dir.join(EXPORT_STATE_FILE).exists());
    }

    #[cfg(unix)]
    #[test]
    fn export_transaction_rejects_symlinked_targets() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let output_dir = temp.path().join("generated");
        std::fs::create_dir_all(&output_dir).expect("create output");
        let outside = temp.path().join("outside.ts");
        std::fs::write(&outside, "outside\n").expect("write outside");
        symlink(&outside, output_dir.join("messages.ts")).expect("create symlink");

        let error = plan_export(&output_dir, &model_with_resource(b"hello = Hello\n"))
            .expect_err("symlinked output should fail");

        assert!(error.to_string().contains("must not be symlinks"));
        assert_eq!(
            std::fs::read_to_string(outside).expect("read outside"),
            "outside\n"
        );
    }

    #[test]
    fn revision_changes_when_resource_contents_change() {
        let first = model_with_resource(b"hello = Hello\n");
        let second = model_with_resource(b"hello = Bonjour\n");

        assert_ne!(first.manifest.revision, second.manifest.revision);
    }

    #[test]
    fn checked_output_path_rejects_parent_components() {
        let error = checked_output_path(Path::new("generated"), "../outside.ftl")
            .expect_err("parent path should fail");
        assert!(error.to_string().contains("invalid generated"));
    }

    #[test]
    fn exported_source_paths_are_workspace_relative() {
        assert_eq!(
            export_source_path(
                "/workspace/examples/shop/src/cart.rs",
                Some(Path::new("/workspace"))
            ),
            "examples/shop/src/cart.rs"
        );
    }
}
