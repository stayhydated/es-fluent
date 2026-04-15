use std::{fs, path::Path};

use anyhow::{bail, Context};

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
    let mut missing_markers = Vec::new();

    for example in &manifest.examples {
        verify_example(
            workspace_root,
            example,
            &mut missing_outputs,
            &mut missing_markers,
        )?;
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

    if !missing_markers.is_empty() {
        bail!(
            "Missing required wasm markers:\n{}",
            missing_markers
                .iter()
                .map(|message| format!("- {message}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "Verified declared outputs and required markers for {} wasm example(s)",
        manifest.examples.len()
    );
    Ok(())
}

fn verify_example(
    workspace_root: &Path,
    example: &WasmExample,
    missing_outputs: &mut Vec<String>,
    missing_markers: &mut Vec<String>,
) -> anyhow::Result<()> {
    let wasm_path = resolve_workspace_path(workspace_root, &example.wasm_path())?;
    let module_path = resolve_workspace_path(workspace_root, &example.module_path())?;

    if !module_path.is_file() {
        missing_outputs.push(module_path.display().to_string());
    }

    if !wasm_path.is_file() {
        missing_outputs.push(wasm_path.display().to_string());
        return Ok(());
    }

    let wasm_bytes = fs::read(&wasm_path)
        .with_context(|| format!("failed to read wasm output at {}", wasm_path.display()))?;

    for marker in &example.required_markers {
        if !contains_bytes(&wasm_bytes, marker.as_bytes()) {
            missing_markers.push(format!(
                "{} missing '{}' in {}",
                example.id,
                marker,
                wasm_path.display()
            ));
        }
    }

    Ok(())
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
