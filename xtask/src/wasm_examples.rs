use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

pub const MANIFEST_RELATIVE_PATH: &str = "web/wasm-examples.json";
pub const SCHEMA_RELATIVE_PATH: &str = "web/wasm-examples.schema.json";
pub const SCHEMA_REFERENCE: &str = "./wasm-examples.schema.json";

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WasmExamplesManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub examples: Vec<WasmExample>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WasmExample {
    pub id: String,
    pub crate_dir: PathBuf,
    pub out_dir: PathBuf,
    pub out_name: String,
    #[serde(default)]
    pub wasm_pack_args: Vec<String>,
    #[serde(default)]
    pub copy: Vec<CopyPath>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CopyPath {
    pub source: PathBuf,
    pub destination: PathBuf,
}

impl WasmExample {
    pub fn wasm_path(&self) -> PathBuf {
        self.out_dir
            .join(format!("{}_bg.wasm", self.out_name.trim()))
    }

    pub fn module_path(&self) -> PathBuf {
        self.out_dir.join(format!("{}.js", self.out_name.trim()))
    }
}

pub fn manifest_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(MANIFEST_RELATIVE_PATH)
}

pub fn schema_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(SCHEMA_RELATIVE_PATH)
}

pub fn resolve_workspace_path(
    workspace_root: &Path,
    relative_path: &Path,
) -> anyhow::Result<PathBuf> {
    if relative_path.is_absolute() {
        bail!(
            "manifest paths must be workspace-relative: {}",
            relative_path.display()
        );
    }

    Ok(workspace_root.join(relative_path))
}

pub fn load_manifest(workspace_root: &Path) -> anyhow::Result<WasmExamplesManifest> {
    let manifest_path = manifest_path(workspace_root);
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: WasmExamplesManifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn write_schema(workspace_root: &Path) -> anyhow::Result<PathBuf> {
    let schema_path = schema_path(workspace_root);
    let schema = schema_for!(WasmExamplesManifest);
    let schema_json = serde_json::to_string_pretty(&schema)?;
    fs::write(&schema_path, format!("{schema_json}\n"))
        .with_context(|| format!("failed to write {}", schema_path.display()))?;
    Ok(schema_path)
}

fn validate_manifest(manifest: &WasmExamplesManifest) -> anyhow::Result<()> {
    if manifest.schema != SCHEMA_REFERENCE {
        bail!(
            "manifest $schema must be '{}', found '{}'",
            SCHEMA_REFERENCE,
            manifest.schema
        );
    }

    if manifest.examples.is_empty() {
        bail!("wasm example manifest must declare at least one example");
    }

    let mut ids = BTreeSet::new();
    let mut out_dirs = BTreeSet::new();
    for example in &manifest.examples {
        validate_example(example, &mut ids, &mut out_dirs)?;
    }

    Ok(())
}

fn validate_example(
    example: &WasmExample,
    ids: &mut BTreeSet<String>,
    out_dirs: &mut BTreeSet<PathBuf>,
) -> anyhow::Result<()> {
    if example.id.trim().is_empty() {
        bail!("wasm example id must not be empty");
    }
    if !ids.insert(example.id.clone()) {
        bail!("duplicate wasm example id '{}'", example.id);
    }

    validate_relative_path(&example.crate_dir, "crate_dir", &example.id)?;
    validate_relative_path(&example.out_dir, "out_dir", &example.id)?;

    if !out_dirs.insert(example.out_dir.clone()) {
        bail!(
            "duplicate out_dir '{}' in wasm example manifest",
            example.out_dir.display()
        );
    }

    let out_dir_name = example
        .out_dir
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or_default();
    if !out_dir_name.ends_with("-example") {
        bail!(
            "out_dir for '{}' must end with '-example' so turbo output globs stay scoped: {}",
            example.id,
            example.out_dir.display()
        );
    }

    if example.out_name.trim().is_empty() {
        bail!("out_name for '{}' must not be empty", example.id);
    }

    for copy_path in &example.copy {
        validate_relative_path(&copy_path.source, "copy.source", &example.id)?;
        validate_relative_path(&copy_path.destination, "copy.destination", &example.id)?;
    }

    Ok(())
}

fn validate_relative_path(path: &Path, field_name: &str, example_id: &str) -> anyhow::Result<()> {
    if path.as_os_str().is_empty() {
        bail!("{field_name} for '{example_id}' must not be empty");
    }
    if path.is_absolute() {
        bail!(
            "{field_name} for '{}' must be workspace-relative: {}",
            example_id,
            path.display()
        );
    }

    Ok(())
}
