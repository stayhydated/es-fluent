use std::path::Path;

use anyhow::bail;

use crate::{
    util::workspace_root,
    wasm_examples::{load_manifest, manifest_path, resolve_workspace_path, WasmExample},
};

pub fn run() -> anyhow::Result<()> {
    run_from_workspace_root(&workspace_root()?)
}

fn run_from_workspace_root(workspace_root: &Path) -> anyhow::Result<()> {
    let manifest = load_manifest(workspace_root)?;
    println!(
        "Verifying {} declared wasm example(s) from {}",
        manifest.examples.len(),
        manifest_path(workspace_root).display()
    );

    let mut missing_outputs = Vec::new();
    for example in &manifest.examples {
        verify_example(workspace_root, example, &mut missing_outputs)?;
    }

    if !missing_outputs.is_empty() {
        bail!(
            "Missing declared wasm example outputs:\n{}",
            missing_outputs
                .iter()
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "Verified declared outputs for {} wasm example(s)",
        manifest.examples.len()
    );
    Ok(())
}

fn verify_example(
    workspace_root: &Path,
    example: &WasmExample,
    missing_outputs: &mut Vec<String>,
) -> anyhow::Result<()> {
    let wasm_path = resolve_workspace_path(workspace_root, &example.wasm_path())?;
    let module_path = resolve_workspace_path(workspace_root, &example.module_path())?;

    if !module_path.is_file() {
        missing_outputs.push(module_path.display().to_string());
    }

    if !wasm_path.is_file() {
        missing_outputs.push(wasm_path.display().to_string());
    }

    Ok(())
}
